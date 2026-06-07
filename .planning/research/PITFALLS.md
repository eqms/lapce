# Pitfalls Research

**Domain:** Rust desktop code editor — hardening milestone (async runtime intro, dep migration, integrity verification, error propagation)
**Researched:** 2026-06-07
**Confidence:** HIGH (all pitfalls grounded in documented crate behavior, CVE records, and lapce-specific codebase evidence from CONCERNS.md)

---

## Critical Pitfalls

### Pitfall 1: Nested Runtime Panic — `block_on` Inside an Existing Tokio Context

**What goes wrong:**
Calling `tokio::runtime::Handle::block_on` or `Runtime::block_on` from inside an already-executing async task causes an immediate panic: `Cannot start a runtime from within a runtime`. This is the single most common mistake when introducing tokio into a codebase that previously used `std::thread::spawn` + `reqwest::blocking`.

**Why it happens:**
The typical mistake is converting a `std::thread::spawn` closure to `async move` and calling some legacy helper that internally calls `block_on`. In lapce specifically: `get_url` in `lapce-proxy/src/lib.rs` currently calls `reqwest::blocking::get`. If a developer wraps that function body in `async move` without removing the `blocking::get` call, the first HTTP request will panic because `reqwest::blocking` internally calls `block_on` and a tokio context is already active via `rt.enter()`.

**How to avoid:**
- Never call `reqwest::blocking::*` from inside a `tokio::spawn` task or any `async fn`. The `blocking` feature must be removed from `reqwest` at the same time the async runtime is introduced.
- The correct migration pattern for `get_url`: replace `reqwest::blocking::get(url)` with `reqwest::Client::new().get(url).send().await`.
- Audit every call site of `get_url` — if any caller is not already `async`, wrap with `tokio::spawn` rather than adding `block_on`.
- If a synchronous interface is genuinely needed (e.g., called from a non-async context on a background thread), use `Handle::current().block_on(...)` only from a thread that was NOT spawned by tokio (i.e., a `std::thread::spawn` thread). Never from `tokio::spawn`.

**Warning signs:**
- Thread panics with message containing `Cannot start a runtime from within a runtime` in CI or at startup.
- Any remaining `reqwest::blocking` imports after the tokio migration is declared complete — run `grep -r "reqwest::blocking" lapce-proxy/src/` and `grep -r "reqwest::blocking" lapce-app/src/` as a gate check.
- `#[tokio::main]` on `main()` is added (incompatible with Floem owning the main thread).

**Phase to address:**
Performance cluster — the async runtime introduction is the largest single change. Must be done atomically: remove `blocking` feature from reqwest and add tokio in the same PR. The `rt.enter()` guard pattern (not `#[tokio::main]`) is the only safe approach given Floem's main-thread ownership.

---

### Pitfall 2: Blocking Calls Starving the Tokio Runtime

**What goes wrong:**
Calling CPU-bound or blocking I/O operations directly inside a `tokio::spawn` task starves the async executor. With `worker_threads(2)` (the recommended low-footprint config for lapce), a single blocking operation monopolizes 50% of the runtime, and two simultaneous blocking operations make the runtime completely unresponsive for other tasks. Symptoms: HTTP downloads appear to hang; LSP responses time out; UI events stall.

**Why it happens:**
Network downloads themselves are non-blocking (tokio handles the I/O asynchronously), but related operations are not:
- `zstd::Decoder::new` + `tar::Archive::unpack` — archive decompression is synchronous CPU work that can take hundreds of milliseconds for a 10 MB plugin archive.
- `std::fs::write` for large files is blocking.
- SHA256 hashing of a large downloaded buffer is synchronous.
Placing any of these inside a `tokio::spawn` without wrapping in `spawn_blocking` will cause task starvation.

**How to avoid:**
- Use `tokio::task::spawn_blocking(|| { ... })` for all CPU-bound work: hash computation, archive decompression/extraction, synchronous file writes.
- The download itself (`reqwest::Response::bytes().await`) is fine in a regular `tokio::spawn` task.
- Pattern for `download_volt` and `download_release`: (1) async download bytes, (2) `spawn_blocking` for SHA256 verify + extract.
- Bound `worker_threads` to 2 for the lapce use case — network I/O only. This forces discipline: any code that accidentally blocks will be obvious in testing.

