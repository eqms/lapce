# Architecture Research

**Domain:** Rust desktop code editor — async runtime introduction into sync std::thread + crossbeam-channel architecture
**Researched:** 2026-06-07
**Confidence:** HIGH

---

## Standard Architecture

### System Overview — Before (Current State)

```
lapce binary (main thread: Floem UI loop)
│
├── std::thread::spawn ──► update.rs: download_release()
│                              └── reqwest::blocking::Client (blocks OS thread)
│
├── std::thread::spawn ──► plugin.rs: install plugin
│                              └── lapce_proxy::get_url (blocks OS thread)
│
└── crossbeam_channel::unbounded ──► lapce-proxy (separate OS process)
                                          │
                                          ├── std::thread::spawn ──► get_url()
                                          │                            (plugin download, proxy bootstrap)
                                          └── reqwest::blocking::Client (blocks OS thread)
```

**Problem:** Each download monopolizes one OS thread for its entire duration. There is no shared executor, no cancellation, no backpressure. The blocking feature of reqwest 0.11 is also incompatible with the tokio 1.x ecosystem required by reqwest 0.12.

### System Overview — After (Target State)

```
lapce binary entry point  (lapce-app/src/bin/lapce.rs)
│
├── tokio::runtime::Builder::new_multi_thread()   ← lives HERE, one instance, pre-Floem
│   .worker_threads(2).enable_io().enable_time()
│   .build()
│   let _guard = rt.enter();   ← ambient context for all threads
│
└── floem::launch(app_view)   ← owns main thread as before
        │
        ├── Floem reactive signals / crossbeam_channel  (unchanged)
        │
        ├── tokio::spawn ──► DownloadPipeline (lapce-app/src/download.rs)
        │                       ├── fetch(url) → bytes        (reqwest::Client async)
        │                       ├── verify_sha256(bytes, expected)  (sha2 0.10)
        │                       └── Result<Bytes, DownloadError> → send via create_ext_action
        │
        ├── tokio::spawn ──► update.rs (uses DownloadPipeline)
        │
        └── tokio::spawn ──► plugin.rs (uses DownloadPipeline for icon/metadata fetches)


lapce-proxy binary  (lapce-proxy/src/bin/lapce-proxy.rs)
│
├── tokio::runtime::Builder::new_multi_thread()   ← lives HERE too, pre-mainloop
│   let _guard = rt.enter();
│
└── lapce_proxy::mainloop()   ← unchanged std::thread + crossbeam_channel structure
        │
        └── tokio::spawn ──► get_url() [migrated to async] → DownloadPipeline
                                ├── download_volt()  (plugin/mod.rs)
                                └── proxy bootstrap download  (called from app-side remote.rs)
```

---

## Component Boundaries

### Component: Tokio Runtime (one per process binary)

| | lapce-app | lapce-proxy |
|---|---|---|
| **Where** | `lapce-app/src/bin/lapce.rs`, before `floem::launch()` | `lapce-proxy/src/bin/lapce-proxy.rs`, before `mainloop()` |
| **Scope** | Ambient via `rt.enter()` guard held for process lifetime | Ambient via `rt.enter()` guard held for process lifetime |
| **Worker threads** | 2 (network I/O only; keeps footprint small; Floem owns main thread) | 2 (plugin downloads; proxy binary downloads) |
| **Features** | `rt-multi-thread`, `net`, `time`, `sync`, `macros` | same |
| **Floem interaction** | `floem::launch()` runs on main thread after guard; Floem never calls `block_on` | N/A (no Floem in proxy) |

**Critical rule:** Do not use `#[tokio::main]`. Do not call `Runtime::block_on` from inside a tokio async task. Use `tokio::spawn` + channels only.

### Component: DownloadPipeline (new shared helper)

**Location:** `lapce-app/src/download.rs`

This is the reusable component that all three download call sites migrate to. It encapsulates:
- Async HTTP fetch via `reqwest::Client` (non-blocking)
- Optional SHA256 integrity verification via `sha2 0.10`
- Structured error type: `DownloadError { kind: NetworkError | HashMismatch | ... }`
- `https_proxy` env var validation (scheme check before use)

