# Phase 3: Download Pipeline + Crash Fixes - Pattern Map

**Mapped:** 2026-06-08
**Files analyzed:** 12 new/modified files
**Analogs found:** 12 / 12

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `lapce-proxy/src/lib.rs` (`get_url` → async) | network-core | request-response | itself (current blocking implementation lines 196–222) | self-migration |
| `lapce-app/src/download.rs` (NEW) | network-wrapper | request-response | `lapce-app/src/update.rs` (thin call-site wrapper pattern) | role-match |
| `lapce-app/src/update.rs` | call-site | request-response | itself + `lapce-proxy/src/lib.rs` get_url | self-migration |
| `lapce-app/src/plugin.rs` | call-site | request-response | `lapce-app/src/update.rs` | exact |
| `lapce-app/src/app/grammars.rs` | call-site | request-response | `lapce-app/src/update.rs` | exact |
| `lapce-app/src/proxy/remote.rs` | call-site | request-response | `lapce-app/src/update.rs` | exact |
| `lapce-proxy/src/plugin/mod.rs` (CRASH-04 + call-site) | call-site + crash-fix | request-response | itself (lines 1555–1627) + `dispatch.rs` error pattern | self-migration |
| `lapce-proxy/src/dispatch.rs` (CRASH-02/05) | crash-fix | event-driven | itself (lines 338–352 `GitCommit` arm) | exact |
| `lapce-proxy/src/plugin/dap.rs` (CRASH-03) | crash-fix | request-response | `lapce-proxy/src/plugin/catalog.rs` lines 624–637 | exact |
| `lapce-proxy/src/plugin/catalog.rs` (CRASH-03 notification) | crash-fix | request-response | itself (lines 630–637 `show_message` call) | exact |
| `lapce-app/src/keypress/condition.rs` (CRASH-01 test) | test | — | `lapce-app/src/runtime_tests.rs` (Phase 2 regression test) | role-match |
| `lapce-app/src/keypress/loader.rs` (D-10 warn) | crash-fix | — | itself (lines 39–43 `error!` call) | exact |
| `Cargo.toml` (workspace) | config/manifest | — | itself (reqwest entry) | self-edit |

---

## Pattern Assignments

### `lapce-proxy/src/lib.rs` — async `get_url` core (network-core, request-response)

**Analog:** itself, lines 196–222 (blocking implementation to replace)

**Current blocking implementation** (lines 196–222) — the exact function to migrate:
```rust
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

**Target async pattern** (D-02, D-03 — RESEARCH.md §Call Site Inventory):
- Replace `reqwest::blocking::Client::builder()` with `reqwest::Client::builder()`
- Replace `reqwest::blocking::Response` return type with `reqwest::Response`
- Make `get_url_async` async; keep `get_url` as a sync shim via `Handle::current().block_on(...)`
- Preserve: `https_proxy` env handling, 10s timeout, up-to-3 retry loop (semantics locked by D-03 / discretion decision)
- Return type change: `reqwest::Response` (not `reqwest::blocking::Response`)

**Sync shim pattern** (RESEARCH.md §Pattern 1):
```rust
pub fn get_url<T: reqwest::IntoUrl + Clone>(
    url: T,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    tokio::runtime::Handle::current()
        .block_on(get_url_async(url, user_agent))
}
```

**Anti-pattern to avoid:** Never call `tokio::runtime::Builder::new_*().build()` inside `get_url` — only `Handle::current().block_on(...)`. A second runtime panics inside the ambient Phase-2 runtime.

---

### `lapce-app/src/download.rs` (NEW) — DownloadPipeline wrapper (network-wrapper, request-response)

**Analog:** `lapce-app/src/update.rs` lines 1–53 (thin wrapper around `lapce_proxy::get_url`)

**Imports pattern** — copy from `update.rs` lines 1–5:
```rust
use anyhow::{Result, anyhow};
```
Add `lapce_proxy` import for the async core. No second `reqwest::Client` — D-02 forbids it.

**Core pattern** — `DownloadPipeline` is a thin struct or a set of free functions that delegate to `lapce_proxy::get_url(...)`. It provides the named API surface RT-03 requires without duplicating any HTTP logic. Example shape:
```rust
pub struct DownloadPipeline;

