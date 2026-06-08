---
phase: 03-download-pipeline-crash-fixes
reviewed: 2026-06-08T12:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - Cargo.toml
  - lapce-app/src/app.rs
  - lapce-app/src/app/grammars.rs
  - lapce-app/src/app/logging.rs
  - lapce-app/src/download.rs
  - lapce-app/src/keypress/condition.rs
  - lapce-app/src/keypress/loader.rs
  - lapce-app/src/lib.rs
  - lapce-app/src/plugin.rs
  - lapce-app/src/proxy/remote.rs
  - lapce-app/src/update.rs
  - lapce-proxy/src/cli.rs
  - lapce-proxy/src/dispatch.rs
  - lapce-proxy/src/lib.rs
  - lapce-proxy/src/plugin/catalog.rs
  - lapce-proxy/src/plugin/dap.rs
  - lapce-proxy/src/plugin/mod.rs
findings:
  critical: 4
  warning: 6
  info: 3
  total: 13
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-06-08T12:00:00Z
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

This phase migrated 11 `reqwest::blocking` call sites to async `reqwest` via a
`get_url_async` core and a `Handle::current().block_on(...)` sync shim, dropped
the `blocking` Cargo feature, and patched five panic sites (CRASH-01..05) so
errors surface as UI notifications rather than hard crashes.

The overall structure is sound: the ambient runtime is properly established in
both entry-point binaries before `Handle::current()` is ever called, and the
five CRASH fixes are genuine improvements. However, several issues remain that
can cause panics or silent data loss under reachable conditions:

- Two index-slice panics on short `target_commitish` strings survive from the
  pre-phase code and are still reachable through the new async path.
- `Handle::current().block_on(get_url_async(...))` inside `get_url` will panic
  if called from within an existing tokio async context (nested-runtime).
- `download_remote` in `proxy/remote.rs` silently continues after a download
  failure instead of propagating the error.
- Three `expect`/`unwrap` calls in `proxy/remote.rs` are unguarded panics.
- The `res` variable in `logging.rs` is unused, masking whether the OS
  notification was dispatched.
- `unsafe { std::env::set_var(...) }` in `lib.rs` is data-race-unsafe under
  multi-threaded startup.

---

## Critical Issues

### CR-01: Panic on `target_commitish` shorter than 7 bytes

**File:** `lapce-app/src/update.rs:48`, `lapce-app/src/app/grammars.rs:106`
**Issue:** Both sites slice `&release.target_commitish[..7]` unconditionally.
The GitHub API returns the full branch name (e.g. `"main"`) rather than a
commit hash when the nightly release points to a branch, making `target_commitish`
as short as 4 bytes. A byte-slice on a shorter string panics at runtime with
`index out of range`. This is a CRASH-01-class regression: the new async path
surfaces the response text correctly but does nothing to guard the subsequent
slice. Because `get_latest_release` and `find_grammar_release` run on worker
threads, the panic kills that thread silently and the UI receives no update, but
on some platforms the panic hook calls `error_notification`/`error_modal`, so
the impact is visible.

**Fix:**
```rust
// update.rs:48  and  grammars.rs:106 — replace the bare slice with:
let short = if release.target_commitish.len() >= 7 {
    &release.target_commitish[..7]
} else {
    &release.target_commitish
};
format!("{}+Nightly.{}", env!("CARGO_PKG_VERSION"), short)
// or in grammars.rs:
format!("nightly-{}", short)
```

---

### CR-02: `get_url` / `block_on` nested-runtime panic

**File:** `lapce-proxy/src/lib.rs:228`
**Issue:** `pub fn get_url` is implemented as:
```rust
Handle::current().block_on(get_url_async(url, user_agent))
```
`Handle::current()` itself panics if there is no tokio runtime context on the
calling thread. More critically, `tokio::runtime::Handle::block_on` **panics
if called from within an async task** (`cannot block the current thread from
within the async context`). Every caller of `get_url` that runs inside an async
task — current or future — will hit this panic. At present all callers are on
OS threads, not async tasks, so the code works today; but the function is
`pub`, carries no `# Safety` or `# Panics` contract in its doc, and the
constraint is entirely invisible to callers. Any future refactor that awaits
this path inside an async context will panic, not return an error.

The correct fix is to use `tokio::task::block_in_place` (for multi-thread
runtimes) which yields the scheduler slot rather than blocking the runtime
thread, making the call safe from within async contexts:

**Fix:**
```rust
pub fn get_url<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    tokio::task::block_in_place(|| {
        Handle::current().block_on(get_url_async(url, user_agent))
    })
}
```
Additionally, document the `# Panics` contract: panics if called with no
tokio runtime entered on the calling thread.

---

### CR-03: `download_remote` continues silently after proxy download failure