```rust
pub struct DownloadPipeline {
    client: reqwest::Client,  // shared, cheap to clone (Arc internally)
}

impl DownloadPipeline {
    pub fn new(proxy_env: Option<&str>) -> Result<Self, DownloadError> { ... }

    /// Fetch bytes. If expected_sha256 is Some, verify before returning.
    /// Fails closed: returns Err(HashMismatch) if hash does not match — never returns data.
    pub async fn fetch_verified(
        &self,
        url: &str,
        expected_sha256: Option<&str>,
    ) -> Result<Bytes, DownloadError> { ... }

    /// Convenience: fetch + write to file, with optional hash check.
    pub async fn fetch_to_file(
        &self,
        url: &str,
        dest: &Path,
        expected_sha256: Option<&str>,
    ) -> Result<(), DownloadError> { ... }
}
```

**Why in lapce-app, not lapce-proxy or lapce-core:**
- All three download initiators (update.rs, plugin.rs, proxy/remote.rs) live in lapce-app.
- `download_volt` in lapce-proxy calls the existing `get_url` which lives in lapce-proxy; after migration it calls an async `get_url_async` in lapce-proxy (or delegates up via RPC — see note below).
- lapce-core must remain UI-framework-free and should not carry a tokio or reqwest dependency.
- The `sha2` crate is already used in lapce-app (`db.rs`, `plugin.rs`) — no new dep.

**Note on lapce-proxy's `download_volt`:** Because lapce-proxy is a separate binary, it cannot import from lapce-app. Two options exist:
1. Create a minimal `get_url_async` in `lapce-proxy/src/lib.rs` alongside the existing `get_url` (preferred — keeps the proxy self-contained for remote SSH scenarios where it runs without the app).
2. Route download requests back to the app process over RPC (overkill for this milestone).

Use option 1. The `DownloadPipeline` struct lives in lapce-app; an equivalent thin `AsyncHttpClient` wrapper lives in lapce-proxy. They share the same sha2 0.10 hash check logic (duplicated trivially, or extracted to lapce-core if it stays dependency-light).

### Component: Integrity Verifier (inline, not a separate crate)

**Not a new crate.** `sha2` is already present. Integrity verification is two functions:

```rust
// lapce-app/src/download.rs  (or lapce-proxy/src/lib.rs for the proxy side)
fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<(), DownloadError> {
    use sha2::{Digest, Sha256};
    let computed = format!("{:x}", Sha256::digest(data));
    if computed != expected_hex {
        return Err(DownloadError::HashMismatch { expected: expected_hex.to_owned(), got: computed });
    }
    Ok(())
}
```

The verifier is called inside `fetch_verified` before returning data. There is no code path where caller gets data without a passing hash check when `expected_sha256` is `Some`. Fail-closed by construction.

### Component: Migrated Call Sites

| File | Current pattern | Post-migration pattern |
|------|-----------------|------------------------|
| `lapce-proxy/src/lib.rs` `get_url()` | `reqwest::blocking::Client` | Keep function signature for callers that need it during migration; add `get_url_async()` alongside; deprecate blocking variant after all callers migrated |
| `lapce-proxy/src/plugin/mod.rs` `download_volt()` | calls `get_url` synchronously, unpacks inline | `async fn download_volt_async()` calling `get_url_async`, hash-verified, path-traversal guarded |
| `lapce-app/src/update.rs` `download_release()` | calls `get_url` synchronously | `tokio::spawn(DownloadPipeline::fetch_to_file(...))` + `create_ext_action` to report back |
| `lapce-app/src/plugin.rs` | multiple `get_url` calls in `std::thread::spawn` | replace outer `thread::spawn` with `tokio::spawn`; inner calls become `DownloadPipeline::fetch_verified()` |
| `lapce-app/src/proxy/remote.rs` `download` | `get_url` in `thread::spawn` | `tokio::spawn(DownloadPipeline::fetch_to_file(..., expected_hash))` |

---

## Data Flow

### Download with Integrity Check (target state)

```
User triggers install plugin / check update / connect remote
        │
        ▼
[UI thread — Floem]
  tokio::spawn(async move { ... })
        │
        ▼
[tokio worker thread]
  DownloadPipeline::fetch_verified(url, expected_sha256)
        │
        ├─── reqwest::Client::get(url).send().await
        │         └─── follows redirects (plugin: lapce.dev → S3)
        │
        ├─── response.bytes().await   ← all bytes in memory before verify
        │
        ├─── verify_sha256(&bytes, expected)
        │         ├── PASS → return Ok(bytes)
        │         └── FAIL → return Err(HashMismatch)  ← no data returned
        │
        └─── (for file writes) tokio::fs::write(dest, bytes).await
        │
        ▼
[create_ext_action callback → back to Floem reactive loop]
  match result {
      Ok(_) => update UI signal (install complete, update ready, proxy ready)
      Err(HashMismatch) => surface error to user via InternalCommand → alert
      Err(Network) => surface error to user
  }
```

