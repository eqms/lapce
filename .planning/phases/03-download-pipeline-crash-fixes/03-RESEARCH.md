# Phase 3: Download Pipeline + Crash Fixes - Research

**Researched:** 2026-06-08
**Domain:** Rust async HTTP migration (reqwest 0.12 blocking→async), panic elimination, RPC error surfacing
**Confidence:** HIGH — all findings verified against source code; no external docs required

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Migrate EVERY `get_url` caller (not just app-side RT-03 sites). Proxy-side `download_volt` (`lapce-proxy/src/plugin/mod.rs:1561,1569`) and `lapce-app/src/app/grammars.rs` (lines 14, 113) are also migrated. Success Criterion #1 requires zero `reqwest::blocking` workspace-wide.
- **D-02:** Async HTTP core stays in `lapce-proxy`. `get_url` replaced by async implementation in `lapce-proxy/src/lib.rs`. `DownloadPipeline` in `lapce-app/src/download.rs` is a thin wrapper around the shared proxy-side core — does NOT spin a second independent `reqwest::Client`.
- **D-03:** Bridge sync→async at call sites via `tokio::runtime::Handle::current().block_on(...)`. No `async`/`.await` pushed up call chains. Minimal-invasive: no function signature changes at call sites.
- **D-04:** `CoreNotification::ShowMessage { message: ShowMessageParams }` (`lapce-rpc/src/core.rs:78`) is the single, uniform channel for all proxy-side error notifications. Already wired; already rendered.
- **D-05:** Severity for failed user-triggered operations = `MessageType::ERROR`.
- **D-06:** `dispatch.rs:1343` `.unwrap()` in `handle_workspace_fs_event` → guard with `Option` check + **early return / no-op** (no user toast from background fs-event handler).
- **D-07:** User-triggered git command arms `dispatch.rs:354–388` (`GitCheckout`, `GitDiscardFilesChanges`, `GitDiscardWorkspaceChanges`, `GitInit`) — add missing else-branch emitting `ShowMessage "No folder open"` when workspace is None.
- **D-08:** CRASH-05: replace `eprintln!("{e:?}")` in same arms (lines 358, 369, 377, 385) with `ShowMessage` (ERROR).
- **D-09:** `check_condition` evaluator (`lapce-app/src/keypress.rs:524`) is already panic-free. Phase work = regression test locking the non-panic guarantee + load-time diagnostic.
- **D-10:** Load-time condition diagnostic: emit `tracing::warn!` in `KeyMapLoader::load_from_str` for unparseable `when` conditions. Eval-time stays silent skip.
- **D-11:** Lock eval-time semantics as test contract: unknown condition → `false`; `!unknown` → `true`. No behaviour change.
- **D-12:** CRASH-04: replace `zstd::Decoder::new(&mut resp).unwrap()` (`plugin/mod.rs:1590`) with `?` propagation.
- **D-13:** CRASH-03: replace DAP stdio-capture `.unwrap()`s (`plugin/dap.rs:104,105`) with error returns.
- **Transport semantics preserved (Discretion):** Preserve 10s timeout and up-to-3 retry loop from current `get_url`.

### Claude's Discretion

- Whether CRASH-04's plugin error uses `ShowMessage` directly vs. the existing `VoltInstalling { error }` install-feedback path.
- Exact test-seam mechanism for asserting "error reached the UI as a notification".
- Whether to factor the async download core + retry/timeout logic into a shared helper vs. inline.

### Deferred Ideas (OUT OF SCOPE)

- SHA256 integrity verification (SEC-01..03) — Phase 4.
- Path-traversal / symlink-escape guards in archive extraction (SEC-04) — Phase 4.
- `https_proxy` scheme validation (SEC-05) — Phase 4.
- Pushing async up through call chains / concurrent download pool — Phase 5 / v2.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RT-02 | All network I/O migrated off `reqwest::blocking`; zero blocking call sites after runtime active | §Standard Stack, §Call Site Inventory, §Migration Pattern |
| RT-03 | Shared `DownloadPipeline` in `lapce-app/src/download.rs` wraps async reqwest; app-side call sites use it | §Standard Stack, §Architecture Patterns |
| CRASH-01 | Compound keybinding conditions (AND/OR/NOT) evaluate without panicking | §Crash Site Inventory — CRASH-01 |
| CRASH-02 | Git operations with no open workspace fail gracefully, not panic | §Crash Site Inventory — CRASH-02 |
| CRASH-03 | DAP server stdio-capture failure returns error instead of panicking | §Crash Site Inventory — CRASH-03 |
| CRASH-04 | Malformed zstd plugin archive returns error instead of panicking | §Crash Site Inventory — CRASH-04 |
| CRASH-05 | Failed git operations surface to user via RPC instead of swallowed by `eprintln!` | §Crash Site Inventory — CRASH-05 |
| TEST-01 | Every crash fix ships with regression test asserting error reaches UI as notification | §Regression Test Strategy |
</phase_requirements>

---

## Summary