**File:** `lapce-app/src/proxy/remote.rs:354-363`
**Issue:** When the HTTP download of the remote proxy binary fails (non-2xx
status), the code logs the error and falls through without returning `Err`:

```rust
if resp.status().is_success() {
    // ... write file ...
} else {
    error!("proxy download failed with: {}", resp.status());
    // NO RETURN / NO ERROR PROPAGATION
}
// execution continues: upload_file is then called with a non-existent
// or empty local_proxy_file
```

This means `remote.upload_file(&local_proxy_file, remote_proxy_file)` is
called with a file that was never written (or contains stale content from a
previous failed attempt). The subsequent SSH connection will then attempt to
run a corrupted proxy, producing a confusing and hard-to-diagnose failure.

**Fix:**
```rust
} else {
    return Err(anyhow!(
        "proxy download failed with status: {}",
        resp.status()
    ));
}
```

---

### CR-04: Three unguarded `expect` / `unwrap` panics in `proxy/remote.rs`

**File:** `lapce-app/src/proxy/remote.rs:93, 262, 357-360`
**Issue:** Three panic-capable paths remain in the remote-proxy bootstrap code:

1. **Line 93**: `host_specification(&remote).unwrap()` — `host_specification`
   returns `Result` and the `Err` branch propagates SSH command failure. The
   `.unwrap()` will panic if the SSH transport itself errors rather than
   returning an unknown platform.

2. **Line 262**: `Directory::proxy_directory().unwrap().join("proxy.ps1")` —
   `proxy_directory()` returns `Option<PathBuf>`. The `.unwrap()` panics if the
   platform data directory is unavailable (e.g., sandboxed environments).

3. **Lines 357, 360**: `.expect("failed to create file")` and
   `.expect("failed to copy content")` — file creation and `io::copy` failures
   are converted to panics rather than propagating as `Err`.

All three are in a code path that was explicitly targeted by this phase for
panic elimination. They should return `Err` so the caller (`start_remote`) can
surface the error to the UI via `show_message`.

**Fix (illustrative for line 93):**
```rust
let (platform, architecture) = host_specification(&remote)?;
```
**Fix (line 262):**
```rust
let local_proxy_script = Directory::proxy_directory()
    .ok_or_else(|| anyhow!("can't find proxy directory"))?
    .join("proxy.ps1");
```
**Fix (lines 357–360):**
```rust
let mut out = std::fs::File::create(&local_proxy_file)
    .with_context(|| format!("failed to create {local_proxy_file:?}"))?;
std::io::copy(&mut gz, &mut out)
    .context("failed to decompress proxy binary")?;
```

---

## Warnings

### WR-01: Retry loop in `get_url_async` silently drops errors

**File:** `lapce-proxy/src/lib.rs:213-221`
**Issue:** The retry loop has an off-by-one in its intent and silently discards
intermediate errors:
```rust
let mut try_time = 0;
loop {
    let rs = client.get(url.clone()).send().await;
    if rs.is_ok() || try_time > 3 {
        return Ok(rs?);  // rs? panics-via-? if rs is Err here
    } else {
        try_time += 1;
    }
}
```
When `try_time > 3` (i.e., 4 attempts made), if `rs` is still `Err`, the code
executes `Ok(rs?)` which propagates the error correctly via `?`, but the outer
`Ok(...)` wrapper is misleading — it returns the inner `Err` as a `Result::Err`
because `?` short-circuits. The code is accidentally correct, but hard to reason
about. Additionally, the condition `try_time > 3` means 5 attempts (0,1,2,3,4)
not 4, which may not match the intended retry count. All intermediate errors are
silently swallowed.

**Fix:**
```rust
let mut last_err = None;
for _ in 0..4 {
    match client.get(url.clone()).send().await {
        Ok(resp) => return Ok(resp),
        Err(e) => last_err = Some(e),
    }
}
Err(last_err.expect("loop ran at least once").into())
```

---

### WR-02: `install_volt` error path does not surface start failure to UI

**File:** `lapce-proxy/src/plugin/mod.rs:1607-1631`
**Issue:** When `start_volt` fails after a successful download, the error is
only logged (`tracing::error!`) but not reported to the user via `core_rpc`.
The `volt_installed` notification is then still sent (line 1629), making the UI
believe the plugin is ready when its server process never started.

**Fix:** Mirror the `download_volt_result.is_err()` pattern used above:
```rust
if let Err(err) = start_volt(...) {
    tracing::error!("{:?}", err);
    catalog_rpc.core_rpc.volt_installing(
        volt.clone(),
        format!("Could not start plugin: {err}"),
    );
    return Err(err);
}
```

---

### WR-03: Grammar `download_release` returns `Ok(true)` even when no asset matched

