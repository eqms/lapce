---
phase: 03-download-pipeline-crash-fixes
plan: 01
subsystem: http-download-pipeline
tags: [async-migration, reqwest, tokio, rt-02, rt-03, download-pipeline]
dependency_graph:
  requires: [02-02]
  provides: [get_url_async, DownloadPipeline, async-http-core]
  affects: [update.rs, plugin.rs, grammars.rs, remote.rs, plugin/mod.rs]
tech_stack:
  added: []
  patterns:
    - Handle::current().block_on() for sync->async bridging at call sites
    - Bytes-then-Cursor pattern for async reqwest::Response -> impl Read
    - DownloadPipeline thin delegation wrapper (RT-03)
key_files:
  created:
    - lapce-app/src/download.rs
  modified:
    - lapce-proxy/src/lib.rs
    - lapce-app/src/lib.rs
    - lapce-app/src/update.rs
    - lapce-app/src/plugin.rs
    - lapce-app/src/app/grammars.rs
    - lapce-app/src/proxy/remote.rs
    - lapce-proxy/src/plugin/mod.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "Bridge sync->async via Handle::current().block_on() at every response body call (.text/.bytes/.json) — not just at get_url boundary; async reqwest::Response body methods are async futures in sync context"
  - "Preserved zstd .unwrap() in plugin/mod.rs per plan — CRASH-04 fix (Plan 03-03) replaces it with ? propagation"
  - "cargo fmt --all applied; reformatted logging.rs, cli.rs, app.rs as collateral"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-08"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 13
requirements:
  - RT-02
  - RT-03
---

# Phase 03 Plan 01: Atomic Async HTTP Migration Summary

Atomically migrated all 11 blocking HTTP call sites onto the async tokio runtime
introduced in Phase 2, dropping the `reqwest::blocking` Cargo feature and
introducing the `DownloadPipeline` wrapper (RT-02 + RT-03).

## What Was Built

- `lapce-proxy/src/lib.rs`: `get_url` (blocking) replaced with `get_url_async`
  (pub async fn) + `get_url` sync shim via `Handle::current().block_on`. No
  second `tokio::Runtime` constructed — only `Handle::current()`.
- `lapce-app/src/download.rs` (NEW): `DownloadPipeline` thin wrapper delegating
  to `lapce_proxy::get_url`. No second `reqwest::Client` (D-02).
- All 11 call sites across 5 files wired onto the async core.
- `"blocking"` feature removed from workspace `reqwest` in `Cargo.toml`.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | get_url_async + sync shim + Cargo.toml blocking removal | e63df918 |
| 2 | DownloadPipeline + app-side call site migration (update, grammars, remote, plugin) | e63df918 |
| 3 | Proxy-side plugin/mod.rs migration + full workspace build verification | e63df918 |

All three tasks land in a single atomic commit per Critical Constraint (partial
migration panics at runtime — STATE.md Critical Pitfall #1).

## Verification Results

1. `grep -rn "reqwest::blocking" lapce-app/ lapce-proxy/ lapce-rpc/ lapce-core/`
   → **PASS: zero results**
2. `cargo build --workspace` → **PASS: Finished dev profile in 32s**
3. `grep -n "pub async fn get_url_async" lapce-proxy/src/lib.rs` → **PASS: line 197**
4. `grep -n "Handle::current" lapce-proxy/src/lib.rs` → **PASS: line 228**
5. `grep -n "struct DownloadPipeline" lapce-app/src/download.rs` → **PASS: line 7**
6. `grep -n "lapce_proxy::get_url" lapce-app/src/download.rs` → **PASS: line 14**
7. No `.copy_to` in update.rs, grammars.rs, remote.rs → **PASS**
8. `cargo fmt --all --check` → **PASS**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Async response body methods also need block_on at call sites**

- **Found during:** Task 2 — first `cargo check` after wiring call sites
- **Issue:** PLAN.md stated "`.text()`/`.bytes()`/`.json()` calls remain
  identical" — but async `reqwest::Response` body methods are `async fn`
  returning futures, not synchronous. Calling `.text()?` in sync code fails
  with E0277 "the `?` operator cannot be applied to impl Future<...>".
- **Fix:** Added `Handle::current().block_on(resp.text())` etc. at every
  response body call in update.rs, grammars.rs, plugin.rs, remote.rs, and
  plugin/mod.rs. The sync shim `get_url` returns `reqwest::Response` but
  callers must also block_on individual body reads.
- **Files modified:** lapce-app/src/update.rs, lapce-app/src/plugin.rs,
  lapce-app/src/app/grammars.rs, lapce-app/src/proxy/remote.rs,
  lapce-proxy/src/plugin/mod.rs
- **Commit:** e63df918

**2. [Rule 3 - Blocking] cargo fmt --all reformatted collateral files**

- **Found during:** Task 1 format check
- **Issue:** `cargo fmt --all` reformatted logging.rs, cli.rs, app.rs (import
  ordering/line-length adjustments) as side effects.
- **Fix:** Included reformatted files in the atomic commit; no behavior changes.
- **Files modified:** lapce-app/src/app.rs, lapce-app/src/app/logging.rs,
  lapce-proxy/src/cli.rs
- **Commit:** e63df918

## Intentional Preservation

- `zstd::Decoder::new(&mut cursor).unwrap()` in `plugin/mod.rs` is **kept**
  deliberately — CRASH-04 fix (Plan 03-03) replaces it with `?` propagation.
  The Bytes-then-Cursor migration is complete; only the error handling change
  is deferred.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema
changes introduced. The async migration preserves the existing trust boundary
(Internet → lapce-proxy get_url_async) without adding new surface.

## Self-Check: PASSED

- `lapce-app/src/download.rs` exists: FOUND
- Commit e63df918 exists: FOUND
- `grep -rn "reqwest::blocking"` returns zero: VERIFIED
- `cargo build --workspace` exits 0: VERIFIED
- No unexpected file deletions in commit: VERIFIED
