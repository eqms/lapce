---
phase: 01-dependency-foundation
plan: "02"
subsystem: compile-fixes
tags: [ipc, tracing, interprocess, zip, security, regression-tests]
dependency_graph:
  requires:
    - workspace-dep-interprocess-2.4.2
    - workspace-dep-tracing-stable
    - workspace-dep-zip-2.4.0
  provides:
    - ipc-interprocess-2x-call-sites
    - tracing-subscriber-compile-clean
    - regression-test-zip-slip
    - regression-test-ipc-roundtrip
  affects:
    - lapce-app/src/app.rs
    - lapce-app/src/app/logging.rs
    - lapce-app/src/update.rs
    - lapce-proxy/src/cli.rs
    - lapce-proxy/src/lib.rs
    - Cargo.toml
tech_stack:
  added: []
  patterns:
    - interprocess 2.x ListenerOptions builder API
    - tracing-subscriber reload::Layer (not reload::Subscriber)
    - tracing-subscriber fmt::layer() / fmt::Layer (not subscriber/Subscriber)
    - Handle<L, S> with explicit Registry subscriber type arg
key_files:
  modified:
    - lapce-app/src/app/logging.rs
    - lapce-app/src/app.rs
    - lapce-app/src/update.rs
    - lapce-proxy/src/cli.rs
    - lapce-proxy/src/lib.rs
    - Cargo.toml
decisions:
  - "floem reverted to git rev 31fa8f4 — crates.io 0.2.0 API is incompatible with existing code (documented fallback from PATTERNS.md)"
  - "tracing-subscriber reload feature removed from Cargo features list — reload module is always available in 0.3.x, not a named cargo feature"
  - "Handle<Targets, Registry> used instead of Handle<Targets> — 0.3.23 requires both type args"
  - "IPC regression test is #[cfg(unix)] only — Windows named-pipe CI unreliable; explicit scope decision from plan"
metrics:
  duration_seconds: 900
  completed_date: "2026-06-07"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 7
---

# Phase 01 Plan 02: Compile Fixes + Regression Tests Summary

Fix all compile errors from plan 01-01 dependency upgrades and add two mandatory regression tests: interprocess 2.x IPC rewrite across three files, tracing-subscriber API rename in logging.rs, floem fallback to git rev, and zip-slip + IPC roundtrip regression tests.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Fix tracing-subscriber reload feature + reload::Layer rename | dca9815c | Cargo.toml, lapce-app/src/app/logging.rs |
| 2 | Rewrite interprocess 2.x IPC call sites + regression tests | 6b8bc677 | Cargo.toml, Cargo.lock, lapce-app/src/app.rs, lapce-app/src/app/logging.rs, lapce-app/src/update.rs, lapce-proxy/src/cli.rs, lapce-proxy/src/lib.rs |

## Changes Made

### Cargo.toml (workspace root)

| Change | Detail | Requirement |
|--------|--------|-------------|
| Remove `"reload"` from tracing-subscriber features | reload is an always-available module in 0.3.x, not a cargo feature | DEPS-06 |
| Revert floem to git rev `31fa8f4` | crates.io 0.2.0 palette::css, MouseButton, Renderer APIs incompatible with existing code | DEPS-06 fallback |

### lapce-app/src/app/logging.rs

| Change | Before | After |
|--------|--------|-------|
| `reload::Subscriber::new(...)` | Task 1 (01-01) | `reload::Layer::new(...)` |
| `fmt::subscriber()` | `tracing_subscriber::fmt::subscriber()` | `fmt::layer()` |
| `fmt::Subscriber::default()` | (two occurrences) | `fmt::Layer::default()` |
| Return type `Handle<Targets>` | 1 type arg | `Handle<Targets, Registry>` (2 args) |

### lapce-app/src/app.rs

| Change | Before | After |
|--------|--------|-------|
| interprocess import | none | `use interprocess::local_socket::{GenericFilePath, ListenerOptions, Stream, ToFsName, prelude::*}` |
| `get_socket()` return type | `LocalSocketStream` | `Stream` |
| `get_socket()` body | `LocalSocketStream::connect(path)` | `path.to_fs_name::<GenericFilePath>()? + Stream::connect(name)?` |
| `try_open_in_existing_process` param | `LocalSocketStream` | `Stream` |
| `listen_local_socket()` | `LocalSocketListener::bind(path)` | `ListenerOptions::new().name(name).create_sync()?` |
| `listen_local_socket()` loop | `socket.incoming().flatten()` | `listener.incoming().filter_map(|r| r.ok())` |
| `tracing_handle` field type | `Handle<Targets>` | `Handle<Targets, Registry>` |
| Added test module | — | `#[cfg(test)] mod tests { single_instance_ipc_roundtrip }` |

### lapce-proxy/src/cli.rs

| Change | Before | After |
|--------|--------|-------|
| Import | none | `use interprocess::local_socket::{GenericFilePath, Stream, ToFsName, prelude::*}` |
| `try_open_in_existing_process` body | `LocalSocketStream::connect(path)` | `path.to_fs_name::<GenericFilePath>()? + Stream::connect(name)?` |

### lapce-proxy/src/lib.rs

