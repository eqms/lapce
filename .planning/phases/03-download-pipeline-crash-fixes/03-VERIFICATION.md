---
phase: 03-download-pipeline-crash-fixes
verified: 2026-06-08T14:45:00Z
status: human_needed
score: 6/6 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "SSH proxy download else-branch now returns Err (fail-closed); commit d69b3665"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Connect to a remote host via SSH with a controlled network that returns a non-2xx response for the proxy binary download URL"
    expected: "The connection should fail with a clear error message surfaced in the UI, not proceed to attempt SSH installation with a missing binary"
    why_human: "Requires a real SSH remote target with controllable network conditions; no HTTP mock or SSH mock infrastructure exists in this project's test harness"
---

# Phase 3: Download Pipeline + Crash Fixes — Verification Report

**Phase Goal:** All network I/O runs on the async DownloadPipeline; no blocking download call
sites remain; all panic sites are eliminated and errors reach the user
**Verified:** 2026-06-08T14:45:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (commit d69b3665)

---

## Re-verification Summary

**Gap closed:** CR-03 fail-open SSH proxy download. The `else` branch at
`lapce-app/src/proxy/remote.rs:361-367` now contains `return Err(anyhow!(...))`.
Control flow no longer falls through to `mkdir`/`upload_file` on a non-2xx
HTTP response. The fix is fail-closed.

**All automated checks pass:**
- `grep -rn "reqwest::blocking"` — exit 1 (zero results workspace-wide)
- `cargo build --workspace` — Finished ci profile, no errors
- `cargo test --workspace` — all test result lines show 0 failed
- All 8 crash-fix regression tests pass (see Behavioral Spot-Checks below)

**One human verification item remains** (carried forward from initial verification):
the `download_remote` fix has no automated regression test because the function
is SSH-bound and HTTP-bound with no mock infrastructure. Manual end-to-end
SSH bootstrap with a controlled non-2xx response is required to close this item.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `grep -rn "reqwest::blocking"` returns zero results workspace-wide | VERIFIED | `grep -rn "reqwest::blocking" lapce-app/ lapce-proxy/ lapce-rpc/ lapce-core/` exits 1 (zero matches). Confirmed live in re-verification. |
| 2 | Plugin install, self-update check, and SSH remote proxy bootstrap all complete via the async DownloadPipeline | VERIFIED | `remote.rs:361-367`: else-branch now `return Err(anyhow!(...))`. `mkdir`/`upload_file` at lines 369/385 unreachable on failed download. Fail-closed. Commit d69b3665. |
| 3 | Typing a compound keybinding (AND/OR/NOT condition) no longer crashes; evaluates or skipped gracefully | VERIFIED | `unknown_condition_evaluates_to_false_not_panic` and `negated_unknown_condition_evaluates_to_true_not_panic` both pass. |
| 4 | Triggering a git operation with no folder open surfaces a user-visible error notification instead of crashing | VERIFIED | `crash_02_no_panic_and_no_notification_when_workspace_none`, `crash_05_git_checkout_no_workspace_emits_show_message`, `crash_05_git_init_error_emits_show_message` — all 3 pass. |
| 5 | A malformed/corrupted zstd plugin archive surfaces an error notification instead of crashing | VERIFIED | `crash_04_corrupt_zstd_returns_err` and `crash_04_error_reaches_notification_channel` both pass. |
| 6 | Each of the five crash/stability fixes ships with a regression test asserting the error reaches the UI as a notification | VERIFIED | 8 tests pass across lapce-proxy + lapce-app. No regression test exists for `download_remote` (SSH/HTTP-bound, no mock infra — see human verification item below). |

