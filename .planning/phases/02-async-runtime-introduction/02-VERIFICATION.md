---
phase: 02-async-runtime-introduction
verified: 2026-06-07T12:00:00Z
status: human_needed
score: 11/12 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Launch the editor binary and confirm it starts without panicking, opens a workspace, and all panels are functional"
    expected: "Editor opens normally; no panic dialog or crash in the terminal; Floem UI renders"
    why_human: "SC-4 (all existing editor behavior continues: LSP, terminal, plugin install, SSH remote) cannot be verified by grep or static analysis — requires runtime smoke test"
  - test: "Open a file, trigger LSP completions, open the terminal, and attempt a plugin install — confirm all work with the runtime present"
    expected: "Completions appear, terminal opens, plugin install completes — no regressions from the ambient tokio runtime"
    why_human: "Behavioral parity with Phase 1 cannot be verified statically"
---

# Phase 02: Async Runtime Introduction — Verification Report

**Phase Goal:** A tokio multi-thread runtime is ambient in both binaries; the editor behaves identically to Phase 1.
**Verified:** 2026-06-07
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | Editor launches without panicking; Floem retains main-thread ownership; tokio runtime is ambient via `rt.enter()` guard for process lifetime | ? UNCERTAIN (human) | `lapce-app/src/bin/lapce.rs`: `_rt` declared at line 8, `_guard = _rt.enter()` at line 20, `app::launch()` at line 21 — guard wraps entire launch; no `#[tokio::main]`; Floem event loop not touched. Static structure is correct. Runtime launch without panic requires human smoke-test. |
| SC-2 | lapce-proxy binary launches with tokio runtime ambient before `mainloop()` | ✓ VERIFIED | `lapce-proxy/src/bin/lapce-proxy.rs`: `_rt` at line 6, `_guard = _rt.enter()` at line 18, `mainloop()` at line 19 — guard precedes mainloop. Pattern matches D-01/D-02 exactly. |
| SC-3 | No `#[tokio::main]` macro appears in either entry-point file | ✓ VERIFIED | `grep -c "tokio::main" lapce-app/src/bin/lapce.rs` → 0; `grep -c "tokio::main" lapce-proxy/src/bin/lapce-proxy.rs` → 0. |
| SC-4 | All existing editor behavior continues to work with the runtime present but unused | ? UNCERTAIN (human) | Static analysis confirms the runtime is ambient-only and no call sites were changed. Behavioral parity (LSP, terminal, plugin install, SSH remote) requires a live runtime smoke-test. |

**Score:** 2/4 ROADMAP success criteria statically verifiable; 2 require human confirmation.