**Key invariant:** Data never reaches the unpack / execute step unless `verify_sha256` has returned `Ok`. The `Result` type enforces this at compile time — there is no `bytes` value reachable from the `Err` branch.

### Plugin Download (proxy-side, same principle)

```
[lapce-proxy: install_volt called from PluginCatalog thread]
        │
        ▼
  tokio::spawn(async move {
      get_url_async(registry_url)          // get S3 redirect URL
          → get_url_async(s3_url)          // fetch archive bytes
          → verify_sha256(&bytes, hash)    // hash from registry API response
          → extract_archive_safe(&bytes, &plugin_dir)  // path-traversal guarded
  })
        │
        ▼
  catalog_rpc.core_rpc.volt_installed(meta, icon)   // notify UI via RPC
```

---

## Suggested Build Order

The five clusters below must be executed in this sequence. Each cluster unblocks the next.

### Cluster 1: Dependency Foundation (no behaviour change)

**Goal:** Get the workspace to compile with new dependency versions before touching any logic.

**Work:**
1. Upgrade `reqwest` 0.11 → 0.12 in workspace `Cargo.toml` (remove `blocking` feature).
2. Add `tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync", "macros"] }` to workspace deps.
3. Pin `toml = "0.8"`, replace tracing git-SHA pins with versioned releases.
4. Upgrade `zip` 0.6.6 → `"2"` (CVE fix; mandatory before any archive extraction changes).
5. Move `sha2` to workspace deps so lapce-proxy can use it.
6. Upgrade `interprocess` 1.2.1 → 2.x; migrate `ToFsName` call sites in `app.rs`.
7. Verify `alacritty_terminal` and `floem` pin status; change `rfd-async-std` → `rfd-tokio` in floem features.

**Blocking dependency:** Everything else depends on reqwest 0.12 being present. Without it, `reqwest::Client` (async) cannot be used, and the tokio integration has no payoff.

**Test gate:** `cargo build --workspace` succeeds. No behaviour change; existing blocking calls still compile because the migration has not happened yet. (Temporarily: the old `get_url` can keep `reqwest::blocking` if it is still a workspace dep with the `blocking` feature — drop blocking only after Cluster 3 is done.)

### Cluster 2: Runtime Introduction (infrastructure only)

**Goal:** Stand up the tokio runtime in both binaries without touching any call sites.

**Work:**
1. In `lapce-app/src/bin/lapce.rs`: construct `tokio::runtime::Builder::new_multi_thread()`, call `rt.enter()`, then call `floem::launch()`. Keep `_guard` alive for process lifetime.
2. In `lapce-proxy/src/bin/lapce-proxy.rs`: same pattern before `mainloop()`.
3. Run the app and confirm Floem still owns the main thread; no nested-runtime panics; no behavioural difference.

**Blocking dependency:** Cluster 1 (tokio dep present in workspace).

**Test gate:** App launches, LSP works, terminal works. No regression. The runtime is ambient but unused — no spawn calls yet.

### Cluster 3: DownloadPipeline + Migrate Call Sites

**Goal:** Replace all `reqwest::blocking` call sites with async equivalents.

**Work, in order (smallest blast radius first):**

1. Write `lapce-app/src/download.rs` with `DownloadPipeline` (async fetch, no verify yet — keep verify optional for now to decouple from hash availability).
2. Migrate `lapce-app/src/update.rs` `download_release()`: replace `thread::spawn + get_url` with `tokio::spawn(DownloadPipeline::fetch_to_file)` + `create_ext_action`. Update `get_latest_release()` to use async `reqwest::Client`.
3. Migrate `lapce-app/src/proxy/remote.rs` proxy binary download.
4. Migrate `lapce-app/src/plugin.rs` icon/metadata fetches (the `get_url` calls not going through `download_volt`).
5. Add `get_url_async()` to `lapce-proxy/src/lib.rs` (async version of `get_url`; keep blocking variant temporarily).
6. Migrate `lapce-proxy/src/plugin/mod.rs` `download_volt()` to async.
7. Drop `reqwest` `blocking` feature once all call sites are migrated.

