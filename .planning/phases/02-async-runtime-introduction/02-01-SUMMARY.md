---
phase: 02-async-runtime-introduction
plan: "01"
subsystem: async-runtime
tags: [tokio, runtime, entry-point, ambient-context]
dependency_graph:
  requires: [01-dependency-foundation]
  provides: [tokio-ambient-runtime]
  affects: [lapce-app/src/bin/lapce.rs, lapce-proxy/src/bin/lapce-proxy.rs]
tech_stack:
  added: []
  patterns: [named-local-runtime-guard, fail-closed-build-error]
key_files:
  created: []
  modified:
    - Cargo.toml
    - lapce-app/Cargo.toml
    - lapce-proxy/Cargo.toml
    - lapce-app/src/bin/lapce.rs
    - lapce-proxy/src/bin/lapce-proxy.rs
decisions:
  - "D-01/D-02: Named-local runtime guard pattern (_rt then _guard) in both bin/*.rs entry points"
  - "D-03: Builder::new_multi_thread().enable_all() for explicit configuration"
  - "D-04: Worker thread names lapce-app-worker and lapce-proxy-worker for profiler visibility"
  - "D-05: Default tokio worker count (number of CPUs) — no cap imposed"
  - "D-06: Build failure exits cleanly via tracing::error! + eprintln! + exit(1), no panic"
  - "D-07: No Handle stored in CommonData or any shared state — purely ambient"
  - "Root crate (lapce) must declare tokio and tracing as direct deps for its [[bin]] targets"
metrics:
  duration_minutes: 8
  completed_date: "2026-06-07"
  tasks_completed: 3
  files_modified: 5
---

# Phase 02 Plan 01: Ambient Tokio Runtime Introduction Summary

**One-liner:** Tokio multi-thread runtime with named-worker-thread EnterGuard pattern in both binary entry points, using fail-closed error handling and correct drop order.

## What Was Built

Introduced a tokio multi-thread runtime as ambient context in both binary entry points:

- `lapce-app/src/bin/lapce.rs`: `Builder::new_multi_thread().enable_all().thread_name("lapce-app-worker").build()` with `EnterGuard` wrapping `app::launch()`
- `lapce-proxy/src/bin/lapce-proxy.rs`: Same pattern with `"lapce-proxy-worker"` thread name wrapping `mainloop()`
- Both Cargo.toml files gain `tokio = { workspace = true }`
- Root `Cargo.toml` gains `tokio` and `tracing` for its `[[bin]]` compilation context

The runtime is ambient and unused in Phase 2. Phase 3 will migrate blocking HTTP calls onto it. The guard ensures `Handle::try_current()` succeeds anywhere in the process, enabling `floem::ext_event::create_signal_from_tokio_channel` (which calls `tokio::spawn`) to work correctly.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add tokio dep to lapce-app and lapce-proxy Cargo.toml | f909e292 | lapce-app/Cargo.toml, lapce-proxy/Cargo.toml |
| 2 | Runtime + EnterGuard in lapce-app/src/bin/lapce.rs | c12a8b4f | lapce-app/src/bin/lapce.rs |
| 3 | Runtime + EnterGuard in lapce-proxy/src/bin/lapce-proxy.rs | ecf8f7f0 | lapce-proxy/src/bin/lapce-proxy.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Root crate missing tokio and tracing dependencies**
- **Found during:** Overall verification after Task 3 (`cargo build --workspace`)
- **Issue:** The root workspace crate `lapce` defines `[[bin]]` targets pointing to `lapce-app/src/bin/lapce.rs` and `lapce-proxy/src/bin/lapce-proxy.rs`. When Cargo compiles these binaries, it resolves external crate names against the **root crate's** dependency set, not `lapce-app`/`lapce-proxy`'s. The binary files now reference `tokio` and `tracing` directly, so both must be in root `[dependencies]`.
- **Fix:** Added `tokio = { workspace = true }` and `tracing = { workspace = true }` to root `Cargo.toml [dependencies]`
- **Files modified:** `Cargo.toml`
- **Commit:** 0c808424

The plan only mentioned adding tokio to `lapce-app/Cargo.toml` and `lapce-proxy/Cargo.toml`, which is correct for crate-level compilation (`cargo build -p lapce-app`, `cargo build -p lapce-proxy`). The root crate requirement was not anticipated but was mandatory for `cargo build --workspace` to succeed.

### Pre-existing fmt Issues (Out of Scope)

`cargo fmt --all --check` shows diffs in `lapce-app/src/app.rs`, `lapce-app/src/app/logging.rs`, `lapce-app/src/update.rs`, `lapce-proxy/src/cli.rs`, `lapce-proxy/src/lib.rs`. These are all pre-existing from Phase 1 and are NOT caused by Plan 02-01 changes. The two entry-point files written in this plan (`lapce.rs`, `lapce-proxy.rs`) are individually fmt-clean (verified with `rustfmt --check`). Pre-existing fmt issues are logged here per scope-boundary rules and deferred to a future cleanup pass.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build --workspace` exits 0 | PASS |
| No `#[tokio::main]` in entry files | PASS (0 matches) |
| `lapce-app-worker` thread name in lapce.rs | PASS |
| `lapce-proxy-worker` thread name in lapce-proxy.rs | PASS |
| `process::exit` fail-closed path in lapce.rs | PASS (1 match) |
| `process::exit` fail-closed path in lapce-proxy.rs | PASS (1 match) |
| `cargo clippy --profile ci` — no `^error` lines | PASS |
| `cargo build -p lapce-app` exits 0 | PASS |
| `cargo build -p lapce-proxy` exits 0 | PASS |
| `rustfmt --check` on lapce.rs | PASS |
| `rustfmt --check` on lapce-proxy.rs | PASS |

## Known Stubs

None. This plan introduces runtime scaffolding only — no UI rendering paths, no data sources, no placeholder text.

## Threat Flags

No new threat surface introduced. The runtime is ambient and unused; the only observable external effect is a possible stderr message on catastrophic startup failure, which was pre-assessed as acceptable (T-02-01, T-02-02 in plan threat model).

## Self-Check: PASSED

All files modified exist and commits are present in git log.
