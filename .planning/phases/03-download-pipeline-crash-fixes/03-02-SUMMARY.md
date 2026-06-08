---
phase: 03-download-pipeline-crash-fixes
plan: 02
subsystem: crash-fix
tags: [rust, dispatch, git, rpc, panic-elimination, error-surfacing]

requires:
  - phase: 02-async-runtime
    provides: ambient tokio runtime — not directly used here, but CRASH-02/05 are pure sync fixes independent of async migration

provides:
  - CRASH-02 guard: handle_workspace_fs_event returns early without panicking when workspace is None
  - CRASH-05 fix: GitCheckout/GitDiscardFilesChanges/GitDiscardWorkspaceChanges/GitInit arms emit ShowMessage ERROR instead of swallowing via eprintln!
  - D-07 else-branch: all four git arms emit ShowMessage "No folder open" when workspace is None
  - TEST-01 regression tests: three tests in dispatch::crash_fix_tests covering CRASH-02 and CRASH-05

affects: [03-03, 03-04, phase-4-integrity-verification]

tech-stack:
  added: []
  patterns:
    - "CoreRpcHandler::new() + rx().recv_timeout() test seam for asserting notification emission without mocks"
    - "let-else guard for Option fields in background event handlers (D-06)"
    - "ShowMessage ERROR pattern for user-triggered git command arm errors (D-07, D-08)"

key-files:
  created: []
  modified:
    - lapce-proxy/src/dispatch.rs

key-decisions:
  - "Background fs-event handler (handle_workspace_fs_event) returns early silently when workspace is None — no user toast (D-06 rationale: spurious UX)"
  - "User-triggered git arms emit ShowMessage 'No folder open' in else-branch (D-07) — visible error for user action with no open folder"
  - "eprintln! swallows replaced with self.core_rpc.show_message() ERROR (D-08) — errors now visible in the editor UI"
  - "Tests use real CoreRpcHandler + rx() receiver — zero mock structs needed (crossbeam unbounded channel is the seam)"

patterns-established:
  - "CoreRpcHandler test seam: construct CoreRpcHandler::new(), inject, recv_timeout on rx() to assert notification"
  - "Git arm error pattern: match Ok/Err; Err arm calls show_message(title, ShowMessageParams { typ: ERROR, message: e.to_string() }); else-branch emits 'No folder open'"

requirements-completed: [CRASH-02, CRASH-05, TEST-01]

duration: 35min
completed: 2026-06-08
---

# Phase 03 Plan 02: CRASH-02/CRASH-05 Git Dispatch Panic and Error-Swallow Fixes Summary

**let-else guard eliminates workspace-None panic in fs-event handler; four git arms now emit ShowMessage ERROR via CoreRpc instead of swallowing errors with eprintln!; three regression tests lock both guarantees**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-08T12:14:00Z
- **Completed:** 2026-06-08T12:49:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- CRASH-02 eliminated: `self.workspace.clone().unwrap()` at dispatch.rs line 1343 replaced with `let Some(workspace) = self.workspace.clone() else { return; }` — no panic, no spurious notification from the background fs-event handler
- CRASH-05 fixed for all four git arms: `eprintln!("{e:?}")` swallows at lines 358, 369, 377, 385 replaced with `self.core_rpc.show_message(...)` ERROR calls — git errors now surface in the editor UI
- D-07 else-branches added: all four arms (GitCheckout, GitDiscardFilesChanges, GitDiscardWorkspaceChanges, GitInit) emit ShowMessage "No folder open" when workspace is None
- Three regression tests in `dispatch::crash_fix_tests` pass under `cargo test -p lapce-proxy crash_fix`: CRASH-02 no-panic, CRASH-05 no-workspace ShowMessage, CRASH-05 git-error ShowMessage

## Task Commits

1. **Task 1: Guard CRASH-02 and wire CRASH-05 git error surfacing** — `ba0b76dd` (fix)
2. **Task 2: Add regression tests for CRASH-02 and CRASH-05** — `77502440` (test)

## Files Created/Modified

- `/Users/picard/gitbase/lapce/lapce-proxy/src/dispatch.rs` — CRASH-02 let-else guard at line 1407 (was 1343); CRASH-05 eprintln→show_message for four git arms at lines 354–448; D-07 else-branches; crash_fix_tests module at end of file (131 lines added)

## Decisions Made

- Background handler returns silently (no toast) per D-06 — a notification from a file-system event watcher firing while workspace is None would be confusing to the user
- CRASH-05 tests use `GitCheckout` (no-workspace path) and `GitInit` (bad path causing git error) — these are the most direct triggers; the other two arms (GitDiscardFilesChanges, GitDiscardWorkspaceChanges) follow identical code paths and are covered transitively

## Deviations from Plan

None — plan executed exactly as written. The brace-matching issue during test module insertion was an edit artifact, auto-corrected before commit.

## Issues Encountered

- During test insertion, the Edit tool matched the wrong closing `}` causing a brace mismatch (`Ok(ProxyResponse::GlobalSearchResponse {...})` became orphaned). Fixed by correcting the insertion point and removing the duplicate.
- `CoreRpc` does not implement `Debug` — removed the `{:?}` format argument from the assert message. This is expected; the assertion logic is unchanged.
- `rustfmt` reformatted test code (max_width=85) after initial compilation — applied `cargo fmt -p lapce-proxy` and verified tests still pass.

## Known Stubs

None — no placeholder data or TODO values introduced.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundary surfaces introduced. All changes are purely dispatch.rs error-handling improvements.

## Self-Check

- [x] `lapce-proxy/src/dispatch.rs` exists and contains all changes
- [x] `ba0b76dd` commit exists (Task 1 fix)
- [x] `77502440` commit exists (Task 2 tests)
- [x] `cargo fmt --all --check` passes
- [x] `cargo test -p lapce-proxy crash_fix` — 3 passed, 0 failed

## Self-Check: PASSED

All files present, both commits verified, format and tests green.

## Next Phase Readiness

- CRASH-02 and CRASH-05 complete; these fixes are independent of the async migration (Plan 01/03)
- The `GitGetRemoteFileUrl` arm at line 605 also has an `eprintln!` swallow — out of scope for this plan; logged for future cleanup
- CRASH-03 (DAP stdio), CRASH-04 (zstd unwrap), and CRASH-01 (keypress condition) remain for Plans 03-03 and 03-04

---
*Phase: 03-download-pipeline-crash-fixes*
*Completed: 2026-06-08*