**Warning signs:**
- UI feels frozen during plugin downloads (main Floem thread is fine; the symptom is that a second download does not start while one is in progress).
- `tokio-console` shows tasks in `POLL` state for >100 ms continuously.
- `grep -rn "zstd\|tar::Archive\|std::fs::write\|sha2" lapce-proxy/src/plugin/mod.rs` inside functions that are not wrapped in `spawn_blocking`.

**Phase to address:**
Performance cluster — implement `spawn_blocking` wrappers for extraction and hashing at the same time as the async runtime introduction. Do not split these across separate PRs.

---

### Pitfall 3: Tokio Runtime Dropped While Tasks Are In Flight

**What goes wrong:**
When the `tokio::Runtime` value is dropped, it synchronously blocks the dropping thread until all spawned tasks complete. If `rt` is dropped at the end of a `main()` function that previously called `floem::launch()` (which blocks until window close), this is safe. But if `rt` is stored in a struct that can be dropped before window close (e.g., dropped from an event handler), all in-flight downloads are cancelled without notification, causing silent partial writes or corrupt state.

**Why it happens:**
The `let rt = tokio::runtime::Builder::new_multi_thread()...build()` pattern creates a value that must outlive all tasks spawned on it. If stored in a `Config` or `AppState` struct that gets replaced on config reload, the old runtime is dropped and all downloads are silently killed. The `_guard = rt.enter()` pattern makes this subtler — the guard must also be held.

**How to avoid:**
- Create the runtime in `main()` before `floem::launch()` and do not move it anywhere:
  ```rust
  let rt = tokio::runtime::Builder::new_multi_thread()...build()?;
  let _guard = rt.enter();
  floem::launch(app_view); // blocks until window close
  // rt dropped here, after all windows are closed
  ```
- Never store `Runtime` in application state structs. If tasks need to be spawned from deep in the app, use `tokio::runtime::Handle::current()` (which does not own the runtime) and call `handle.spawn(...)`.
- If cancellation on shutdown is needed (e.g., cancel in-flight downloads gracefully), use a `tokio::sync::CancellationToken` broadcast to running tasks before dropping `rt`.

**Warning signs:**
- Downloads silently abort mid-flight when the user opens Settings or switches workspaces.
- `rt` appears as a field in any struct other than a `main`-function-level binding.
- Partial plugin archives left in the plugin cache directory after apparent installation failures.

**Phase to address:**
Performance cluster — architectural decision made upfront when the runtime is introduced. Cannot be retrofitted cheaply.

---

### Pitfall 4: reqwest 0.11 → 0.12 Body/Stream API Regression

**What goes wrong:**
In reqwest 0.11, `response.bytes()` and `response.text()` return `Result<Bytes, Error>`. In reqwest 0.12, the API surface is the same for basic usage, but the underlying `hyper` 1 `Body` type changes mean that any code that accesses `response.body_mut()`, uses `reqwest::Body::wrap_stream`, or calls hyper-specific APIs directly will fail to compile. Less obviously, the `reqwest::Response::error_for_status_ref()` method and the `hyper::body::to_bytes` helper have different signatures.

**Why it happens:**
Lapce's `get_url` function is simple (GET + `response.bytes()`) so the compile error is unlikely. The risk is in any callers that chain `.json::<T>()` on responses that were previously handled differently, or that explicitly handle redirect behavior. reqwest 0.12 also changes default redirect policy for CONNECT tunnels (relevant to proxy support).

**How to avoid:**
- Before upgrading, audit all `reqwest::` call sites: `grep -rn "reqwest::" lapce-app/src/ lapce-proxy/src/`.
- The specific sites in lapce: `lib.rs:get_url`, `update.rs:download_release`, `proxy/remote.rs`, `plugin/mod.rs:download_volt`. All use simple `GET + bytes()` — migration is mechanical.
- Remove the `blocking` feature flag entirely from `reqwest` in `Cargo.toml`; leaving it would compile but create the nested runtime pitfall (Pitfall 1).
- Run `cargo test` after the version bump before touching any logic — compile errors surface immediately.
- Verify the `socks` feature flag is preserved: lapce uses `Proxy::all(proxy_url)` for SOCKS5 proxy support.

**Warning signs:**
- Build failure mentioning `hyper::body` or `http_body` types after the version bump.
- Proxy downloads (SSH remote sessions) silently fail but direct downloads work — indicates the `socks` feature was accidentally dropped.
- `reqwest::blocking` still in scope (means migration is incomplete and Pitfall 1 is active).