impl DownloadPipeline {
    pub fn get(url: impl reqwest::IntoUrl + Clone, user_agent: Option<&str>)
        -> Result<reqwest::Response>
    {
        lapce_proxy::get_url(url, user_agent)
    }
}
```
Exact type/method names are planner's discretion (CONTEXT.md §Claude's Discretion).

**What NOT to add:** No `reqwest::Client` field, no `tokio::Runtime` field, no retry logic — all of that lives in `lapce-proxy/src/lib.rs`.

---

### `lapce-app/src/update.rs` — call-site migration (call-site, request-response)

**Analog:** itself

**Current call-site patterns** (lines 33, 75–80) — the two patterns to transform:

Pattern A — JSON fetch (line 33):
```rust
let resp = lapce_proxy::get_url(url, Some("Lapce"))?;
if !resp.status().is_success() {
    return Err(anyhow!("get release info failed {}", resp.text()?));
}
let mut release: ReleaseInfo = serde_json::from_str(&resp.text()?)?;
```
After migration: `resp.text()?` becomes `resp.text()?` — same call, because `get_url` shim still returns a sync-compatible `reqwest::Response` via `block_on`. No call-site change needed if the shim preserves the return type shape.

Pattern B — streaming copy to file (lines 75–80):
```rust
let mut resp = lapce_proxy::get_url(&asset.browser_download_url, None)?;
// ...
let mut out = std::fs::File::create(&file_path)?;
resp.copy_to(&mut out)?;   // BROKEN after blocking removal
```
**`copy_to` is a `blocking::Response`-only method** — must be replaced (RESEARCH.md §Pitfall 4):
```rust
let resp = lapce_proxy::get_url(&asset.browser_download_url, None)?;
// resp is now reqwest::Response (async, via block_on shim)
let bytes = resp.bytes()?;
std::io::copy(&mut std::io::Cursor::new(bytes), &mut out)?;
```

---

### `lapce-app/src/app/grammars.rs` — call-site migration (call-site, request-response)

**Analog:** `lapce-app/src/update.rs` (same two patterns)

**Call sites** (lines 14, 113):
- Line 14: JSON fetch → same as update.rs Pattern A
- Line 113: streaming copy → same as update.rs Pattern B (`copy_to` → `bytes()` + `std::io::copy`)

The `tempfile::tempfile()` file handle at line 113 is a `std::io::Write` — `std::io::copy` works directly.

---

### `lapce-app/src/proxy/remote.rs` — call-site migration + `.expect` removal (call-site, request-response)

**Analog:** `lapce-app/src/update.rs` Pattern B (streaming copy)

**Call site** (line 353):
```rust
// Current — panics on .expect("request failed")
let mut resp = lapce_proxy::get_url(url, None).expect("request failed");
```
`.expect("request failed")` is a known panic site (RESEARCH.md). Replace with `?` propagation per CONVENTIONS.md (`.expect()` only for programmer errors):
```rust
let resp = lapce_proxy::get_url(url, None)?;
```
Then apply Pattern B bytes-then-Cursor for the `GzDecoder::new(&mut resp)` that follows — `resp` no longer implements `Read` after async migration.

---

### `lapce-proxy/src/plugin/mod.rs` — CRASH-04 + call-site migration (crash-fix, request-response)

**Analog:** itself (lines 1555–1627) + `lapce-proxy/src/dispatch.rs` lines 338–352 for error-surfacing

**Current crash site** (lines 1569–1592):
```rust
let mut resp = crate::get_url(url, None)?;
// ...
if is_zstd {
    let tar = zstd::Decoder::new(&mut resp).unwrap();  // CRASH-04
    let mut archive = Archive::new(tar);
    archive.unpack(&plugin_dir)?;
} else {
    let tar = GzDecoder::new(&mut resp);               // also broken after async
    let mut archive = Archive::new(tar);
    archive.unpack(&plugin_dir)?;
}
```

**Both branches** require Bytes-then-Cursor (RESEARCH.md §Pattern 2 + §Pitfall 2) because async `reqwest::Response` does not implement `Read`:
```rust
let body = resp.bytes()?;
let mut cursor = std::io::Cursor::new(body);
if is_zstd {
    let tar = zstd::Decoder::new(&mut cursor)
        .map_err(|e| anyhow!("malformed zstd plugin archive: {e}"))?;
    let mut archive = Archive::new(tar);
    archive.unpack(&plugin_dir)?;
} else {
    let tar = GzDecoder::new(cursor);
    let mut archive = Archive::new(tar);
    archive.unpack(&plugin_dir)?;
}
```

**Error surfacing:** CRASH-04 propagates `?` up to `install_volt` (line 1609–1613). The existing `VoltInstalling` path already handles this:
```rust
// install_volt (lines 1609–1613) — already wired, no change needed:
let download_volt_result = download_volt(&volt);
if download_volt_result.is_err() {
    catalog_rpc.core_rpc.volt_installing(
        volt, "Could not download Plugin".to_string()
    );
}
```

---

### `lapce-proxy/src/dispatch.rs` — CRASH-02 + CRASH-05 (crash-fix, event-driven)

**Analog:** itself, lines 338–352 (`GitCommit` arm — the already-correct model to copy for CRASH-05)

**Imports already present** (lines 1, 38–42):
```rust
use lsp_types::{
    CancelParams, MessageType, NumberOrString, Position, Range, ShowMessageParams,
    // ...
};
```
`MessageType` and `ShowMessageParams` are already imported — no import changes needed.

**CRASH-02 fix — guard at line 1343:**

Current (panics when workspace is None):
```rust
let workspace = self.workspace.clone().unwrap();
```
Fix (D-06 — early return, no notification):
```rust
let Some(workspace) = self.workspace.clone() else {
    return;
};
```

**CRASH-05 fix — git command arms lines 354–388:**

The `GitCommit` arm at lines 338–352 is the **exact model** to copy for all four arms:
```rust
// ANALOG — GitCommit arm (lines 338–352) — already correct pattern:
GitCommit { message, diffs } => {
    if let Some(workspace) = self.workspace.as_ref() {
        match git_commit(workspace, &message, diffs) {
            Ok(()) => (),
            Err(e) => {
                self.core_rpc.show_message(
                    "Git Commit failure".to_owned(),
                    ShowMessageParams {
                        typ: MessageType::ERROR,
                        message: e.to_string(),
                    },
                );
            }
        }
    }
}
```

Apply this pattern to each of the four broken arms (replacing `eprintln!("{e:?}"`), and add the D-07 else-branch for the no-workspace case:
```rust
GitCheckout { reference } => {
    if let Some(workspace) = self.workspace.as_ref() {
        match git_checkout(workspace, &reference) {
            Ok(()) => (),
            Err(e) => {
                self.core_rpc.show_message(
                    "Git Checkout failure".to_owned(),
                    ShowMessageParams {
                        typ: MessageType::ERROR,
                        message: e.to_string(),
                    },
                );
            }
        }
    } else {
        self.core_rpc.show_message(
            "Git operation failed".to_owned(),
            ShowMessageParams {
                typ: MessageType::ERROR,
                message: "No folder open".to_owned(),
            },
        );
    }
}
// Repeat pattern for GitDiscardFilesChanges, GitDiscardWorkspaceChanges, GitInit
```

---

### `lapce-proxy/src/plugin/dap.rs` — CRASH-03 (crash-fix, request-response)

**Analog:** `lapce-proxy/src/plugin/catalog.rs` lines 624–637 (`show_message` in Err branch)

**Current crash site** (lines 104–105):
```rust
let stdin = process.stdin.take().unwrap();   // panics if stdin not captured
let stdout = process.stdout.take().unwrap(); // panics if stdout not captured
```

**Fix (D-13)** — copy `.ok_or_else(|| anyhow!(...))` pattern from CONVENTIONS.md:
```rust
let stdin = process.stdin.take()
    .ok_or_else(|| anyhow!("failed to capture DAP stdin"))?;
let stdout = process.stdout.take()
    .ok_or_else(|| anyhow!("failed to capture DAP stdout"))?;
```

`?` propagates to `DapClient::start` → `catalog.rs` `DapStart` arm. The error must then reach the UI (TEST-01 / Criterion #6) — see `catalog.rs` section below.

---

### `lapce-proxy/src/plugin/catalog.rs` — CRASH-03 notification seam (crash-fix, request-response)

**Analog:** itself, lines 630–637 (existing `show_message` call for "Debugger not found")

**Existing correct pattern** (lines 630–637):
```rust
self.plugin_rpc.core_rpc.show_message(
    "debug fail".to_owned(),
    ShowMessageParams {
        typ: MessageType::ERROR,
        message: "Debugger not found. Please install the appropriate plugin."
            .to_owned(),
    },
)
```

In the `DapStart` arm, the `Err(err)` branch at line ~624 currently only logs:
```rust
Err(err) => {
    tracing::error!("{:?}", err);
}
```
Add `show_message` after (or instead of) the `tracing::error!` call, using the existing pattern above:
```rust
Err(err) => {
    tracing::error!("{:?}", err);
    self.plugin_rpc.core_rpc.show_message(
        "DAP start failure".to_owned(),
        ShowMessageParams {
            typ: MessageType::ERROR,
            message: err.to_string(),
        },
    );
}
```

---

### `lapce-app/src/keypress/condition.rs` — CRASH-01 regression test (test)

**Analog:** `lapce-app/src/runtime_tests.rs` (Phase 2 self-contained regression test pattern) + existing `#[cfg(test)] mod test` in `condition.rs` lines 76–162

**Existing test structure** (lines 76–162) — add new cases to `test_check_condition` or new test functions in the same `mod test` block:

```rust
// Existing test block to extend (condition.rs lines 76–162):
#[cfg(test)]
mod test {
    use super::Condition;
    use crate::keypress::{KeyPressData, KeyPressFocus, condition::CheckCondition};

    // MockFocus already defined at lines 84–110

    #[test]
    fn test_check_condition() {
        // ... existing cases ...
    }

    // NEW — CRASH-01 regression test (D-11 contract):
    #[test]
    fn unknown_condition_evaluates_to_false_not_panic() {
        let focus = MockFocus { accepted_conditions: &[] };
        // Unknown positive token → false (binding skipped)
        assert!(
            !KeyPressData::check_condition("totally_unknown_condition_xyz", &focus),
            "unknown condition should evaluate to false, not panic"
        );
    }

    #[test]
    fn negated_unknown_condition_evaluates_to_true_not_panic() {
        let focus = MockFocus { accepted_conditions: &[] };
        // Unknown negated token → true (permissive, per D-11)
        assert!(
            KeyPressData::check_condition("!totally_unknown_condition_xyz", &focus),
            "negated unknown condition should evaluate to true, not panic"
        );
    }
}
```

**Key design constraint:** `MockFocus` is already defined in the module — reuse it. Do not add a second mock struct.

---

### `lapce-app/src/keypress/loader.rs` — D-10 load-time warn (crash-fix)

**Analog:** itself, lines 39–43 (existing `error!` call on unparseable keymap)

**Current error handling at lines 39–43:**
```rust
Err(err) => {
    error!("Could not parse keymap: {err}");
    continue;
}
```
`tracing` is already imported as `use tracing::{debug, error};` (line 4).

**D-10 addition:** After `KeyMap` is constructed (line 114–126), validate the `when` field tokens via `Condition::from_str` and emit `tracing::warn!` for any token that fails. Add `warn` to the existing tracing import and emit after the `KeyMap` is built:
```rust
// In load_from_str, after successful keymap construction:
if let Some(ref when) = keymap.when {
    // Split on || and && to check individual tokens
    for token in when.split(|c| c == '|' || c == '&')
        .map(|t| t.trim().trim_start_matches('!'))
        .filter(|t| !t.is_empty())
    {
        if token.parse::<Condition>().is_err() {
            tracing::warn!(
                "Unparseable condition token {:?} in keymap {:?}",
                token, keymap.command
            );
        }
    }
}
```
Import: change `use tracing::{debug, error};` to `use tracing::{debug, error, warn};` — or use `tracing::warn!` qualified (either is idiomatic per CONVENTIONS.md).

---

### `Cargo.toml` (workspace) — drop `blocking` feature (config/manifest)

**Analog:** itself (reqwest entry)

**Change:** Remove `"blocking"` from the `reqwest` features list. This must be in the **same commit** as wiring all 11 call sites to the async shim (RESEARCH.md §Pitfall 1 — removing `blocking` while any `reqwest::blocking::*` call site remains causes a runtime panic).

The exact current entry to locate and modify:
```toml
# Find: reqwest entry in [workspace.dependencies]
# Remove "blocking" from the features array
reqwest = { version = "...", features = [..., "blocking", ...] }
# After:
reqwest = { version = "...", features = [...] }  # "blocking" removed
```

---

## Shared Patterns

### Error Notification to UI (`CoreRpcHandler::show_message`)

**Source:** `lapce-rpc/src/core.rs` lines 336–338 + `lapce-proxy/src/dispatch.rs` lines 343–350

**Apply to:** All proxy-side crash fixes (CRASH-03, CRASH-05, D-07) that surface errors as user notifications.

```rust
// CoreRpcHandler::show_message — the single notification method (core.rs:336–338):
pub fn show_message(&self, title: String, message: ShowMessageParams) {
    self.notification(CoreNotification::ShowMessage { title, message });
}

// Call pattern (dispatch.rs:343–350 — GitCommit arm, the reference model):
self.core_rpc.show_message(
    "Git Commit failure".to_owned(),
    ShowMessageParams {
        typ: MessageType::ERROR,
        message: e.to_string(),
    },
);
```

Required imports (already present in `dispatch.rs` lines 38–42, add to any new file that calls this):
```rust
use lsp_types::{MessageType, ShowMessageParams};
```

### Error Handling Convention (`anyhow` + `?`)

**Source:** `lapce-app/src/update.rs` lines 3–4, `lapce-proxy/src/dispatch.rs` line 14

**Apply to:** All `.unwrap()` replacements (CRASH-02, CRASH-03, CRASH-04).

```rust
use anyhow::{Result, anyhow};

// Option → Result:
let stdin = process.stdin.take()
    .ok_or_else(|| anyhow!("failed to capture DAP stdin"))?;

// io::Error → anyhow:
let tar = zstd::Decoder::new(&mut cursor)
    .map_err(|e| anyhow!("malformed zstd plugin archive: {e}"))?;
```

### Bytes-then-Cursor Pattern (async `reqwest::Response` → `impl Read`)

**Source:** RESEARCH.md §Pattern 2 (verified pattern; `reqwest::Response` does not implement `Read`)

**Apply to:** `update.rs` line 80, `grammars.rs` line 123, `proxy/remote.rs` line 353, `plugin/mod.rs` lines 1590/1594.

```rust
// After async migration, replace resp.copy_to(&mut writer) and
// GzDecoder::new(&mut resp) / zstd::Decoder::new(&mut resp):
let body = resp.bytes()?;
let mut cursor = std::io::Cursor::new(body);
// cursor: impl Read + Seek — pass to GzDecoder, zstd::Decoder, std::io::copy
std::io::copy(&mut cursor, &mut out)?;
```

### Regression Test Seam (`CoreRpcHandler::new()` + `rx()`)

**Source:** `lapce-rpc/src/core.rs` lines 167–207

**Apply to:** All proxy-side crash fix tests (CRASH-03, CRASH-05) that must assert notification emission.

```rust
// CoreRpcHandler::new() creates an unbounded crossbeam channel internally.
// rx() returns &Receiver<CoreRpc> — readable without consuming the handler.
// (core.rs:175–184, 205–207)
pub fn new() -> Self {
    let (tx, rx) = crossbeam_channel::unbounded();
    // ...
}
pub fn rx(&self) -> &Receiver<CoreRpc> {
    &self.rx
}

// Test assertion pattern (RESEARCH.md §Pattern 3):
let core_rpc = CoreRpcHandler::new();
// ... trigger failure path with core_rpc injected ...
let msg = core_rpc.rx()
    .recv_timeout(std::time::Duration::from_millis(100))
    .expect("ShowMessage notification not emitted");
let CoreRpc::Notification(n) = msg else { panic!("expected notification") };
let CoreNotification::ShowMessage { message, .. } = *n else {
    panic!("expected ShowMessage, got {:?}", n)
};
assert_eq!(message.typ, MessageType::ERROR);
```

**Imports needed in test modules:**
```rust
use std::time::Duration;
use lapce_rpc::core::{CoreNotification, CoreRpc, CoreRpcHandler};
use lsp_types::MessageType;
```

### `block_on` Bridge (sync call sites → async core)

**Source:** `lapce-app/src/runtime_tests.rs` lines 17–24 (ambient runtime pattern)

**Apply to:** `lapce-proxy/src/lib.rs` sync shim; any call site that needs to bridge.

```rust
// The ambient runtime is entered via rt.enter() in the binary entry points
// (Phase 2). Handle::current() resolves everywhere in the process.
tokio::runtime::Handle::current()
    .block_on(get_url_async(url, user_agent))
```

**Never use:** `tokio::runtime::Builder::new_*().build().block_on(...)` — a second runtime panics inside the ambient one.

---

## No Analog Found

All files have analogs or are self-migrations. No file requires inventing a pattern from scratch.

| File | Role | Data Flow | Analog Note |
|------|------|-----------|-------------|
| `lapce-app/src/download.rs` (NEW) | network-wrapper | request-response | Closest analog is `update.rs`; the `DownloadPipeline` struct shape has no direct codebase precedent but is a trivial delegation wrapper |

---

## Metadata

**Analog search scope:** `lapce-proxy/src/`, `lapce-app/src/`, `lapce-rpc/src/`, `lapce-app/src/keypress/`
**Files scanned:** 13 (lib.rs, dispatch.rs, plugin/mod.rs, plugin/dap.rs, plugin/catalog.rs, update.rs, plugin.rs, app/grammars.rs, proxy/remote.rs, keypress/condition.rs, keypress/loader.rs, runtime_tests.rs, lapce-rpc/src/core.rs)
**Pattern extraction date:** 2026-06-08