**Score:** 6/6 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `lapce-app/src/download.rs` | DownloadPipeline thin wrapper (RT-03) | VERIFIED | Exists; `pub struct DownloadPipeline;` at line 7; delegates to `lapce_proxy::get_url` at line 14. |
| `lapce-proxy/src/lib.rs` | `get_url_async` async core + `get_url` sync shim | VERIFIED | `pub async fn get_url_async` at line 197; `pub fn get_url` sync shim at line 224 via `Handle::current().block_on`. |
| `lapce-proxy/src/dispatch.rs` | CRASH-02 let-else guard + CRASH-05 show_message | VERIFIED | Guard at line 1407; 4 arms each have `show_message` ERROR and "No folder open" else-branch. No `eprintln!` at old sites. |
| `lapce-proxy/src/plugin/dap.rs` | CRASH-03 — DAP stdio unwrap eliminated | VERIFIED | No `.unwrap()` at stdio capture lines; `ok_or_else(|| anyhow!(...))` confirmed. |
| `lapce-proxy/src/plugin/catalog.rs` | CRASH-03 — show_message in DapStart Err arm | VERIFIED | `show_message` at line 626 in DapStart Err arm; regression test at line 776 passes. |
| `lapce-proxy/src/plugin/mod.rs` | CRASH-04 — zstd unwrap eliminated | VERIFIED | `map_err(|e| anyhow!("malformed zstd plugin archive: {e}"))` at line 1594 confirmed. |
| `lapce-app/src/keypress/condition.rs` | CRASH-01 regression tests | VERIFIED | Both test functions present at lines 164 and 175; both pass. |
| `lapce-app/src/keypress/loader.rs` | D-10 load-time warn for unparseable conditions | VERIFIED | `parse::<Condition>()` at line 52; `warn!` at line 53. |
| `lapce-app/src/proxy/remote.rs` | CR-03 fail-closed on non-2xx download | VERIFIED | `return Err(anyhow!(...))` at lines 363-366 in else branch; `mkdir`/`upload_file` unreachable on download failure. |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `lapce-app/src/download.rs` | `lapce-proxy/src/lib.rs` | `lapce_proxy::get_url` | WIRED | Line 14 delegates to `lapce_proxy::get_url`. |
| `get_url` (sync shim) | `get_url_async` | `Handle::current().block_on` | WIRED | Line 228: `Handle::current().block_on(get_url_async(url, user_agent))`. |
| `dispatch.rs` git arms | `CoreRpcHandler::show_message` | `self.core_rpc.show_message` | WIRED | 8 `show_message` calls confirmed across 4 git arms (Err + None-workspace branches). |
| `dap.rs start_process` | `catalog.rs DapStart Err arm` | `Result` propagation via `?` | WIRED | `.ok_or_else` at dap.rs:104-105; Err propagates to catalog.rs DapStart. |
| `plugin/mod.rs zstd` | `install_volt VoltInstalling` | `?` propagation | WIRED | `map_err` at mod.rs:1594; `?` propagates to `install_volt` which calls `volt_installing`. |
| `lapce-app/src/proxy/remote.rs` | Error propagation on download failure | `return Err(anyhow!(...))` | WIRED | Non-2xx branch at lines 361-367 now returns Err; `?`-chain at call site propagates to SSH bootstrap entrypoint. **Fixed in d69b3665.** |

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo build --workspace` succeeds | `cargo build --workspace --profile ci` | Finished ci profile in 2m 05s, no errors | PASS |
| Zero `reqwest::blocking` references | `grep -rn "reqwest::blocking" lapce-app/ lapce-proxy/ lapce-rpc/ lapce-core/` | exit 1 (zero results) | PASS |
| CRASH-02/05 regression tests pass | `cargo test -p lapce-proxy -p lapce-app` | `crash_02`, `crash_05_git_checkout`, `crash_05_git_init` — all ok | PASS |
| CRASH-03/04 regression tests pass | `cargo test -p lapce-proxy -p lapce-app` | `crash_03_dap_failure`, `crash_04_corrupt_zstd`, `crash_04_error_reaches_notification_channel` — all ok | PASS |
| CRASH-01 regression tests pass | `cargo test -p lapce-app` | `unknown_condition_evaluates_to_false_not_panic`, `negated_unknown_condition_evaluates_to_true_not_panic` — both ok | PASS |
| All workspace tests | `cargo test --workspace` | 12 test-result lines, all 0 failed | PASS |

---

## Code Review Findings: Independent Assessment

### CR-02: `Handle::current().block_on()` nested-runtime risk

**Assessment: LATENT RISK — not a current blocker**

All 11 `get_url` call sites are invoked from `std::thread::spawn` OS threads, not from
tokio async task contexts. The Phase 2 runtime's `rt.enter()` makes `Handle::current()`
safe on any thread. There is no active regression.

This is a legitimate code-quality concern (the `pub` function carries no `# Panics` doc)
but does not constitute a failure of any phase success criterion. Should be addressed
(with `tokio::task::block_in_place`) before Phase 4 adds more download call sites.

**Verdict: WARNING (not BLOCKER for this phase)**

---

### CR-01/CR-04: `target_commitish[..7]` slice panics in update.rs:48 and grammars.rs:106

**Assessment: OUT-OF-SCOPE pre-existing panics — WARNING only**

Both `lapce-app/src/update.rs:48` and `lapce-app/src/app/grammars.rs:106` contain
`&release.target_commitish[..7]` byte-slices that panic when the GitHub API returns a
short branch name (e.g., `"main"` for nightly releases). These are NOT enumerated in
CRASH-01..05 and are pre-existing code. Should be tracked for a follow-up fix.

**Verdict: WARNING (pre-existing, out of scope for CRASH-01..05)**

---

## Requirements Coverage