### Plan Must-Haves (02-01)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Both binary entry points declare tokio as a direct crate dependency | ✓ VERIFIED | `lapce-app/Cargo.toml:43` `tokio = { workspace = true }`; `lapce-proxy/Cargo.toml:28` `tokio = { workspace = true }`; root `Cargo.toml:13` `tokio = { workspace = true }` (required for [[bin]] compilation) |
| 2 | lapce.rs constructs runtime + EnterGuard in main(), holds for full process lifetime (D-01) | ✓ VERIFIED | `_rt` at line 8; `_guard = _rt.enter()` at line 20; `app::launch()` at line 21 — guard is live during entire `launch()` call |
| 3 | lapce-proxy.rs does the same, wrapping mainloop() (D-01) | ✓ VERIFIED | `_rt` at line 6; `_guard = _rt.enter()` at line 18; `mainloop()` at line 19 |
| 4 | Both binaries bind `_rt` before `_guard` (D-02 drop order) | ✓ VERIFIED | lapce.rs: `_rt` line 8, `_guard` line 20; lapce-proxy.rs: `_rt` line 6, `_guard` line 18. Rust drops in reverse declaration order; `_guard` drops before `_rt` — correct. |
| 5 | Runtime built with `Builder::new_multi_thread().enable_all()` (D-03) | ✓ VERIFIED | Both files: `.enable_all()` present on the builder chain. |
| 6 | Worker threads named "lapce-app-worker" (GUI) and "lapce-proxy-worker" (proxy) (D-04) | ✓ VERIFIED | lapce.rs line 10: `.thread_name("lapce-app-worker")`; lapce-proxy.rs line 8: `.thread_name("lapce-proxy-worker")` |
| 7 | No `.worker_threads()` cap — tokio default worker count (D-05) | ✓ VERIFIED | Neither file contains `.worker_threads()` call. |
| 8 | Runtime build failure exits cleanly via `tracing::error!` + `eprintln!` + `exit(1)`; no panic, no `.expect()` (D-06) | ✓ VERIFIED | Both files: `Err(e)` arm calls `tracing::error!`, `eprintln!`, `std::process::exit(1)`. No `.expect()` or `.unwrap()` on the build result. |
| 9 | No `tokio::runtime::Handle` stored in `CommonData` or any shared state (D-07) | ✓ VERIFIED | `grep -rn "tokio::runtime::Handle" lapce-app/src/ lapce-proxy/src/` outside `runtime_tests.rs` returns empty. Handle is not propagated into application state. |
| 10 | No `#[tokio::main]` in either entry-point file | ✓ VERIFIED | Both files: 0 matches. |
| 11 | `cargo build --workspace` exits 0 (confirmed by SUMMARY + commit log) | ✓ VERIFIED | SUMMARY records PASS; commits f909e292, c12a8b4f, ecf8f7f0, 0c808424 exist in git log and match described changes. Root crate fix (0c808424) was required and applied. |

### Plan Must-Haves (02-02)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 12 | Regression test asserts `Handle::try_current()` succeeds inside entered multi-thread runtime context | ✓ VERIFIED (with caveat — see WR-01) | `lapce-app/src/runtime_tests.rs` exists; `Handle::try_current()` called at line 27; `RuntimeFlavor::MultiThread` asserted at line 32-35; module registered in `lapce-app/src/lib.rs:50` under `#[cfg(test)]`. Test builds its own runtime — does not exercise the binary entry-point guard. |

**Overall score:** 11/12 must-haves VERIFIED (1 has caveat from WR-01 — test passes but its stated guard purpose is overstated).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `lapce-app/Cargo.toml` | tokio workspace dep | ✓ VERIFIED | Line 43: `tokio = { workspace = true }` |
| `lapce-proxy/Cargo.toml` | tokio workspace dep | ✓ VERIFIED | Line 28: `tokio = { workspace = true }` |
| `Cargo.toml` (root) | tokio + tracing workspace deps | ✓ VERIFIED | Lines 13-14: both present (unplanned but required fix for [[bin]] compilation context) |
| `lapce-app/src/bin/lapce.rs` | GUI binary entry with ambient runtime | ✓ VERIFIED | Contains `Builder::new_multi_thread`, `lapce-app-worker`, `_guard = _rt.enter()`, `app::launch()` |
| `lapce-proxy/src/bin/lapce-proxy.rs` | Proxy binary entry with ambient runtime | ✓ VERIFIED | Contains `Builder::new_multi_thread`, `lapce-proxy-worker`, `_guard = _rt.enter()`, `mainloop()` |
| `lapce-app/src/runtime_tests.rs` | Regression test for RT-01 ambient runtime invariant | ✓ VERIFIED (exists, substantive, wired) | File exists; contains `Handle::try_current`; registered in `lib.rs:50` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `lapce-app/src/bin/lapce.rs` | `tokio::runtime::Runtime::enter()` | `_guard = _rt.enter()` before `app::launch()` | ✓ WIRED | Line 20: `let _guard = _rt.enter();` Line 21: `app::launch();` — guard is live when launch executes |
| `lapce-proxy/src/bin/lapce-proxy.rs` | `tokio::runtime::Runtime::enter()` | `_guard = _rt.enter()` before `mainloop()` | ✓ WIRED | Line 18: `let _guard = _rt.enter();` Line 19: `mainloop();` — guard is live when mainloop executes |
| `lapce-app/src/runtime_tests.rs` | `tokio::runtime::Handle::try_current()` | `#[test]` inside entered runtime context | ✓ WIRED | Module registered in `lib.rs:50`; test calls `Handle::try_current()` at line 27 inside `let _guard = rt.enter()` scope |