Phase 3 has two coupled deliverables: (1) migrate all blocking HTTP I/O onto the ambient tokio runtime Phase 2 introduced, and (2) fix five crash/error-swallow sites so every failure surfaces as a user-visible notification. The two deliverables are coupled by a hard constraint: `reqwest::blocking` panics inside an active tokio context (STATE.md Critical Pitfall #1), so the feature must be dropped in the same commit batch that wires call sites onto the async pipeline.

The research confirms all CONTEXT.md decisions are technically sound and verified against the current codebase. The `check_condition` evaluator (CRASH-01) is **already panic-free** at runtime; the phase work there is purely a regression test + load-time warning. The four remaining panic sites are confirmed at precise file:line locations. `CoreRpcHandler` already implements `rx()` returning a `crossbeam_channel::Receiver<CoreRpc>` — the test seam for asserting notification emission is simply: construct a `CoreRpcHandler::new()`, pass it into the code under test, and `recv()` on `handler.rx()` after the trigger.

**Primary recommendation:** Implement in three ordered plan units — (A) async `get_url` core in `lapce-proxy` + `DownloadPipeline` wrapper + drop `blocking` feature; (B) crash fixes for CRASH-02/03/04/05; (C) CRASH-01 regression test + load-time warning. Units (B) and (C) can proceed in parallel once (A) is complete.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HTTP download (plugin, update, grammar, proxy) | Proxy (lapce-proxy) | App wrapper (lapce-app/download.rs) | `get_url` already lives in lapce-proxy; app is a separate process that depends on lapce-proxy; shared core avoids duplication |
| Async runtime bridge (block_on at call sites) | Call site (whatever thread is running) | — | D-03: obtain handle via `Handle::current()`, no stored handle |
| Error notification to UI | lapce-proxy Dispatcher / plugin catalog | lapce-rpc CoreRpcHandler | `CoreNotification::ShowMessage` is the existing, rendered channel |
| Keybinding condition parsing | lapce-app (UI process) | — | Keymaps loaded at app startup; evaluation is UI-side only |
| DAP process management | lapce-proxy plugin catalog | — | `DapClient::start` spawns process in proxy; stdio capture is proxy-local |
| Git operations | lapce-proxy Dispatcher | — | `dispatch.rs` handles all git RPC messages; workspace is proxy-side `Option<PathBuf>` |

---

## Standard Stack

### Core (already in Cargo.toml — no new deps required)

| Library | Version | Purpose | Note |
|---------|---------|---------|------|
| `reqwest` | 0.12.28 (workspace) | Async HTTP client | Feature `blocking` is DROPPED in this phase; async `reqwest::Client` is the default |
| `tokio` | 1.52.3 (workspace) | Runtime + `Handle::current()` + `block_on` | Already ambient from Phase 2; `rt-multi-thread` + `sync` + `time` features |
| `anyhow` | workspace | Error propagation (`?`, `.context()`) | Convention; `.unwrap()` replacements use `?` |
| `tracing` | workspace (git rev) | `warn!` for load-time condition diagnostic | Existing `debug!`/`error!` already imported in loader |
| `zstd` | 0.11.2 (workspace) | Zstd decompressor for plugin archives | `zstd::Decoder::new(impl Read)` — sync; takes `std::io::Cursor` after body bytes loaded |
| `crossbeam-channel` | 0.5.12 (workspace) | Test seam — `CoreRpcHandler::rx()` returns receiver | Used in regression tests to assert notification emission |

[VERIFIED: codebase grep of Cargo.toml and Cargo.lock]

### No New Dependencies

This phase adds zero new Cargo dependencies. All capabilities needed are already in the workspace. The only Cargo.toml change is **removing** the `"blocking"` feature from the `reqwest` entry.

---

## Package Legitimacy Audit

> No new packages are installed in this phase. The only Cargo change is a feature removal.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Call Site Inventory

### All `reqwest::blocking` Call Sites (workspace-wide)

[VERIFIED: `grep -rn "reqwest::blocking\|get_url\b"` on lapce-app/, lapce-proxy/]

| # | File | Lines | What Downloaded | Return Usage | Migration Action |
|---|------|-------|-----------------|--------------|-----------------|
| 1 | `lapce-app/src/update.rs` | 33 | GitHub API JSON (release info) | `.text()?.into json parse` | `block_on(get_url_async(...))?.text()` |
| 2 | `lapce-app/src/update.rs` | 75 | Self-update archive (.dmg/.tar.gz/.msi) | `.copy_to(&mut out)?` | `bytes().then std::io::copy` |
| 3 | `lapce-app/src/plugin.rs` | 298 | Plugin info JSON from `plugins.lapce.dev` | `.json().ok()` | `block_on(...)?.json()` |
| 4 | `lapce-app/src/plugin.rs` | 433 | Plugin icon bytes | `.bytes()?.to_vec()` | `block_on(...)?.bytes()` |
| 5 | `lapce-app/src/plugin.rs` | 460 | Plugin README markdown text | `.text()?` | `block_on(...)?.text()` |
| 6 | `lapce-app/src/plugin.rs` | 474 | Plugin list JSON | `.json()?` (VoltsInfo) | `block_on(...)?.json()` |
| 7 | `lapce-app/src/app/grammars.rs` | 14 | GitHub API JSON (grammar releases) | `.text()?` | `block_on(...)?.text()` |
| 8 | `lapce-app/src/app/grammars.rs` | 113 | Tree-sitter grammar archive | `.copy_to(file)?` | `bytes() then std::io::copy` |
| 9 | `lapce-app/src/proxy/remote.rs` | 353 | Remote proxy binary (.gz) | `.copy_to + GzDecoder` | `block_on(...)?.bytes() + Cursor + GzDecoder` |
| 10 | `lapce-proxy/src/plugin/mod.rs` | 1561 | Plugin download redirect URL | `.text()?` (S3 URL) | `block_on(...)?.text()` |
| 11 | `lapce-proxy/src/plugin/mod.rs` | 1569 | Plugin archive from S3 | `zstd::Decoder::new(&mut resp)` | `bytes() → Cursor → zstd::Decoder::new` |

**Total: 11 call sites across 5 files.** All route through `lapce_proxy::get_url`.

### The Central Keystone: `lapce-proxy/src/lib.rs:196–222`

```rust
// CURRENT (blocking)
pub fn get_url<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::blocking::Response> {
    let mut builder = if let Ok(proxy) = std::env::var("https_proxy") {
        let proxy = reqwest::Proxy::all(proxy)?;
        reqwest::blocking::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(10))
    } else {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
    };
    if let Some(user_agent) = user_agent {
        builder = builder.user_agent(user_agent);
    }
    let client = builder.build()?;
    let mut try_time = 0;
    loop {
        let rs = client.get(url.clone()).send();
        if rs.is_ok() || try_time > 3 {
            return Ok(rs?);
        } else {
            try_time += 1;
        }
    }
}
```

**Async replacement signature (D-02, D-03):**

```rust
// Target: async core, same semantics preserved (10s timeout, up-to-3 retries)
pub async fn get_url_async<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> { ... }

// Sync shim for call sites (D-03): obtain ambient handle, block_on
pub fn get_url<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    tokio::runtime::Handle::current().block_on(get_url_async(url, user_agent))
}
```

[ASSUMED] — the exact sync shim pattern is one valid approach; the planner may choose to inline `block_on` at call sites or keep it as a wrapper. Both satisfy D-03.

### reqwest 0.12 Async API Differences vs Blocking

[VERIFIED: Cargo.lock confirms reqwest 0.12.28 with tokio/hyper-1.x backend]

| Blocking Method | Async Replacement | Notes |
|----------------|------------------|-------|
| `reqwest::blocking::Client::builder()` | `reqwest::Client::builder()` | Async client is the default export |
| `resp.text()?` | `resp.text().await?` (inside `block_on`) | Same API name |
| `resp.json()?` | `resp.json().await?` | Same API name |
| `resp.bytes()?` | `resp.bytes().await?` | Same API name |
| `resp.copy_to(&mut writer)?` | No direct equivalent — use `resp.bytes().await?` + `std::io::copy(&mut Cursor::new(bytes), &mut out)?` | `copy_to` is blocking-only |
| `resp.status()` | `resp.status()` | Sync method on both |
| `reqwest::blocking::Client::builder().proxy(...)` | `reqwest::Client::builder().proxy(...)` | Same API |

**Critical:** Removing the `"blocking"` feature from the workspace `reqwest` entry disables `reqwest::blocking::*` entirely. This MUST happen in the same commit as wiring all call sites. [VERIFIED: STATE.md Critical Pitfall #1; confirmed by reqwest feature gate architecture]

---

## Crash Site Inventory

### CRASH-01 — Keybinding Condition Evaluation

[VERIFIED: source read of `lapce-app/src/keypress/condition.rs` and `lapce-app/src/keypress.rs:524–561`]

**Status:** The REQUIREMENTS.md audit refs `condition.rs:95,104,108` are stale. The current codebase has already refactored the panic. The `check_condition` implementation at `keypress.rs:524` is panic-free:

- `check_one_condition` uses `Condition::from_str(trimmed)` — returns `Ok`/`Err` (strum `EnumString`), no `.unwrap()`
- Unknown positive token → `false` (falls through the `else` in the inner helper)
- Unknown negated token `!unknown` → `true`
- `CheckCondition::parse_first` slices only at ASCII `||`/`&&` byte offsets — always valid UTF-8 boundaries
- Existing `#[cfg(test)]` module in `condition.rs` already has `test_check_condition` covering compound AND/OR/NOT with a `MockFocus`

**Phase work:**
- CRASH-01 regression test: new test asserting that an **unknown condition string** evaluates to `false`, and `!unknown` evaluates to `true`, locking the non-panic guarantee. Can be added to the existing `#[cfg(test)] mod test` in `condition.rs`.
- Load-time warning (D-10): add `tracing::warn!` in `KeyMapLoader::load_from_str` when a keymap's `when` field contains a token that fails `Condition::from_str`. Currently `when` is stored as a raw `String` without validation — the loader's `get_keymap` function at `loader.rs:90–120` builds a `KeyMap { when: Option<String>, ... }` without parsing `when`. The warning should be emitted at load time after the `KeyMap` is constructed, checking each `when` token via `Condition::from_str`.

### CRASH-02 — Git No-Workspace Panic

[VERIFIED: `dispatch.rs:1343` confirmed as `self.workspace.clone().unwrap()` in `handle_workspace_fs_event`]

**Location:** `lapce-proxy/src/dispatch.rs:1343`

```rust
// CURRENT — panics when workspace is None (editor opened with no folder)
let workspace = self.workspace.clone().unwrap();
```

**Context:** `handle_workspace_fs_event` is a **background filesystem-event handler** that spawns a git-diff polling thread. It is called by the `notify` file watcher, not by user action. `self.workspace` is `Option<PathBuf>` (confirmed at `dispatch.rs:56`).

**Fix (D-06):** Guard with early return:

```rust
let Some(workspace) = self.workspace.clone() else {
    return;
};
```

No user notification from this path (spurious UX for background handler).

### CRASH-05 — Swallowed Git Errors (co-located with CRASH-02)

[VERIFIED: `dispatch.rs:354–388` confirmed, eprintln lines at 358, 369, 377, 385]

**Location:** `lapce-proxy/src/dispatch.rs:354–388` — user-triggered git command arms: `GitCheckout` (354), `GitDiscardFilesChanges` (362), `GitDiscardWorkspaceChanges` (373), `GitInit` (381).

**Current pattern (all four arms identical):**
```rust
GitCheckout { reference } => {
    if let Some(workspace) = self.workspace.as_ref() {
        match git_checkout(workspace, &reference) {
            Ok(()) => (),
            Err(e) => eprintln!("{e:?}"),  // CRASH-05: swallowed
        }
    }
    // No else: silent when workspace is None — D-07 adds else here
}
```

**Fix (D-07 + D-08):** Add else-branch for D-07, replace `eprintln!` for D-08:
```rust
GitCheckout { reference } => {
    if let Some(workspace) = self.workspace.as_ref() {
        match git_checkout(workspace, &reference) {
            Ok(()) => (),
            Err(e) => self.core_rpc.show_message(
                "Git Checkout failed".to_owned(),
                ShowMessageParams { typ: MessageType::ERROR, message: e.to_string() },
            ),
        }
    } else {
        self.core_rpc.show_message(
            "Git operation failed".to_owned(),
            ShowMessageParams { typ: MessageType::ERROR, message: "No folder open".to_owned() },
        );
    }
}
```

Note: `self.core_rpc.show_message(title, params)` is already used at `dispatch.rs:343–350` for `GitCommit` — identical pattern to follow. [VERIFIED: dispatch.rs:338–352]

### CRASH-03 — DAP Stdio Capture Panic

[VERIFIED: `lapce-proxy/src/plugin/dap.rs:104,105`]

**Location:** `lapce-proxy/src/plugin/dap.rs:104–105`

```rust
// In start_process(&self) -> Result<()>
let stdin = process.stdin.take().unwrap();   // line 104 — panics if stdin not captured
let stdout = process.stdout.take().unwrap(); // line 105 — panics if stdout not captured
```

**Context:** `start_process` is called from `DapClient::start` (public `fn start(...) -> Result<DapRpcHandler>`). The `Result` return type already propagates upward through the call chain to `catalog.rs` where it is currently only `tracing::error!`'d (catalog.rs:~625). The error does NOT reach the UI today.

**Fix (D-13):** Replace `.unwrap()` with `.ok_or_else(|| anyhow!(...))`:
```rust
let stdin = process.stdin.take()
    .ok_or_else(|| anyhow!("failed to capture DAP stdin"))?;
let stdout = process.stdout.take()
    .ok_or_else(|| anyhow!("failed to capture DAP stdout"))?;
```

**UI propagation:** The error propagates to `DapClient::start` → `catalog.rs DapStart` arm. That arm currently logs `tracing::error!("{:?}", err)`. To satisfy TEST-01 (error reaches UI as notification), add `self.plugin_rpc.core_rpc.show_message(...)` in the `Err` branch at `catalog.rs`. The `show_message` pattern is already used at `catalog.rs:~633` for "Debugger not found". [VERIFIED: catalog.rs:625–638]

### CRASH-04 — Malformed Zstd Plugin Archive Panic

[VERIFIED: `lapce-proxy/src/plugin/mod.rs:1589–1592`]

**Location:** `lapce-proxy/src/plugin/mod.rs:1590`

```rust
// In download_volt(), inside if is_zstd branch
let tar = zstd::Decoder::new(&mut resp).unwrap(); // panics on corrupt/malformed archive
let mut archive = Archive::new(tar);
archive.unpack(&plugin_dir)?;
```

**Context (post-async-migration):** After D-03 migration, `resp` is no longer a `reqwest::blocking::Response` directly. `get_url` (via `block_on`) returns `reqwest::Response`. The zstd `Decoder::new` takes `impl Read`. The async `reqwest::Response` does NOT implement `Read`. After async migration, the pattern must be:

```rust
let body = resp.bytes()?; // block_on wrapped; returns Bytes
let cursor = std::io::Cursor::new(body);
let tar = zstd::Decoder::new(cursor)  // cursor: impl Read
    .map_err(|e| anyhow!("malformed zstd plugin archive: {e}"))?;
```

**Fix (D-12):** Replace `.unwrap()` with `?` (or `.map_err(...)?` for a clear message).

**UI propagation:** `download_volt` returns `Result<VoltMetadata>`. It is called from `install_volt` at `plugin/mod.rs:1609`. `install_volt` already calls `catalog_rpc.core_rpc.volt_installing(volt, "Could not download Plugin".to_string())` when `download_volt_result.is_err()` (line ~1611). So the `?` propagation from a zstd parse failure will surface via the existing `VoltInstalling { error }` path. No additional UI wiring needed — the error already reaches the user through the plugin-install feedback channel. [VERIFIED: mod.rs:1609–1620]

---

## Architecture Patterns

### System Architecture Diagram

```
lapce-app (UI process)
  ├── update.rs            ──block_on──► DownloadPipeline (download.rs)
  ├── plugin.rs            ──block_on──►      │
  ├── app/grammars.rs      ──block_on──►      │
  └── proxy/remote.rs      ──block_on──►      │
                                              │ wraps
                                              ▼
lapce-proxy (proxy process, separate binary)
  └── lib.rs::get_url_async()  ◄──────────────┘
        │  (async reqwest::Client)
        │  preserves: 10s timeout, 3-retry loop, https_proxy env handling
        ▼
  reqwest::Client (async, tokio-backed)
        │
        ▼
  reqwest::Response  (async)
        │
        ├── .text().await?    →  String
        ├── .json().await?    →  T: Deserialize
        └── .bytes().await?   →  Bytes
              │
              └── std::io::Cursor::new(bytes) → impl Read
                    ├── zstd::Decoder (CRASH-04 fix)
                    └── GzDecoder (proxy binary download)

lapce-rpc (shared types)
  └── CoreNotification::ShowMessage
        │
        └── CoreRpcHandler::show_message(title, ShowMessageParams)
              │  (crossbeam_channel tx)
              ▼
        lapce-app window_tab.rs::handle_core_notification()
              ▼
        messages: RwSignal<Vec<(String, ShowMessageParams)>> (rendered in app.rs:2842)
```

### Recommended Project Structure

```
lapce-app/src/
├── download.rs            # NEW — DownloadPipeline thin wrapper (RT-03)
├── update.rs              # CHG — get_url → block_on(get_url_async)
├── plugin.rs              # CHG — get_url → block_on(get_url_async)
├── app/grammars.rs        # CHG — get_url → block_on(get_url_async)
├── proxy/remote.rs        # CHG — get_url → block_on(get_url_async), remove .expect
├── keypress/condition.rs  # CHG — add CRASH-01 regression test
└── keypress/loader.rs     # CHG — add tracing::warn! for unparseable condition tokens

lapce-proxy/src/
├── lib.rs                 # CHG — replace get_url (blocking) with get_url_async (async) + block_on shim
├── dispatch.rs            # CHG — CRASH-02 guard at 1343, CRASH-05 eprintln→ShowMessage 358/369/377/385, D-07 else-branch
└── plugin/
    ├── mod.rs             # CHG — CRASH-04 zstd unwrap→?, async resp bytes pattern
    ├── dap.rs             # CHG — CRASH-03 stdin/stdout unwrap→ok_or?
    └── catalog.rs         # CHG — DAP start error → show_message (TEST-01 seam)

Cargo.toml (workspace)    # CHG — remove "blocking" from reqwest features
```

### Pattern 1: block_on Bridge at Sync Call Sites (D-03)

[ASSUMED] — based on tokio docs; exact pattern consistent with Phase 2 ambient runtime design.

```rust
// In a synchronous call site (background thread or Floem UI thread):
// No signature change needed — the function stays sync.
fn download_something(url: &str) -> Result<String> {
    let resp = lapce_proxy::get_url(url, None)?;  // same call signature
    Ok(resp.text()?)
}

// In lapce-proxy/src/lib.rs — the sync shim wraps async core:
pub fn get_url<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    tokio::runtime::Handle::current()
        .block_on(get_url_async(url, user_agent))
}
```

### Pattern 2: Bytes-then-Cursor for Streaming-to-Reader

The async `reqwest::Response` does not implement `std::io::Read`. For code that needs a `Read` impl (zstd decoder, GzDecoder, `std::io::copy`), download the full body into memory first:

```rust
// After async migration: resp is reqwest::Response
let body_bytes = resp.bytes()?;  // via block_on — Bytes
let mut cursor = std::io::Cursor::new(body_bytes);
// cursor implements Read + Seek
let tar = zstd::Decoder::new(&mut cursor)
    .map_err(|e| anyhow!("malformed archive: {e}"))?;
```

For large file downloads (update archive, grammar archive) where streaming is preferable, the pattern is identical — we accept full-body buffering in this phase (D-03: minimal-invasive).

[ASSUMED] — this is the standard approach; streaming write via `tokio::io` would require signature changes violating D-03.

### Pattern 3: Regression Test Seam (CoreRpcHandler::rx())

`CoreRpcHandler` is `Clone` and has a public `rx()` method returning `&Receiver<CoreRpc>`. This is the test seam for asserting notification emission:

```rust
// In a test: construct a real CoreRpcHandler, pass it to the code under test,
// then recv() to assert the notification was emitted.
#[test]
fn crash_fix_surfaces_notification() {
    let core_rpc = CoreRpcHandler::new();
    // trigger the failure path with the core_rpc injected
    // ...
    let msg = core_rpc.rx().recv_timeout(Duration::from_millis(100))
        .expect("expected notification");
    match msg {
        CoreRpc::Notification(n) => match *n {
            CoreNotification::ShowMessage { message, .. } => {
                assert_eq!(message.typ, MessageType::ERROR);
            }
            _ => panic!("wrong notification type"),
        },
        _ => panic!("expected notification"),
    }
}
```

[VERIFIED: `lapce-rpc/src/core.rs:167–186` — `CoreRpcHandler::new()`, `rx()`, `CoreRpc` enum variants]

### Anti-Patterns to Avoid

- **`.unwrap()` → `?` discarded at thread boundary:** Replacing `.unwrap()` with `?` inside a `thread::spawn(move || { ... })` closure that returns `()` silently swallows the error. Always ensure `?` propagates to a handler that calls `core_rpc.show_message(...)`.
- **Second `tokio::Runtime::new()` per download:** Violates the "one runtime per binary" constraint (STATE.md Architecture Constraints). Always `Handle::current().block_on(...)`.
- **`async`/`.await` pushed up through call chains:** D-03 explicitly forbids this. Keep all call sites sync; only `get_url_async` + its `block_on` shim are async.
- **Keeping `reqwest::blocking` in Cargo.toml after wiring async:** The blocking calls will panic at runtime (STATE.md Critical Pitfall #1). Feature must be dropped atomically with wiring.
- **`resp.copy_to(&mut writer)` in async context:** `copy_to` is a method on `reqwest::blocking::Response` only. It will not compile after removing the `blocking` feature. Use `resp.bytes()?` + `std::io::copy`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Retry loop for HTTP | Custom retry state machine | Preserve existing `try_time` loop in `get_url_async` (3 retries, identical logic) | Already tested semantics; Phase 4 will extend this exact function |
| Timeout enforcement | Manual `tokio::time::timeout` wrapper | `reqwest::Client::builder().timeout(Duration::from_secs(10))` | Client-level timeout applies to entire request including connection |
| Notification channel | Custom RPC abstraction | `CoreRpcHandler::show_message(title, ShowMessageParams)` | Already wired, already rendered in UI (app.rs:2842) |
| Test mock for RPC | Hand-rolled mock struct | `CoreRpcHandler::new()` + `.rx()` receiver | Real handler; crossbeam channel; instant, deterministic assertion |
| `Read` impl on async response | Async→sync adapter trait | `resp.bytes()? → std::io::Cursor::new(bytes)` | Cursor is `Read`; no unsafe, no extra deps |

---

## Common Pitfalls

### Pitfall 1: Blocking Inside Active Runtime

**What goes wrong:** `reqwest::blocking` uses its own internal tokio runtime. Calling `reqwest::blocking::Client::send()` inside an active `tokio::Runtime` context panics with "Cannot start a runtime from within a runtime".

**Why it happens:** The Phase-2 `rt.enter()` guard makes a multi-thread runtime ambient everywhere in the process. `reqwest::blocking` tries to create a single-thread runtime internally — nested runtime creation panics.

**How to avoid:** The `blocking` feature must be REMOVED from workspace `reqwest` in the same commit batch that wires all 11 call sites to the async `get_url_async`. No interim state where async runtime is active but blocking calls remain.

**Warning signs:** `thread 'lapce-app-worker' panicked at 'Cannot start a runtime from within a runtime'` — this is the exact panic if partial migration is attempted.

### Pitfall 2: `reqwest::Response` Does Not Implement `Read`

**What goes wrong:** Code that calls `zstd::Decoder::new(&mut resp)` or `GzDecoder::new(&mut resp)` will fail to compile after switching from `blocking::Response` (which wraps a `std::io::Read` impl) to async `reqwest::Response` (which does not).

**Why it happens:** `reqwest::blocking::Response` wraps a synchronous body; async `reqwest::Response` is stream-based.

**How to avoid:** For every call site that needs a `Read` impl: `resp.bytes()?` → `std::io::Cursor::new(bytes)`. The cursor implements `Read`. The full-body buffering is acceptable for this phase (archives are typically < 50MB; the current blocking code also buffered fully via `copy_to`).

### Pitfall 3: `?` Discarded at Thread Boundary

**What goes wrong:** Replacing `unwrap()` with `?` inside `thread::spawn(|| ...)` closures that return `()` silently drops the error.

**Why it happens:** The current CRASH-03 / CRASH-05 sites are inside thread closures. `?` returns from the closure, not the outer function, and `thread::JoinHandle::join()` is never called.

**How to avoid:** After replacing `unwrap()` with `?`, ensure the failure path calls `core_rpc.show_message(...)` in the `Err` arm OR ensure the closure returns `Result<()>` and a `JoinHandle::join()` propagates it — but the former is the pattern already established in the codebase (catalog.rs:621–638).

### Pitfall 4: `copy_to` Removal Breaks Copy Pattern

**What goes wrong:** `resp.copy_to(&mut out)?` at `update.rs:80`, `grammars.rs:123` is a convenience method on `blocking::Response`. After removing `blocking`, these lines will not compile.

**How to avoid:** Replace with:
```rust
let bytes = resp.bytes()?;
std::io::copy(&mut std::io::Cursor::new(bytes), &mut out)?;
```

For `grammars.rs:123`, the file handle `file` is a `tempfile::tempfile()` — same approach applies.

### Pitfall 5: Root Crate `Cargo.toml` Needs Updating

**What goes wrong:** The root `Cargo.toml` gained `reqwest` as a direct dep for its `[[bin]]` targets in Phase 1 (confirmed in Phase 2 SUMMARY deviations: root crate needed tokio + tracing). If `lapce-app/src/bin/lapce.rs` or the root crate binary references `reqwest`, the root Cargo.toml must also drop the `blocking` feature.

**Why it happens:** Root crate `[[bin]]` targets resolve deps against root `[dependencies]`, not just `lapce-app`'s Cargo.toml.

**How to avoid:** Verify with `grep -rn "reqwest" lapce-app/src/bin/lapce.rs lapce-proxy/src/bin/lapce-proxy.rs` — if present, root Cargo.toml change is needed. Currently the entry-point files do NOT import reqwest directly (confirmed: `grep -n "reqwest" lapce-app/src/bin/lapce.rs lapce-proxy/src/bin/lapce-proxy.rs` returns nothing). The `blocking` feature removal needs to happen in workspace `Cargo.toml` only.

---

## Regression Test Strategy (TEST-01)

### Test Seam: `CoreRpcHandler::new()` + `rx()`

[VERIFIED: `lapce-rpc/src/core.rs:167–198`]

`CoreRpcHandler::new()` creates a handler with an internal `crossbeam_channel::unbounded()`. The `rx()` method returns `&Receiver<CoreRpc>`. Any code that calls `show_message(...)` on a `CoreRpcHandler` will send a `CoreRpc::Notification(Box<CoreNotification::ShowMessage {...}>)` that is immediately readable from `rx()`.

This is a zero-mock test seam: no trait objects, no custom mock structs. All five crash fixes can use the same pattern.

### Test Pattern for Proxy-Side Crash Fixes

```rust
#[cfg(test)]
mod crash_fix_tests {
    use std::time::Duration;
    use lapce_rpc::{core::{CoreNotification, CoreRpc, CoreRpcHandler}};
    use lsp_types::MessageType;

    #[test]
    fn crash_03_dap_stdio_failure_surfaces_notification() {
        // Construct a handler with no real stdout/stdin capture possible
        // (e.g., spawn a process with stdio not piped, then call start_process)
        let core_rpc = CoreRpcHandler::new();
        // ... trigger the failure path ...
        let msg = core_rpc.rx()
            .recv_timeout(Duration::from_millis(100))
            .expect("ShowMessage notification not emitted");
        let CoreRpc::Notification(n) = msg else { panic!("expected notification") };
        let CoreNotification::ShowMessage { message, .. } = *n else {
            panic!("expected ShowMessage, got {:?}", n)
        };
        assert_eq!(message.typ, MessageType::ERROR);
    }
}
```

### Per-Fix Test Plan

| Fix | Test Location | Trigger Mechanism | Assertion |
|-----|--------------|-------------------|-----------|
| CRASH-01 (regression) | `lapce-app/src/keypress/condition.rs` existing `#[cfg(test)] mod test` | `KeyPressData::check_condition("unknown_token", &focus)` | Returns `false` (no panic) |
| CRASH-01 negated | Same | `KeyPressData::check_condition("!unknown_token", &focus)` | Returns `true` (no panic) |
| CRASH-02 | `lapce-proxy/src/dispatch.rs` new `#[cfg(test)]` | Call `handle_workspace_fs_event` with `workspace: None` | No panic; no notification (early return) |
| CRASH-03 | `lapce-proxy/src/plugin/catalog.rs` new `#[cfg(test)]` | `DapClient::start` with program that fails process::Command (bad program path) | `core_rpc.rx()` yields `ShowMessage { typ: ERROR }` |
| CRASH-04 | `lapce-proxy/src/plugin/mod.rs` new `#[cfg(test)]` | `download_volt` with a mock that returns corrupted bytes | Error propagates as `Err(...)` — checked via `assert!(result.is_err())` + existing `VoltInstalling` path |
| CRASH-05 | `lapce-proxy/src/dispatch.rs` new `#[cfg(test)]` | Call git command arms with workspace set to a non-git path (triggers `git_checkout` error) | `core_rpc.rx()` yields `ShowMessage { typ: ERROR }` |

**Precedent in codebase:** `lapce-app/src/runtime_tests.rs` (Phase 2) demonstrates the pattern: construct a self-contained runtime, run the test, assert. The `CoreRpcHandler::new()` pattern is the same design.

---

## State of the Art

| Old Approach | Current Approach | Impact for Phase 3 |
|--------------|------------------|-------------------|
| `reqwest` 0.11 blocking-only API | `reqwest` 0.12 async-first (blocking is an optional feature) | Phase 1 already upgraded to 0.12.28; `blocking` feature still present, must be removed |
| `reqwest::blocking::Response.copy_to()` | `reqwest::Response.bytes().await?` + `std::io::copy` | All 3 streaming download call sites need this rewrite |
| No tokio runtime in Lapce binaries | Ambient tokio from Phase 2 `rt.enter()` guard | `Handle::current()` resolves everywhere; `block_on` is safe |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `block_on` sync shim as a new `get_url` that wraps `get_url_async` is the simplest migration pattern | §Standard Stack, §Pattern 1 | Planner may choose to inline `block_on` at each call site; both are valid per D-03 |
| A2 | Full-body buffering via `resp.bytes()?` is acceptable for streaming download sites (no streaming write) | §Pattern 2, §Pitfall 4 | Acceptable for this phase (D-03: minimal-invasive); Phase 4/5 may optimize |
| A3 | Root crate `Cargo.toml` does not need changes for reqwest feature removal | §Pitfall 5 | If entry-point .rs files gain a reqwest import, root crate needs the feature dropped too — verify before commit |

**If table is empty:** All claims verified. Only A1-A3 are assumptions — all three are low-risk planning choices, not factual unknowns.

---

## Open Questions

1. **`copy_to` replacement: in-memory buffering vs. chunk streaming**
   - What we know: `resp.bytes()` loads the full body; archives can be 20–100MB
   - What's unclear: Is full-body buffering acceptable for the self-update archive (potentially 100MB+)?
   - Recommendation: Use `bytes()` for Phase 3 (D-03 minimal-invasive); if memory pressure is a concern, note for Phase 5 optimization.

2. **CRASH-02 test: how to trigger `handle_workspace_fs_event` in isolation**
   - What we know: `Dispatcher` is a large struct; constructing a minimal test instance may require many fields
   - What's unclear: Whether a unit test can construct `Dispatcher` with `workspace: None` cheaply
   - Recommendation: Write the test as a focused unit test on the fixed code path (`let Some(workspace) = self.workspace.clone() else { return; }`) — the logic is trivially verifiable without a full Dispatcher instance. Alternatively use an integration test if construction is too expensive.

3. **CRASH-04 test: mocking `download_volt` to return corrupt bytes**
   - What we know: `download_volt` calls `get_url` (which will be `block_on(get_url_async(...))`); there is no injection point
   - What's unclear: Whether to test at the `zstd::Decoder::new(cursor)?` level directly (pass a corrupt `Cursor`) or end-to-end
   - Recommendation: Test the `?` conversion directly: `zstd::Decoder::new(std::io::Cursor::new(b"not zstd data")).map_err(|e| anyhow!("{e}"))` — verify `is_err()`. The regression value is confirming the `.unwrap()` is gone; full end-to-end test belongs in Phase 4.

---

## Environment Availability

> Step 2.6: All work is codebase edits + `cargo build/test`. No external services, tools, or databases needed beyond the Rust toolchain already confirmed in Phase 2.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All | ✓ | 1.87+ (from CLAUDE.md) | — |
| Cargo | Build | ✓ | workspace | — |
| zstd crate | CRASH-04 fix | ✓ | 0.11.2 (Cargo.lock) | — |
| reqwest 0.12 | Migration | ✓ | 0.12.28 (Cargo.lock) | — |
| tokio | Runtime | ✓ | 1.52.3 (workspace) | — |

---

## Validation Architecture

> `nyquist_validation: false` in `.planning/config.json` — this section is SKIPPED per workflow config.

---

## Security Domain

> CRASH-04 note: replacing `.unwrap()` with `?` on `zstd::Decoder::new` prevents a crash but does NOT add path-traversal protection (`archive.unpack(&plugin_dir)?` remains). The Phase 3 fix is strictly "stop panicking"; path-traversal hardening is SEC-04 / Phase 4 scope per CONTEXT.md deferred section.

### Applicable ASVS Categories

| ASVS Category | Applies | Phase 3 Standard Control |
|---------------|---------|--------------------------|
| V5 Input Validation | yes (malformed archive) | `zstd::Decoder::new(cursor)?` — reject corrupt input, return error |
| V6 Cryptography | no | Phase 4 adds SHA256; Phase 3 only stops panic |
| V2/V3 Auth/Session | no | Not in scope |
| V4 Access Control | no | Not in scope |

---

## Sources

### Primary (HIGH confidence — all from direct source code inspection)

- `lapce-proxy/src/lib.rs:196–222` — `get_url` blocking implementation (complete function read)
- `lapce-proxy/src/dispatch.rs:354–388, 1343` — CRASH-02/05 sites (direct read)
- `lapce-proxy/src/plugin/dap.rs:97–106` — CRASH-03 site (direct read)
- `lapce-proxy/src/plugin/mod.rs:1555–1620` — CRASH-04 site + `install_volt` error flow (direct read)
- `lapce-app/src/keypress/condition.rs` — full file read; confirmed panic-free
- `lapce-app/src/keypress.rs:524–561` — `check_condition` implementation (direct read)
- `lapce-rpc/src/core.rs:46–99, 167–198` — `CoreNotification`, `CoreRpcHandler::new/rx/show_message` (direct read)
- `Cargo.toml` (workspace) — reqwest 0.12.28, tokio 1.52.3, features confirmed
- `Cargo.lock` — reqwest 0.12.28 dependency graph confirmed (tokio/hyper-1.x backend)
- `.planning/phases/02-*/02-01-SUMMARY.md`, `02-02-SUMMARY.md` — Phase 2 deliverables (ambient runtime + regression test pattern)
- `.planning/phases/03-*/03-CONTEXT.md` — all locked decisions

### Secondary (MEDIUM confidence)

- reqwest 0.12 API inference from Cargo.lock dependency graph (tokio/hyper-1.x, no sync adapter) — standard Rust async HTTP knowledge

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages verified in Cargo.toml/Cargo.lock
- Architecture: HIGH — all patterns verified against source code
- Crash sites: HIGH — all file:line confirmed by direct source read
- Test strategy: HIGH — CoreRpcHandler seam verified; individual test mechanics are ASSUMED reasonable

**Research date:** 2026-06-08
**Valid until:** 2026-07-08 (stable codebase; no fast-moving deps)