**Phase to address:**
Deps cluster — do the version bump as its own commit, verify all four download call sites compile and pass integration smoke tests, then move on to async migration.

---

### Pitfall 5: interprocess 2.x — Name API Panic on Unnamed Sockets

**What goes wrong:**
In interprocess 1.x, passing a raw `&str` or `PathBuf` to `LocalSocketListener::bind` works for both filesystem-path sockets and abstract Linux sockets. In interprocess 2.x, the name must be explicitly typed via `ToFsName` (filesystem path) or `ToNsName` (Linux abstract namespace). Using `to_fs_name::<GenericFilePath>()` on a path that looks like an abstract namespace name (starts with `@` or `\0`) silently creates a filesystem socket instead, breaking single-instance detection on Linux.

**Why it happens:**
The lapce socket path is constructed in `app.rs` as something like `~/.local/share/lapce/lapce.sock`. This is unambiguously a filesystem path, so `GenericFilePath` is correct. The pitfall arises if a developer sees the `GenericNamespaced` variant in the API and assumes it is "better" for Linux, or if the socket path string is generated differently on different platforms and one platform's path happens to be invalid as a filesystem name.

**How to avoid:**
- Use `GenericFilePath` (filesystem-path sockets) unconditionally for the lapce socket. The path is user-data-directory-based on all three platforms — no abstract namespace needed.
- The migration is two lines in `app.rs` per call site:
  ```rust
  use interprocess::local_socket::prelude::*;
  use interprocess::local_socket::GenericFilePath;
  let name = socket_path.to_fs_name::<GenericFilePath>()?;
  LocalSocketListener::bind(name)?
  ```
- Write a regression test: launch two instances with the same socket path; the second must detect the first and exit (or send a command to the first). This test catches silent failures in single-instance detection.
- Do NOT enable `features = ["tokio"]` for interprocess — the sync API is sufficient and the tokio async local socket types panic outside a tokio context if called from the wrong thread.

**Warning signs:**
- Opening a second lapce window opens a second editor instead of focusing the first (single-instance detection silently fails).
- `strace` on Linux shows `connect(AF_UNIX, ...)` using the wrong socket type (stream vs. abstract).
- Compilation error mentioning `ToFsName` or `ToNsName` not in scope — means the `prelude::*` import is missing.

**Phase to address:**
Deps cluster — the API migration is mechanical; test it with an integration test for single-instance behavior before closing the upgrade.

---

### Pitfall 6: SHA256 TOCTOU — Verifying a Different File Than the One Used

**What goes wrong:**
A time-of-check/time-of-use (TOCTOU) race exists when: (1) the file is written to disk, (2) SHA256 is verified against the on-disk file, (3) the file is then read again for extraction/execution. Between steps 2 and 3, an attacker with write access to the temp directory can swap the verified file. This is the canonical file-system TOCTOU pattern.

**Why it happens:**
The natural implementation flow is: download → save to `tmp_path` → read `tmp_path` back → hash → compare → extract from `tmp_path`. The verify-then-extract gap is the window. In lapce's update flow (`update.rs:55-85`), the archive is downloaded to a temp path and then extracted — adding hash verification as a post-download step against the temp file creates this gap.

**How to avoid:**
- Compute the SHA256 hash against the in-memory `Bytes` buffer, not the on-disk file. The hash is computed before any disk write:
  ```rust
  let bytes = response.bytes().await?;
  let hash = Sha256::digest(&bytes);
  if format!("{:x}", hash) != expected_hash { return Err(...) }
  // Only write to disk after hash is confirmed:
  std::fs::write(&tmp_path, &bytes)?;
  // Then extract from tmp_path (or directly from bytes in memory)
  ```
- This pattern eliminates the TOCTOU window entirely because the bytes that are hashed are the same bytes that are extracted.
- Never hash a path on disk that was written by the same process in an earlier step — always hash the buffer.

**Warning signs:**
- Code that does `sha2::digest(std::fs::read(&path)?)` after a `std::fs::write(&path, &bytes)` — the re-read is the TOCTOU gap.
- Temp file written to a world-writable directory (`/tmp`) with a predictable name — doubly dangerous with TOCTOU.

**Phase to address:**
Security cluster — the hash-then-write pattern must be established in the initial integrity verification implementation. Cannot be patched later without understanding the entire data flow.

---