**Blocking dependency:** Cluster 2 (runtime must be ambient before `tokio::spawn` works outside a `#[tokio::main]` context).

**Test gate:** Plugin install, self-update check, and remote SSH proxy bootstrap all work. No blocking calls remain (verify with `cargo grep reqwest::blocking` returning nothing).

### Cluster 4: Integrity Verification

**Goal:** Fail-closed SHA256 verification on every download path.

**Work:**
1. Add `verify_sha256()` to `lapce-app/src/download.rs` and a matching copy in `lapce-proxy/src/lib.rs` (or extract to `lapce-core` if it gains no new deps).
2. Wire verification into `DownloadPipeline::fetch_verified()`.
3. Audit registry/GitHub API responses for published hash fields:
   - Plugin registry (`plugins.lapce.dev/api/v1/plugins/.../download`): check if response body or headers carry a hash; if not, the API needs extension — flag as a gap.
   - GitHub releases API: releases carry a SHA in a separate `*.sha256` companion asset; fetch that asset and use it.
   - Remote proxy binary: same GitHub releases pattern.
4. Implement path-traversal guard in `download_volt`: iterate entries, check `entry.path()?.components()` for `..` or absolute components; skip/error on violation.
5. Validate `https_proxy` scheme (must be `http://` or `https://`) before passing to `reqwest::Proxy::all`.

**Blocking dependency:** Cluster 3 (all downloads must go through the async pipeline before verification can be wired in uniformly).

**Test gate:** Unit tests for `verify_sha256` (pass/fail cases). Integration test: feed a download with a mismatched hash, confirm `DownloadError::HashMismatch` is returned and no file is written.

### Cluster 5: Performance Tuning (opportunistic)

**Goal:** Cache hot-path computations and reduce clone overhead where profiling justifies it.

**Work (independent from async migration; can start after Cluster 1):**
- Cache compiled `GlobMatcher` in `file_explorer/data.rs:207`.
- Cache parsed font families in `doc.rs:1951`.
- Box large enum variants in plugin/DAP message types to reduce copy cost.

**Blocking dependency:** None — this cluster is independent. It can be worked in parallel with Clusters 2–4 if bandwidth allows, or deferred after Cluster 4.

**Note:** Clone reduction in render hot paths (`doc.rs`, `editor.rs`) requires profiling first (`cargo-flamegraph`). Do not guess which clones are hot — measure.

---

## Architectural Patterns

### Pattern 1: Runtime-Enter Instead of `#[tokio::main]`

**What:** Construct the runtime manually, call `rt.enter()` to make it ambient, then hand the main thread to Floem. No `#[tokio::main]` attribute.

**When to use:** Whenever a framework (Floem, GTK, winit) requires ownership of the main thread. Tokio's `#[tokio::main]` macro calls `block_on` on main — incompatible with Floem's own event loop.

**Trade-offs:** Slightly more boilerplate in the entry point. The `_guard` must be kept alive (not dropped) for the process lifetime — assign to a named binding, never `_`.

```rust
// lapce-app/src/bin/lapce.rs
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let _guard = rt.enter(); // ambient for all subsequent thread::spawn and tokio::spawn
    lapce_app::app::launch();  // calls floem::launch() internally
}
```

### Pattern 2: `create_ext_action` for Async-to-Floem Bridging

**What:** Floem's `create_ext_action` creates a callback that is safe to call from any thread (including tokio worker threads) and delivers the result into the reactive loop as a Floem event.

**When to use:** Every `tokio::spawn` that needs to update UI state must use this bridge. Never mutate `RwSignal` directly from a tokio task.

**Trade-offs:** One extra allocation per download completion. Worth it — the alternative (channels + polling) is more boilerplate and error-prone.

```rust
// in a Floem component setup (runs on main thread):
let send = create_ext_action(cx, move |result: Result<(), DownloadError>| {
    match result {
        Ok(()) => install_state.set(InstallState::Done),
        Err(e) => install_state.set(InstallState::Failed(e.to_string())),
    }
});

// then in a tokio task:
tokio::spawn(async move {
    let result = pipeline.fetch_to_file(url, dest, expected_hash).await;
    send(result);
});
```

### Pattern 3: Fail-Closed Verification

**What:** The `fetch_verified` function returns `Result<Bytes, DownloadError>`. The `Bytes` are only reachable from the `Ok` branch. The `Err(HashMismatch)` branch never carries data. Callers cannot accidentally use unverified bytes.