| Requirement | Plan | Description | Status | Evidence |
|-------------|------|-------------|--------|----------|
| RT-02 | 03-01 | No blocking download call sites remain | SATISFIED | Zero `reqwest::blocking` references; `get_url_async` async core confirmed |
| RT-03 | 03-01 | Shared `DownloadPipeline` component wraps async reqwest | SATISFIED | `lapce-app/src/download.rs` exists; delegates to `lapce_proxy::get_url` |
| CRASH-01 | 03-04 | Compound keybinding conditions no longer panic | SATISFIED | 2 regression tests pass; check_condition confirmed panic-free |
| CRASH-02 | 03-02 | Git operations with no workspace fail gracefully | SATISFIED | let-else guard at dispatch.rs:1407; regression test passes |
| CRASH-03 | 03-03 | DAP stdio-capture failure returns error, not panic | SATISFIED | `ok_or_else` at dap.rs:104-105; show_message in catalog.rs:626; regression test passes |
| CRASH-04 | 03-03 | Malformed zstd archive returns error, not panic | SATISFIED | `map_err` at mod.rs:1594; VoltInstalling notification path confirmed; 2 regression tests pass |
| CRASH-05 | 03-02 | Failed git operations surface via RPC not eprintln | SATISFIED | 4 git arms have `show_message` ERROR; `eprintln!` at lines 358/369/377/385 replaced; 2 regression tests pass |
| TEST-01 | 03-02/03/04 | Regression tests assert error reaches UI as notification | SATISFIED | 8 tests total across lapce-proxy + lapce-app. `download_remote` gap fix has no automated test (SSH/HTTP-bound, no mock infra) — manual verification item raised. |

**Note on remaining `eprintln!` at dispatch.rs:605:** One `eprintln!` remains at line 605 (`GitGetRemoteFileUrl` arm). This is out of scope for CRASH-05 per the plan (which targets lines 358/369/377/385 only). The 03-02-SUMMARY.md explicitly notes it as a known out-of-scope item.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `lapce-app/src/update.rs` | 48 | `&release.target_commitish[..7]` byte-slice — panics if string < 7 bytes | WARNING | Pre-existing; nightly self-update check panics when GitHub API returns branch name |
| `lapce-app/src/app/grammars.rs` | 106 | `&release.target_commitish[..7]` byte-slice — panics if string < 7 bytes | WARNING | Pre-existing; nightly grammar update panics when GitHub API returns branch name |
| `lapce-app/src/proxy/remote.rs` | 93 | `host_specification(&remote).unwrap()` | WARNING | Pre-existing; panics on SSH transport error |
| `lapce-app/src/proxy/remote.rs` | 262 | `Directory::proxy_directory().unwrap()` | WARNING | Pre-existing; panics in sandboxed environments |
| `lapce-app/src/proxy/remote.rs` | 357, 360 | `.expect("failed to create file")` / `.expect("failed to copy content")` | WARNING | Pre-existing; panics on file I/O failure in download success path |

**Note:** The BLOCKER anti-pattern from the initial verification (lines 361-363 fail-open) has been resolved by commit d69b3665.

---

## Human Verification Required

### 1. SSH Remote Proxy Bootstrap End-to-End (gap fix test coverage)

**Test:** Connect to a remote host via SSH with a broken or throttled network that returns a non-2xx response for the proxy binary download URL.
**Expected:** The connection should fail with a clear error message surfaced in the UI, not proceed to attempt SSH installation with a missing binary.
**Why human:** Requires a real SSH remote target with controllable network conditions. The `download_remote` function takes a live `impl Remote` handle that constructs real SSH commands and calls `lapce_proxy::get_url` over a real HTTP connection. There is no HTTP mock or SSH mock infrastructure in this project's test harness. Adding one would require a new trait abstraction (architectural change out of scope per the gap-fix assessment in 03-GAP-SUMMARY.md).

---

## Gaps Summary

No blocking gaps remain. The one gap from the initial verification (CR-03 fail-open SSH proxy download) was resolved by commit d69b3665. All six observable truths are now VERIFIED by automated checks.

The phase cannot be marked `passed` because one human verification item exists (SSH bootstrap end-to-end with non-2xx response). This is an honest reflection of the absence of mock infrastructure for the `download_remote` code path — the fix is correct and verifiable by source reading, but the behavioral assertion requires a real SSH+HTTP environment.

**Out-of-scope follow-up items** (not phase-3 blockers, tracked for later phases):
- CR-01/CR-04: `target_commitish[..7]` slice panics in `update.rs:48` and `grammars.rs:106`
- CR-02: `Handle::current().block_on()` nested-runtime risk in the `get_url` sync shim
- Lines 357/360: `.expect()` calls in `remote.rs` download success path

---

_Initial verification: 2026-06-08T13:30:00Z_
_Re-verified: 2026-06-08T14:45:00Z (after gap closure commit d69b3665)_
_Verifier: Claude (gsd-verifier)_