### Pitfall 7: Trusting a Hash Fetched Over the Same Channel as the Binary

**What goes wrong:**
Fetching the expected SHA256 hash from the same server and over the same HTTPS connection as the binary provides no meaningful integrity guarantee. If the CDN or GitHub release is compromised, the attacker controls both the binary and its "expected" hash. This gives a false sense of security — the verification always passes because both values come from the same compromised source.

**Why it happens:**
The most convenient implementation is: `GET /release/lapce-x86_64.tar.gz` → `GET /release/lapce-x86_64.tar.gz.sha256` → compare. This pattern is used by many projects and "looks like" security but provides protection only against accidental corruption, not against a supply-chain compromise of the release server.

**How to avoid:**
- The hash (or a signing public key) must be embedded in the lapce binary itself at compile time, not fetched at runtime:
  ```rust
  // In update.rs — the hash for the NEXT version is known at build time
  // and baked in, or fetched from a separate trust anchor (e.g., a different CDN)
  const EXPECTED_SHA256: &str = env!("LAPCE_UPDATE_HASH"); // set at CI build time
  ```
- For a pragmatic first step that is meaningfully better than nothing: fetch the hash from a different URL origin than the binary (e.g., binary from GitHub releases, hash from `plugins.lapce.dev` API). An attacker would need to compromise both independently.
- The strongest option (out of scope for this milestone but worth flagging): use `minisign` or `ed25519` signatures with a public key compiled into the binary.
- For plugin downloads: the plugin registry API (`plugins.lapce.dev`) returns plugin metadata including a hash — use that API response hash, not a hash file fetched alongside the archive.

**Warning signs:**
- `GET {base_url}/{archive}` and `GET {base_url}/{archive}.sha256` where `base_url` is the same for both requests.
- No `env!()` macro or compile-time constant for expected hashes anywhere in `update.rs` or `proxy/remote.rs`.

**Phase to address:**
Security cluster — the trust model must be decided at design time. For this milestone, use the plugin registry API hash for plugins and embed the proxy hash at compile time. Document the residual risk for self-update (which cannot easily use a compile-time hash because the hash is for a future version).

---

### Pitfall 8: Verifying Archive Integrity After Extraction Instead of Before

**What goes wrong:**
Extracting a tar.gz or zip archive before verifying its SHA256 hash means that malicious archive entries (path traversal, symlinks, executable scripts) have already been written to disk before the check fails. The verification is useless as a security control if extraction happens first.

**Why it happens:**
The streaming API for archive extraction naturally processes bytes as they arrive, before the full download is complete. Developers sometimes add a hash check at the end of an already-streaming extraction loop — at that point, harm is already done.

**How to avoid:**
- Strict ordering: download completely → hash in memory → verify → extract. Never start extraction until the hash check passes.
- For large files where in-memory buffering is a concern (proxy binary is ~10 MB, plugin archives ~1-5 MB): these sizes are fine to buffer in memory on a desktop machine with 4+ GB RAM. Do not optimize prematurely with streaming extraction.
- If streaming extraction is genuinely required in the future, use a streaming authenticated hash (e.g., HMAC or a Merkle tree per-block scheme) — but this is not needed for lapce's use case.

**Warning signs:**
- `tar::Archive::unpack` or `ZipArchive::extract` called before `Sha256::digest` in the same function.
- Code structure: `download → loop { extract entry } → verify hash` — the loop runs before verify.
- Partially-extracted plugin directories in `~/.local/share/lapce/plugins/` when hash verification fails.

**Phase to address:**
Security cluster — the download-verify-extract ordering must be enforced in the initial implementation of integrity verification for all three paths (plugin, self-update, proxy binary).

---

### Pitfall 9: Zip Slip / Path Traversal in Archive Extraction (CVE-2025-29787)

**What goes wrong:**
Archive entries with paths like `../../.bashrc` or `/etc/passwd` (zip slip) or symlinks pointing outside the target directory cause writes outside the plugin installation directory. CVE-2025-29787 is specifically a symlink escape in `zip` crate versions 1.3.0–2.2.0: a symlink entry pointing to a directory outside the extraction root allows subsequent entries to be written to arbitrary filesystem paths.

