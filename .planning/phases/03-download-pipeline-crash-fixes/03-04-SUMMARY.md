---
phase: 03-download-pipeline-crash-fixes
plan: 04
subsystem: testing
tags: [rust, tracing, keypress, condition, regression-test]

requires:
  - phase: 03-01
    provides: ordering hygiene (wave 2 dependency)

provides:
  - CRASH-01 regression tests locking unknown-condition non-panic semantics (D-11)
  - D-10 load-time tracing::warn! for unparseable keymap when-condition tokens

affects:
  - phase 4 (integrity verification — no direct dep, but establishes test pattern)

tech-stack:
  added: []
  patterns:
    - "TDD regression test: extend existing #[cfg(test)] mod test block with named contract-locking tests"
    - "Load-time warn pattern: parse token at load time, emit tracing::warn! on Err, no eval-time change"

key-files:
  created: []
  modified:
    - lapce-app/src/keypress/condition.rs
    - lapce-app/src/keypress/loader.rs

key-decisions:
  - "D-09/D-11: check_condition is already panic-free upstream; regression tests lock the contract, not fix a live bug"
  - "D-10: warn at load time only — eval-time stays a silent skip to avoid per-keystroke UI spam"
  - "D-11 semantics: unknown token → false (binding skipped), !unknown → true (permissive)"

patterns-established:
  - "CRASH-01 regression: named test functions in existing mod test block, reuse MockFocus"
  - "Condition token validation: split on | and &, trim whitespace and !, parse::<Condition>().is_err()"

requirements-completed:
  - CRASH-01
  - TEST-01

duration: 8min
completed: 2026-06-08
---

# Phase 3 Plan 4: CRASH-01 Regression Tests + D-10 Load-time Warn Summary

**Regression tests locking the unknown-condition non-panic contract in condition.rs and a load-time tracing::warn! in loader.rs for malformed keymap when-condition tokens**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-08T12:35:00Z
- **Completed:** 2026-06-08T12:43:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Added `unknown_condition_evaluates_to_false_not_panic` regression test locking D-11 semantics
- Added `negated_unknown_condition_evaluates_to_true_not_panic` regression test locking D-11 semantics
- Added `tracing::warn!` in `KeyMapLoader::load_from_str` for any keymap `when` token that fails `Condition::from_str` (D-10)
- All tests pass, `cargo fmt --all --check` passes, `cargo check -p lapce-app` clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Add CRASH-01 regression tests and D-10 load-time warn** - `dc0755d2` (test)

**Plan metadata:** see final docs commit below

## Files Created/Modified

- `lapce-app/src/keypress/condition.rs` - Two new regression test functions appended to existing `#[cfg(test)] mod test` block
- `lapce-app/src/keypress/loader.rs` - Added `use super::condition::Condition;` import, `warn` added to tracing import, warn block in `load_from_str` after keymap construction

## Decisions Made

- Followed locked decisions D-09, D-10, D-11 exactly — no new decisions needed
- Used rustfmt-compliant inline call style (max_width=85 allows the lines to fit on one line)
- Used `tracing::warn!` via the `warn` alias already imported with `tracing::{debug, error, warn}` rather than fully-qualified form, for consistency with the file's existing style

## Deviations from Plan

None — plan executed exactly as written. One minor formatting adjustment: the initial multi-line `assert!` call was reformatted to single-line by `cargo fmt --all` (both lines fit within max_width=85); corrected before commit.

## Issues Encountered

- `cargo test -p lapce-app unknown_condition negated_unknown` failed with "unexpected argument" — cargo test only accepts one test name filter. Used `cargo test -p lapce-app unknown_condition` instead (matches both test function names as a prefix).

## Self-Check

- [x] `lapce-app/src/keypress/condition.rs` — both test functions present (lines 164, 178)
- [x] `lapce-app/src/keypress/loader.rs` — `warn!` call present (line 53), `parse::<Condition>()` present (line 52)
- [x] Commit `dc0755d2` exists in git log
- [x] `cargo fmt --all --check` — passes (no output)
- [x] `cargo check -p lapce-app` — exits 0 (1 pre-existing unrelated warning in logging.rs)
- [x] Both regression tests pass

## Self-Check: PASSED

## Next Phase Readiness

- Phase 3 plan 4 of 4 complete — all CRASH-01/TEST-01 requirements addressed
- Phase 4 (integrity verification) can build on the async download call sites established in plans 01-03

---
*Phase: 03-download-pipeline-crash-fixes*
*Completed: 2026-06-08*