### Data-Flow Trace (Level 4)

Not applicable. This phase introduces no new data rendering paths; it is purely runtime scaffolding in entry-point files and a test-only regression module.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| No `#[tokio::main]` in lapce.rs | `grep -c "tokio::main" lapce-app/src/bin/lapce.rs` | 0 | ✓ PASS |
| No `#[tokio::main]` in lapce-proxy.rs | `grep -c "tokio::main" lapce-proxy/src/bin/lapce-proxy.rs` | 0 | ✓ PASS |
| `lapce-app-worker` thread name in lapce.rs | `grep "lapce-app-worker" lapce-app/src/bin/lapce.rs` | matched line 10 | ✓ PASS |
| `lapce-proxy-worker` thread name in lapce-proxy.rs | `grep "lapce-proxy-worker" lapce-proxy/src/bin/lapce-proxy.rs` | matched line 8 | ✓ PASS |
| Fail-closed exit path in lapce.rs | `grep "process::exit" lapce-app/src/bin/lapce.rs` | line 17 | ✓ PASS |
| Fail-closed exit path in lapce-proxy.rs | `grep "process::exit" lapce-proxy/src/bin/lapce-proxy.rs` | line 15 | ✓ PASS |
| `_guard` declared after `_rt` in lapce.rs | `grep -n "let _rt\|let _guard" lapce-app/src/bin/lapce.rs` | _rt line 8, _guard line 20 | ✓ PASS |
| `_guard` declared after `_rt` in lapce-proxy.rs | `grep -n "let _rt\|let _guard" lapce-proxy/src/bin/lapce-proxy.rs` | _rt line 6, _guard line 18 | ✓ PASS |
| `mod runtime_tests` registered in lib.rs | `grep "mod runtime_tests" lapce-app/src/lib.rs` | line 50 | ✓ PASS |
| Runtime flavor assertion in test | `grep "RuntimeFlavor::MultiThread" lapce-app/src/runtime_tests.rs` | line 34 | ✓ PASS |
| cargo build --workspace (from SUMMARY) | Documented in SUMMARY as PASS; commits match described changes | All 4 commits verified in git log | ✓ PASS |

### Probe Execution

No probe scripts declared or found for this phase. Step 7c: SKIPPED (no probe-*.sh files found in scripts/).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RT-01 | 02-01, 02-02 | A `tokio` multi-thread runtime is constructed at each binary entry and held alive via an `rt.enter()` guard — no `#[tokio::main]`, no nested runtime | ✓ SATISFIED | Both entry-point files have the correct pattern; no `#[tokio::main]`; runtime is ambient only |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `lapce-app/src/runtime_tests.rs` | 1-9 | Comment overstates scope: says test "guards against a future change that accidentally removes the `rt.enter()` guard in `lapce-app/src/bin/lapce.rs`" — but test builds its own runtime and cannot detect binary entry-point changes | ⚠️ Warning (WR-01 from code review) | False confidence that binary entry-point guard is regression-tested; the test validates the tokio API contract only, not the binary's specific guard |
| `lapce-app/src/runtime_tests.rs` | 11-12 | Redundant double `#[cfg(test)]` gating: file included under `#[cfg(test)]` in lib.rs, then inner module also `#[cfg(test)]`. Inner module named `runtime_tests` same as outer file module — test full path is `lapce_app::runtime_tests::runtime_tests::...` | ⚠️ Warning (WR-02 from code review) | Confusing test path; harder to target with `cargo test --exact`; cosmetic, no functional impact |
| `lapce-app/src/bin/lapce.rs` | 15 | `tracing::error!` called before any tracing subscriber is initialized in the build-failure error path; the call is a silent no-op (IN-01 from code review) | ℹ️ Info | No functional defect — `eprintln!` on the next line correctly handles user-visible output; misleading for future readers only |
| `lapce-proxy/src/bin/lapce-proxy.rs` | 13 | Same as IN-01 above — `tracing::error!` before subscriber initialization | ℹ️ Info | Same as above |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-modified file. No unresolved debt markers.