**Why it happens:**
- `zip = "0.6.6"` (lapce's current pin) is vulnerable and has no path safety check on extraction.
- The `tar` + `zstd` path for plugin archives (`plugin/mod.rs:1592,1596`) calls `archive.unpack(&plugin_dir)` directly — the `tar` crate performs some path stripping but does not block symlink escapes on all platforms.

**How to avoid:**
- **ZIP**: Upgrade to `zip = "2"` (8.6.0 current, patched at 2.3.0). Call `ZipFile::enclosed_name()` explicitly for defense-in-depth even though 8.x validates internally:
  ```rust
  for i in 0..archive.len() {
      let mut file = archive.by_index(i)?;
      let outpath = match file.enclosed_name() {
          Some(path) => plugin_dir.join(path),
          None => continue, // path traversal detected, skip entry
      };
      // extract to outpath
  }
  ```
- **TAR (plugin archives)**: Add explicit path validation before `archive.unpack`:
  ```rust
  for entry in archive.entries()? {
      let entry = entry?;
      let path = entry.path()?;
      // reject absolute paths and any component that is ".."
      if path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
          return Err(anyhow!("path traversal in plugin archive"));
      }
  }
  // Only after validation passes:
  archive.unpack(&plugin_dir)?;
  ```
- Regression test: create a malicious tar.gz with a `../../evil` entry and assert that extraction returns an error without writing outside `plugin_dir`.

**Warning signs:**
- `zip = "0.6"` still present in `Cargo.toml` after the security hardening phase — this is a known-CVE dependency, treat it as a blocker.
- `archive.unpack(dest)` called without any prior path validation loop.
- No test for path traversal in `plugin/mod.rs` test suite.

**Phase to address:**
Security cluster — `zip` upgrade is mandatory (CVE). TAR path validation for plugin archives is equally mandatory. Both must land in the same PR as integrity verification so the security properties are holistic.

---

### Pitfall 10: `unwrap()` → `?` Migration That Changes Observable Behavior

**What goes wrong:**
Replacing `unwrap()` with `?` in a function that previously returned `()` forces a signature change to `Result<(), E>`. If callers discard the return value (common in Floem reactive closures and `std::thread::spawn` bodies), the error silently disappears — same as before. The migration "looks done" but has identical observable behavior to the original bug.

**Why it happens:**
The four `eprintln!` / `unwrap()` sites in `dispatch.rs` (lines 358, 369, 377, 385) are inside a `match` arm that currently returns `()`. Converting them to `?` requires: (1) the containing function to return `Result`, (2) callers to handle the `Result`, (3) the error to be sent back over the RPC channel to the UI. Step 3 is the one that gets skipped — propagating through `?` to the top of a thread closure with no receiver is effectively equivalent to `eprintln!`.

**How to avoid:**
- For `dispatch.rs` git errors: the fix is not just changing `unwrap()` to `?` but sending the error back through the existing RPC response channel so the UI can display it. Look at how successful git operations return their results — use the same pattern for errors.
- For `plugin/mod.rs:1590` (zstd decoder unwrap): the fix is returning `Err` from `download_volt`, which must then be caught by its caller (likely a `tokio::spawn` task) and sent as a notification to the frontend.
- For `dap.rs:104,105` (stdin/stdout unwrap): return `Result` from the DAP server launch function; the caller in `plugin/mod.rs` must propagate the error as a user-visible "DAP server failed to start" notification.
- Test criterion: the regression test must assert that the error appears in the UI (via the RPC notification), not just that it does not panic.

**Warning signs:**
- A function returns `()` but contains `?` operators — the `?` converts to a panic or is silently swallowed by `let _ = result;`.
- `spawn(|| { foo()?; Ok(()) })` with no `unwrap_or_else(|e| ...)` on the join handle — errors are lost at the thread boundary.
- `eprintln!` → `?` replacement without adding a corresponding `send_error_notification` call in the same diff.

**Phase to address:**
Crash cluster — each `unwrap()`/`eprintln!` replacement must be paired with a test that asserts the error reaches the user. Treat this as a UX fix, not just a code-style cleanup.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keep `reqwest::blocking` for one call site "just temporarily" | Faster to skip migration of one edge-case path | The nested runtime panic is all-or-nothing; one blocking call re-introduces the panic for that code path | Never — remove all blocking uses atomically |
| `unwrap()` → `expect("msg")` instead of `Result` propagation | No signature change needed | Same panic behavior, just slightly better crash messages | Never in production code paths; only in tests |
| Hashing the on-disk file instead of the in-memory buffer | Avoids holding the full archive in memory | Introduces TOCTOU race; the in-memory approach is correct and has no meaningful memory cost for <20 MB files | Never for security-critical paths |
| `zip = "2"` but skipping `enclosed_name()` check | Slightly simpler extraction loop | Defense-in-depth removed; relies entirely on library internals | Acceptable only if the crate's own path safety is formally documented and verified at upgrade time |
| Using `spawn_blocking` for all async work "to be safe" | Prevents blocking-in-async mistakes | Wastes a thread pool slot for genuinely async I/O; obscures where actual blocking occurs | Acceptable during initial migration; refine after profiling |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Floem + tokio | `#[tokio::main]` on `main()` — Floem needs to own main thread | `tokio::runtime::Builder::new_multi_thread().build()` + `rt.enter()` guard; then `floem::launch()` |
| Floem rfd file dialog + tokio | Keeping `rfd-async-std` feature after introducing tokio | Change floem feature to `rfd-tokio`; both async runtimes active simultaneously causes dialog failures |
| reqwest 0.12 + no tokio context | Building a `reqwest::Client` before calling `rt.enter()` | Ensure `rt.enter()` guard is active before any `reqwest::Client::new()` call |
| interprocess 2.x on macOS | Using `GenericNamespaced` (Linux abstract namespace) on macOS where it is unsupported | Use `GenericFilePath` unconditionally; it is supported on all three platforms |
| tokio `spawn_blocking` + plugin extraction | Running `zstd::Decoder` + `tar::Archive::unpack` on the tokio worker thread | Wrap the entire extract-and-verify block in `spawn_blocking`; it is synchronous CPU work |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `reqwest::blocking` inside `tokio::spawn` | Worker threads blocked; second download does not start while first is in progress | Remove `blocking` feature entirely; use `async fn` + `.await` | Immediately on first concurrent download attempt |
| `Sha256::digest` on large buffer on tokio worker thread | Other async tasks stall for 50-200 ms during hash computation | Use `spawn_blocking` for hash + extract | For archives >1 MB on worker_threads(2) |
| `tokio::runtime::Builder::new_multi_thread()` with default thread count | 8+ idle threads for a download-only runtime; memory overhead | Explicit `worker_threads(2)` for the lapce use case | Always — wastes resources even when not breaking |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Fetch expected hash over same HTTPS connection as binary | CDN/GitHub compromise delivers matching malicious binary + hash | Embed hash in binary at compile time, or use separate trust anchor for hash |
| Extract archive before verifying hash | Malicious entries written to disk before check fails; hash verification provides no protection | Enforce: download → verify → extract, never extract-then-verify |
| `zip = "0.6.6"` (CVE-2025-29787) | Symlink escape writes files outside plugin directory | Upgrade to `zip = "2"` (8.6.0); this is a known CVE, not a theoretical risk |
| TAR `archive.unpack(dir)` without path validation | `../../` entries escape plugin directory | Validate all entry paths before calling `unpack`; reject `ParentDir` components and absolute paths |
| `https_proxy` env var without scheme validation | `file://`, `data:` or other non-proxy schemes silently accepted; potential SSRF or local file access | Validate scheme is `http` or `https` before passing to `reqwest::Proxy::all()` |
| `spawn_blocking` closure capturing `Arc<Mutex<T>>` held across `.await` | Deadlock: async task holds lock, `spawn_blocking` task waits for lock on a blocking thread | Release all `MutexGuard`s before `.await` points; use `tokio::sync::Mutex` for guards held across `.await` |

---

## "Looks Done But Isn't" Checklist

- [ ] **Async migration:** `grep -rn "reqwest::blocking" lapce-app/src/ lapce-proxy/src/` returns zero results — if any remain, the nested runtime pitfall is still live.
- [ ] **Integrity verification:** All three download paths (plugin, self-update, proxy binary) have `verify_sha256` called before any `unpack`/`apply` — not just two of three.
- [ ] **Error propagation:** Each replaced `unwrap()` / `eprintln!` has a regression test that asserts the error is *visible to the user* (via RPC notification or UI alert), not just that it does not panic.
- [ ] **ZIP upgrade:** `zip = "2"` in `Cargo.toml` AND `enclosed_name()` check in every extraction loop — the upgrade without the check is defense-in-depth incomplete.
- [ ] **interprocess migration:** Single-instance behavior tested end-to-end — the API compiles with 2.x but single-instance detection can silently degrade if the wrong name type is used.
- [ ] **tokio runtime lifetime:** `rt` is a local variable in `main()`, not stored in any struct — verify with `grep -rn "tokio::runtime::Runtime" lapce-app/src/`.
- [ ] **spawn_blocking coverage:** `zstd::Decoder`, `tar::Archive::unpack`, and `Sha256::digest` on >1 MB data are all inside `spawn_blocking` — verify with `grep -n "zstd\|tar::\|Sha256::digest" lapce-proxy/src/` and cross-reference with `spawn_blocking` wrapping.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Nested runtime panic in production | LOW (if caught in CI) / HIGH (if shipped) | Add CI test: `cargo test -- --nocapture` with a smoke test that spawns two async tasks including a download; panic is caught in test output |
| TOCTOU race exploited (theoretical for lapce's threat model) | HIGH | Move to in-memory hash; audit all temp file paths for predictability; rotate any plugin signing keys |
| CVE-2025-29787 (zip symlink escape) | MEDIUM | Emergency `zip` upgrade PR; audit plugin directory contents for unexpected symlinks |
| Blocking call starving runtime (not caught in testing) | LOW | Wrap offending synchronous call in `spawn_blocking`; no architectural changes needed |
| interprocess 2.x silent single-instance failure | LOW | Integration test catches it; fix is correcting the `GenericFilePath` vs `GenericNamespaced` choice |
| Error propagation "looks done" but still swallowed | MEDIUM | Add test asserting RPC notification is received by frontend mock; cannot rely on "no panic" as success criterion |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Nested runtime panic (`block_on` in async context) | Performance — async runtime introduction | `grep -rn "reqwest::blocking"` returns zero; CI smoke test spawns concurrent async tasks |
| Blocking calls starving tokio runtime | Performance — async runtime introduction | `spawn_blocking` wrapping for all CPU-bound work; `tokio-console` check during plugin install test |
| Tokio runtime dropped while tasks in flight | Performance — async runtime introduction | `rt` is `main()`-scoped; no `Runtime` in any struct field (`grep -rn "Runtime" lapce-app/src/`) |
| reqwest 0.12 body/stream API regression | Deps — reqwest upgrade | All four download call sites compile and pass smoke tests; `socks` feature preserved |
| interprocess 2.x name API panic | Deps — interprocess upgrade | Integration test: launch two instances, second detects first; `strace` shows correct socket type on Linux |
| SHA256 TOCTOU (verify on-disk vs. in-memory) | Security — integrity verification | Code review: hash computed against `Bytes` buffer, not re-read file; no `std::fs::read` after `std::fs::write` in same flow |
| Hash fetched over same channel as binary | Security — integrity verification | Plugin hash sourced from registry API metadata; proxy hash compiled into binary; code review + trust model doc |
| Verify after extraction instead of before | Security — integrity verification | Unit test: provide malicious archive bytes; assert extraction never occurs when hash mismatches |
| Zip slip / path traversal (CVE-2025-29787) | Security — archive extraction hardening | `zip = "2"` in lock file; `enclosed_name()` in extraction loop; malicious-archive regression test |
| `unwrap()` → `?` without UI propagation | Crash — error propagation | Regression test per fix asserts error notification received by frontend; not just "no panic" |

---

## Sources

- tokio documentation: "Bridging with sync code" (https://tokio.rs/tokio/topics/bridging) — block_on nesting constraints, spawn_blocking pattern: HIGH confidence
- CVE-2025-29787 / zip crate advisory (https://github.com/zip-rs/zip2/security/advisories/GHSA-2rxp-6h9h-hm8j) — symlink escape in zip 1.3.0–2.2.0: HIGH confidence
- reqwest 0.12 changelog and hyper 1 migration notes — body API stability, feature flag changes: HIGH confidence
- interprocess 2.x changelog (https://github.com/kotauskas/interprocess/blob/main/CHANGELOG.md) — name API migration, sync API preservation: HIGH confidence
- CONCERNS.md codebase audit (2026-06-07) — specific file/line references for all lapce-specific pitfalls: HIGH confidence (direct codebase evidence)
- STACK.md research (2026-06-07) — tokio integration pattern with Floem, spawn_blocking guidance: HIGH confidence

---
*Pitfalls research for: Lapce hardening milestone — async runtime, dep migrations, integrity verification, error propagation*
*Researched: 2026-06-07*
