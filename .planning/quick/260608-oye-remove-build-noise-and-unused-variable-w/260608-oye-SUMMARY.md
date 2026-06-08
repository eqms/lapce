---
phase: 260608-oye
plan: "01"
subsystem: build / logging
tags: [build-script, warnings, cleanup]
dependency_graph:
  requires: []
  provides: [clean-build-output]
  affects: [lapce-core/build.rs, lapce-app/src/app/logging.rs]
tech_stack:
  added: []
  patterns: [let _ = binding suppression]
key_files:
  created: []
  modified:
    - lapce-core/build.rs
    - lapce-app/src/app/logging.rs
decisions:
  - "Delete informational println!(cargo::warning=...) lines, preserve failure-path warnings"
  - "Use `let _ =` (not `_res` or #[allow(unused)]) to suppress the spawn result — idiomatic Rust"
metrics:
  duration: "< 5 minutes"
  completed: "2026-06-08"
---

# Phase 260608-oye Plan 01: Remove Build Noise and Unused Variable Warning Summary

**One-liner:** Deleted two informational `cargo::warning=` prints from build.rs and suppressed the unused `res` binding on notify-send spawn, eliminating three build-time warnings with no behaviour change.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Remove informational cargo::warning= prints from build.rs | 2c232094 | lapce-core/build.rs |
| 2 | Suppress unused-result binding in logging.rs | 2c232094 | lapce-app/src/app/logging.rs |

## Changes Made

### lapce-core/build.rs
- Removed comment `// Print info to terminal during compilation` and `println!("cargo::warning=Compiling meta: {release_info:?}")` (lines 20-21 before edit).
- Removed `println!("cargo::warning=Commit found: {commit:?}")` (line 99 before edit).
- Retained `println!("cargo::warning=Failed to obtain git repo: {err}")` (line 87) and `println!("cargo::warning=Failed to obtain head: {err}")` (line 94) — these are legitimate failure diagnostics.
- `commit.map(...)` on the following line is unaffected; the `commit` binding is still used.

### lapce-app/src/app/logging.rs
- Changed `let res = std::process::Command::new("notify-send")` to `let _ = std::process::Command::new("notify-send")`.
- The `.args([...]).spawn()` chain and every other byte of the function are unchanged.

## Verification

```
touch lapce-core/build.rs
cargo build --workspace 2>&1 | grep -E "cargo::warning=Compiling meta|cargo::warning=Commit found|unused variable: res"
```

Result: zero matching lines. Build exits `Finished dev profile`.

The `block v0.1.6` future-incompat warning remains — expected (transitive floem dependency, out of scope).

## Deviations from Plan

None — plan executed exactly as written.

## Threat Flags

None — mechanical line deletion only; no logic, no trust boundaries crossed.

## Self-Check: PASSED

- lapce-core/build.rs exists and is modified: FOUND
- lapce-app/src/app/logging.rs exists and is modified: FOUND
- Commit 2c232094 exists: FOUND
- Build produces zero target warnings: VERIFIED