### WR-01 Deep Assessment

The code review finding WR-01 is material to the phase's stated regression test goal but is **not a BLOCKER** for the phase's four ROADMAP success criteria. Here is the analysis:

**What the test actually proves:** The tokio API contract — that `Builder::new_multi_thread().enable_all().build()` produces a runtime where `rt.enter()` makes `Handle::try_current()` return `Ok` with `RuntimeFlavor::MultiThread`. This is legitimately useful as a canary for tokio API breakage.

**What the test cannot prove:** Whether the binary entry-point files (`lapce.rs`, `lapce-proxy.rs`) actually call `rt.enter()`. The test builds its own isolated runtime; removing `let _guard = _rt.enter()` from `lapce.rs` would not cause this test to fail.

**Project requirement scope:** CLAUDE.md Key Decision states "every crash/security fix requires a reproducing regression test." RT-01 is neither a crash fix nor a security fix — it is a structural runtime introduction. REQUIREMENTS.md TEST-01 ("Every crash and security fix ships with a regression test") maps to Phases 3+4, not Phase 2. Therefore, the strict regression test requirement does not apply to this phase.

**Impact on phase goal:** The four ROADMAP success criteria for Phase 2 do not include a regression test requirement. The test was added by the planner as proactive quality scaffolding. Its structural limitation (WR-01) means future accidental removal of the entry-point guard would not be caught by this test — the comment should be corrected, or a `debug_assert!(Handle::try_current().is_ok())` added to `app::launch()`. This is a quality improvement, not a phase-goal blocker.

**Assessment:** WR-01 is a WARNING (comment accuracy / false confidence), not a BLOCKER for Phase 2 goal achievement.

### Human Verification Required

#### 1. Editor Launch Without Panic

**Test:** Launch the `lapce` binary from the build output directory (or `cargo run --bin lapce`) and open a local workspace folder.
**Expected:** The editor window appears; no panic dialog or crash; Floem UI renders correctly; the status bar shows the workspace name.
**Why human:** SC-1 requires behavioral verification — the runtime + EnterGuard pattern can be verified statically, but "Floem retains main-thread ownership" and "no panic" during actual launch require a running process.

#### 2. Existing Behavior Parity (LSP, Terminal, Plugin, SSH)

**Test:** With the editor running from this phase's build:
- Open a Rust file and trigger completions (LSP)
- Open the integrated terminal (Ctrl+backtick)
- Open the plugin browser and install a plugin
- (If SSH remote available) Open an SSH remote workspace
**Expected:** All four behaviors work identically to Phase 1 — no regressions, no hangs, no unexpected errors. The tokio runtime being ambient but unused must not interfere with any of these paths.
**Why human:** Behavioral parity across LSP/terminal/plugin/SSH requires runtime exercising; static analysis cannot verify that adding an ambient runtime context does not cause hidden conflicts with existing synchronous or thread-management code.

## Gaps Summary

No BLOCKER gaps found. All statically verifiable must-haves are VERIFIED. Two ROADMAP success criteria (SC-1 and SC-4) require human smoke-testing to confirm behavioral parity and launch-without-panic.

Two code review WARNINGs (WR-01, WR-02) are carried forward as quality items:
- WR-01: Test comment overstates what the test guards; the binary entry-point guard is not actually regression-tested. Recommended fix: correct the comment, or add `debug_assert!(tokio::runtime::Handle::try_current().is_ok())` to `app::launch()`.
- WR-02: Redundant double `#[cfg(test)]` and self-referential module name. Recommended fix: remove inner `#[cfg(test)] mod runtime_tests { }` wrapper.

Neither WARNING blocks phase goal achievement — they are improvements for the next cleanup pass.

---

_Verified: 2026-06-07_
_Verifier: Claude (gsd-verifier)_