| Change | Before | After |
|--------|--------|-------|
| Import | none | `use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName, prelude::*}` |
| `listen_local_socket()` | `LocalSocketListener::bind(path)` | `ListenerOptions::new().name(name).create_sync()?` |
| `listen_local_socket()` loop | `socket.incoming().flatten()` | `listener.incoming().filter_map(|r| r.ok())` |

### lapce-app/src/update.rs

Added `#[cfg(test)] mod tests` with `zip_slip_traversal_rejected` test:
- Builds in-memory ZIP with `../escape.txt` traversal entry
- Asserts `ZipArchive::extract()` returns Err or does not write outside tempdir
- Verifies CVE-2025-29787 is closed by zip 2.4.0

## Verification Results

All acceptance criteria met:

| Check | Result |
|-------|--------|
| `cargo build --workspace` exits 0 | PASS |
| `reload::Subscriber` occurrences = 0 | PASS |
| `LocalSocketListener\|LocalSocketStream` in app.rs = 0 | PASS |
| `GenericFilePath` count in app.rs >= 1 | PASS (5) |
| `ListenerOptions` count in app.rs >= 1 | PASS (5) |
| `cargo test single_instance_ipc_roundtrip` | PASS |
| `cargo test zip_slip_traversal_rejected` | PASS |
| `cargo clippy --profile ci -p lapce-app` errors = 0 | PASS |
| `cargo tree -i zip` shows only 2.x | PASS |
| lapce-app uses reqwest 0.12.28 | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tracing-subscriber "reload" feature not a cargo feature**
- **Found during:** Task 1 (first cargo check)
- **Issue:** Plan 01-01 added `"reload"` to tracing-subscriber features but 0.3.23 has no such cargo feature — reload is an always-available module
- **Fix:** Removed `"reload"` from features list in workspace Cargo.toml
- **Files modified:** Cargo.toml
- **Commit:** dca9815c (Task 1)

**2. [Rule 1 - Bug] fmt::subscriber / fmt::Subscriber renamed in tracing-subscriber 0.3.23**
- **Found during:** Task 1 (cargo check after reload feature fix)
- **Issue:** `tracing_subscriber::fmt::subscriber()` function and `fmt::Subscriber` type no longer exist; renamed to `fmt::layer()` and `fmt::Layer`
- **Fix:** Updated logging.rs to use `fmt::layer()` and `fmt::Layer::default()`
- **Files modified:** lapce-app/src/app/logging.rs
- **Commit:** 6b8bc677 (Task 2)

**3. [Rule 1 - Bug] Handle<L, S> requires 2 generic type args in tracing-subscriber 0.3.23**
- **Found during:** Task 1 (cargo check)
- **Issue:** `Handle<Targets>` compiles as 1-arg form in old git rev but 0.3.23 stable `Handle<L, S>` requires both layer and subscriber types
- **Fix:** Changed to `Handle<Targets, Registry>` in logging.rs return type and app.rs struct field
- **Files modified:** lapce-app/src/app/logging.rs, lapce-app/src/app.rs
- **Commit:** 6b8bc677 (Task 2)

**4. [Rule 3 - Blocking] lapce-proxy also had interprocess 1.x call sites**
- **Found during:** Task 2 (cargo check --workspace)
- **Issue:** Plan mentioned only app.rs but lapce-proxy/src/cli.rs and lapce-proxy/src/lib.rs also used `LocalSocketStream::connect` and `LocalSocketListener::bind`
- **Fix:** Applied same interprocess 2.x rewrite to both proxy files
- **Files modified:** lapce-proxy/src/cli.rs, lapce-proxy/src/lib.rs
- **Commit:** 6b8bc677 (Task 2)

**5. [Rule 3 - Blocking / Documented Fallback] floem crates.io 0.2.0 API incompatible**
- **Found during:** Task 2 (cargo check after removing floem git rev)
- **Issue:** floem 0.2.0 on crates.io has different APIs from git rev 31fa8f4: `palette::css`, `MouseButton`, `Renderer`, `pointer_events_auto` etc. all differ — 30+ compile errors
- **Fix:** Reverted floem and floem-editor-core to git rev `31fa8f4` (documented fallback in PATTERNS.md: "attempt crates.io first; fall back to git+rfd-tokio if compile fails")
- **Files modified:** Cargo.toml
- **Commit:** 6b8bc677 (Task 2)

## Known Stubs

None.

## Threat Flags

None — changes are limited to IPC call site migration and compile fixes; no new network endpoints, auth paths, or schema changes introduced.

## Self-Check: PASSED

- [x] lapce-app/src/app/logging.rs contains reload::Layer::new — FOUND
- [x] lapce-app/src/app.rs contains ListenerOptions — FOUND (5 occurrences)
- [x] lapce-app/src/app.rs contains GenericFilePath — FOUND (5 occurrences)
- [x] lapce-app/src/update.rs contains zip_slip — FOUND
- [x] cargo build --workspace exits 0 — VERIFIED
- [x] cargo test single_instance_ipc_roundtrip — PASS
- [x] cargo test zip_slip_traversal_rejected — PASS
- [x] Commit dca9815c exists — VERIFIED
- [x] Commit 6b8bc677 exists — VERIFIED
