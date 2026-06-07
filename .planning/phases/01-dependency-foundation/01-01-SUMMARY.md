---
phase: 01-dependency-foundation
plan: "01"
subsystem: cargo-dependencies
tags: [deps, security, cargo, reqwest, tokio, zip, tracing, floem, alacritty]
dependency_graph:
  requires: []
  provides:
    - workspace-dep-reqwest-0.12.28
    - workspace-dep-tokio-1.52.3
    - workspace-dep-sha2-promoted
    - workspace-dep-zip-2.4.0
    - workspace-dep-tracing-stable
    - workspace-dep-floem-crates-io
    - workspace-dep-alacritty-crates-io
    - workspace-dep-interprocess-2.4.2
  affects:
    - Cargo.toml
    - lapce-app/Cargo.toml
tech_stack:
  added:
    - tokio 1.52.3 (explicit workspace dep)
  patterns:
    - workspace = true promotion (sha2)
    - crates.io pins replace git-rev sources (tracing, floem, alacritty_terminal)
key_files:
  modified:
    - Cargo.toml
    - lapce-app/Cargo.toml
decisions:
  - "floem upgraded to crates.io 0.2.0 with rfd-tokio; fallback to git-rev documented in plan if compile errors arise"
  - "tracing family moved from git rev 908cc43 to crates.io stable; tracing-subscriber includes reload feature for logging.rs compatibility"
  - "alacritty_terminal moved from git rev to crates.io 0.24.1; fallback documented in plan"
  - "sha2 promoted to workspace dep rather than local pin to consolidate versioning"
metrics:
  duration_seconds: 300
  completed_date: "2026-06-07"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 2
---

# Phase 01 Plan 01: Dependency Version Pins Summary

All Cargo.toml-only dependency upgrades for Phase 1: reqwest 0.12.28, tokio workspace dep, zip 2.4.0 CVE fix, tracing/floem/alacritty on crates.io stable, sha2 workspace promotion, interprocess 2.4.2.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Workspace Cargo.toml — all version pin changes | 4f317835 | Cargo.toml |
| 2 | lapce-app/Cargo.toml — zip upgrade and sha2 workspace reference | 5fc1029b | lapce-app/Cargo.toml |

## Changes Made

### Cargo.toml (workspace root)

| Dependency | Before | After | Requirement |
|------------|--------|-------|-------------|
| interprocess | 1.2.1 | 2.4.2 | DEPS-04 |
| reqwest | 0.11 | 0.12.28 | DEPS-01 |
| tokio | (absent) | 1.52.3 (new) | DEPS-02 |
| sha2 | (absent) | 0.10.8 (new workspace dep) | DEPS-07 |
| toml | * (wildcard) | 0.8 | DEPS-05 |
| floem | git rev 31fa8f4 | 0.2.0 (crates.io, rfd-tokio) | DEPS-06 |
| floem-editor-core | git rev 31fa8f4 | 0.2.0 (crates.io) | DEPS-06 |
| tracing | git rev 908cc43 | 0.1.44 (crates.io) | DEPS-06 |
| tracing-log | git rev 908cc43 | 0.2.0 (crates.io) | DEPS-06 |
| tracing-subscriber | git rev 908cc43 | 0.3.23 with reload feature | DEPS-06 |
| tracing-appender | git rev 908cc43 | 0.2.5 (crates.io) | DEPS-06 |
| alacritty_terminal | git rev cacdb5b | 0.24.1 (crates.io) | DEPS-06 |

### lapce-app/Cargo.toml

| Dependency | Before | After | Requirement |
|------------|--------|-------|-------------|
| sha2 | version = "0.10.8" | workspace = true | DEPS-07 |
| zip | 0.6.6 | 2.4.0 (deflate feature retained) | DEPS-03 |

## Verification Results

All acceptance criteria met:
- `grep 'reqwest' Cargo.toml | grep '0.12.28'` — PASS
- `grep 'tokio.*1.52.3' Cargo.toml` — PASS
- `grep 'zip.*2.4.0' lapce-app/Cargo.toml` — PASS
- `grep 'sha2.*workspace.*true' lapce-app/Cargo.toml` — PASS
- `grep -v '^#' Cargo.toml | grep -c 'tokio-rs/tracing'` returns 0 — PASS
- `grep 'rfd-tokio' Cargo.toml` — PASS
- `grep 'toml.*0\.8' Cargo.toml` — PASS
- `grep 'reload' Cargo.toml` — PASS
- `cargo metadata --no-deps --format-version 1` — OK

## Expected Post-Plan State

The workspace will NOT compile cleanly after this plan — this is expected and documented in the plan:
1. `tracing-subscriber` 0.3.23 renames `reload::Subscriber` → `reload::Layer` (fixed in plan 01-02)
2. `interprocess` 2.x removes `LocalSocketListener` / `LocalSocketStream` (fixed in plan 01-02)

Plan 01-02 must run before the workspace compiles cleanly.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — all changes are Cargo.toml version pins only; no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- [x] Cargo.toml modified — FOUND
- [x] lapce-app/Cargo.toml modified — FOUND
- [x] Commit 4f317835 exists — VERIFIED
- [x] Commit 5fc1029b exists — VERIFIED
- [x] cargo metadata resolves — PASS
