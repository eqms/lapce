---
status: partial
phase: 02-async-runtime-introduction
source: [02-VERIFICATION.md]
started: 2026-06-07T18:55:00Z
updated: 2026-06-07T18:55:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Editor launches without panic
expected: Launch the `lapce` binary (or `cargo run --bin lapce`) and open a local workspace. The editor window appears; no panic dialog or crash in the terminal; Floem UI renders correctly; status bar shows the workspace name.
result: [pending]

### 2. Behavioral parity with Phase 1 (ambient runtime present but unused)
expected: Open a file and trigger LSP completions, open the integrated terminal, and attempt a plugin install. Completions appear, terminal opens, plugin install completes — no regressions from the ambient tokio runtime. (SSH remote optional if available.)
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