**When to use:** All three download paths (plugin, update, proxy binary).

**Trade-offs:** Requires buffering the full download into memory before verifying. Acceptable for the file sizes involved (plugin archives ~1–20 MB, update binaries ~30–50 MB). For very large files, a streaming hash (feed `Sha256::update` per chunk) is possible — but the `copy_to` + `Bytes` pattern is simpler and the sizes do not justify streaming complexity in this milestone.

---

## Anti-Patterns

### Anti-Pattern 1: Nested Runtime

**What people do:** Call `tokio::runtime::Handle::current().block_on(future)` from inside an already-running tokio task.

**Why it's wrong:** Panics with "Cannot start a runtime from within a runtime". This is the most common tokio footgun when bridging sync and async.

**Do this instead:** Use `tokio::spawn` and send results back via channels or `create_ext_action`. Never call `block_on` from within an async context.

### Anti-Pattern 2: Keeping `reqwest::blocking` After Runtime Introduction

**What people do:** Add the tokio runtime in the entry point but leave `reqwest::blocking` call sites in place because they "still compile".

**Why it's wrong:** `reqwest::blocking` internally creates its own mini tokio runtime. With the outer multi-thread runtime ambient, this causes a "Cannot start a runtime from within a runtime" panic on first use.

**Do this instead:** Migrate all `reqwest::blocking` call sites to async `reqwest::Client` before activating the outer runtime — or, if migration must be incremental, ensure blocking calls happen in `std::thread::spawn` threads that have not entered the tokio context.

### Anti-Pattern 3: `verify_sha256` After Unpack

**What people do:** Download → unpack → verify. If verification fails, delete the already-unpacked files.

**Why it's wrong:** A TOCTOU window exists between unpack and delete. Malicious content has already touched the filesystem. Fail-open by construction.

**Do this instead:** Download → verify → unpack. The verifier runs on the raw bytes before any extraction happens. Extraction never runs if the hash does not match.

### Anti-Pattern 4: One Runtime Per Download

**What people do:** Create a `tokio::runtime::Runtime` inside each `std::thread::spawn` closure (one per plugin download).

**Why it's wrong:** Each runtime spawns its own thread pool. Multiple runtimes fighting over CPU cores; significant overhead; no backpressure.

**Do this instead:** One runtime per binary, constructed at entry. All async tasks share the same thread pool via `tokio::spawn`.

---

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| tokio task → Floem UI | `create_ext_action` callback | Only safe crossing point; do not mutate signals from tokio threads |
| lapce-app → lapce-proxy | JSON-RPC over stdio (`lapce-rpc`) | Unchanged; crossbeam-channel-based; no tokio involvement |
| `DownloadPipeline` → integrity verifier | inline call within `fetch_verified` | No interface needed; single function, same module |
| lapce-proxy `get_url_async` → archive extractor | sequential in same async task | Extract after verify; never in parallel |

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| `plugins.lapce.dev` registry API | async GET via `reqwest::Client`; follow redirect to S3 | Check if API returns a hash field; if not, SHA256 companion asset needed (gap) |
| GitHub releases API | async GET; parse `assets` array; fetch `*.sha256` companion asset | SHA256 files are convention, not guaranteed — verify presence |
| SSH remote host | unchanged: `std::process::Command` subprocess, stdio piped | No tokio involvement; remote.rs async migration covers only the proxy binary download step |
| `https_proxy` env var | validate scheme before `reqwest::Proxy::all()` | Accept only `http://` and `https://` prefixes; reject others with a clear error |

---

## Sources

- Tokio documentation: "Bridging with sync code" — https://tokio.rs/tokio/topics/bridging (runtime-enter pattern, `block_on` pitfalls): HIGH confidence
- STACK.md (this project) — runtime placement decision, reqwest 0.12 rationale, sha2 usage pattern, floem `rfd-tokio` flag: HIGH confidence
- Floem source (`Cargo.toml` main branch, floem optional tokio dep with `features = ["sync", "rt"]`): HIGH confidence
- `lapce-proxy/src/lib.rs` `get_url()` (read directly): HIGH confidence
- `lapce-app/src/update.rs`, `plugin.rs`, `proxy/remote.rs` call sites (read directly): HIGH confidence
- `lapce-proxy/src/plugin/mod.rs` `download_volt()` (read directly): HIGH confidence

---

*Architecture research for: Lapce hardening — async runtime introduction + integrity verification pipeline*
*Researched: 2026-06-07*