**File:** `lapce-app/src/app/grammars.rs:115-148`
**Issue:** `download_release` iterates `release.assets` looking for an asset
whose name starts with `file_name`. If no asset matches (e.g., the release does
not include a grammar for the current OS/arch combination), the function falls
through the loop and returns `Ok(true)` — reporting that an update was
successfully applied when nothing was actually downloaded or written. The version
file is not updated in this path (correct), but the caller receives a misleading
`true` result that triggers `reset_highlight_configs()` in `app.rs`.

**Fix:**
```rust
// After the loop, if no asset was matched:
Ok(false)  // or return Err if the asset is expected to exist
```
The closing `Ok(true)` at line 148 should be `Ok(false)` and the `Ok(true)`
should only be returned inside the `for` block after successful extraction.

---

### WR-04: `unsafe { std::env::set_var }` in multi-threaded context

**File:** `lapce-proxy/src/lib.rs:160-162`
**Issue:** `std::env::set_var` is declared `unsafe` in Rust 1.87+ precisely
because it is not signal/thread-safe. In `mainloop()`, `register_lapce_path()`
is called after `stdio_transport` has already spawned reader/writer threads.
Mutating the environment while other threads are running (including any that
might read `PATH` or call `getenv`) is undefined behaviour on POSIX systems.

**Fix:** Either call `register_lapce_path()` before any threads are spawned, or
replace the unsafe `set_var` with a safe alternative for the intended purpose
(prepending to `PATH` for child-process spawning can be done per-`Command` via
`Command::env`).

---

### WR-05: `fs::create_dir` (not `create_dir_all`) in `download_release`

**File:** `lapce-app/src/app/grammars.rs:99-101`
**Issue:**
```rust
if !dir.exists() {
    fs::create_dir(&dir)?;
}
```
`fs::create_dir` fails with `NotFound` if any parent directory does not yet
exist (e.g., on a fresh install where the entire `lapce/grammars/` subtree is
absent). This converts what should be a transparent first-run operation into an
error that bubbles up as "failed to fetch grammars". `create_dir_all` is the
correct primitive for this pattern.

**Fix:**
```rust
fs::create_dir_all(&dir)?;
```

---

### WR-06: Unused `res` binding in `error_notification` suppresses spawn errors

**File:** `lapce-app/src/app/logging.rs:140-153`
**Issue:** The result of `std::process::Command::new("notify-send").spawn()` is
bound to `res` but never inspected:
```rust
let res = std::process::Command::new("notify-send")
    ...
    .spawn();
```
The compiler emits an `unused variable: res` warning. More importantly, if
`notify-send` is not installed (common on minimal Linux systems), the spawn
error is silently discarded — no fallback notification reaches the user.
This is the known pre-existing warning mentioned in the review brief, but it
does mask a real fail-open: panics on Linux systems without `notify-send` are
silently dropped rather than logged.

**Fix:** Either use `_res` to acknowledge intentional discard, or log on error:
```rust
if let Err(err) = std::process::Command::new("notify-send")
    .args([...])
    .spawn()
{
    tracing::error!("failed to spawn notify-send: {err:?}");
}
```

---

## Info

### IN-01: `DownloadPipeline` wrapper adds zero value

**File:** `lapce-app/src/download.rs`
**Issue:** The `DownloadPipeline` struct exists solely to delegate to
`lapce_proxy::get_url`. It has no state, no additional logic, and every call
site could use `lapce_proxy::get_url` directly (as the proxy-side callers do).
The wrapper creates a false impression of abstraction without providing any.

**Fix:** Remove `DownloadPipeline` and call `lapce_proxy::get_url` directly
at the one call site in `lapce-app`, or add the planned functionality that
justifies the wrapper.

---

### IN-02: `https_proxy` env var only; `http_proxy` and `HTTP_PROXY` ignored

**File:** `lapce-proxy/src/lib.rs:201-208`
**Issue:** Proxy detection reads only `https_proxy`. The conventional variables
`http_proxy`, `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` are ignored. Users
with proxy configurations relying on uppercase or `ALL_PROXY` will silently
bypass the proxy.

**Fix:** Check variables in order of precedence, or delegate to `reqwest`'s
built-in proxy detection (`reqwest::Client::builder()` reads system proxy
settings automatically when no explicit proxy is set).

---

### IN-03: Nightly version string in `download_release` (grammars) truncates
          non-ASCII commitish

**File:** `lapce-app/src/app/grammars.rs:106`
**Issue:** `&release.target_commitish[..7]` is a byte-slice (noted in CR-01).
Even after guarding the length, if the commit hash contains multibyte UTF-8
characters (unlikely but technically possible with custom Git configurations),
the slice may split a character boundary. Use `chars().take(7)` or
`get(..7)` with a fallback.

**Fix:**
```rust
let short: String = release.target_commitish.chars().take(7).collect();
format!("nightly-{short}")
```

---

_Reviewed: 2026-06-08T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
