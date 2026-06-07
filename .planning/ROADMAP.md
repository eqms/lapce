# Roadmap: Lapce Hardening Fork

**Milestone:** Hardening v1 — stability, supply-chain integrity, async runtime, dependency hardening
**Granularity:** Coarse (3-5 phases)
**Created:** 2026-06-07
**Coverage:** 26/26 v1 requirements mapped

---

## Phases

- [x] **Phase 1: Dependency Foundation** - Upgrade all dependency versions; get workspace building cleanly before any logic touches (completed 2026-06-07)
- [ ] **Phase 2: Async Runtime Introduction** - Stand up tokio runtime in both binaries; no call-site changes yet
- [ ] **Phase 3: Download Pipeline + Crash Fixes** - Migrate all blocking HTTP to async DownloadPipeline; eliminate all panic sites; surface errors to UI
- [ ] **Phase 4: Integrity Verification + Archive Hardening** - Wire SHA256 verification into all three download paths; path-traversal guards; proxy scheme validation
- [ ] **Phase 5: Performance Caching + Allocation** - Cache hot-path computations; box oversized enum variants; reduce clone overhead where profiling justifies it

---

## Phase Details

### Phase 1: Dependency Foundation

**Goal**: The workspace compiles cleanly with all target dependency versions, with no behaviour change
**Depends on**: Nothing (first phase)
**Requirements**: DEPS-01, DEPS-02, DEPS-03, DEPS-04, DEPS-05, DEPS-06, DEPS-07
**Success Criteria** (what must be TRUE):

  1. `cargo build --workspace` succeeds with reqwest 0.12, tokio in workspace deps, zip 2.x, interprocess 2.x, toml pinned, tracing on versioned releases, sha2 as workspace dep
  2. The editor launches and all existing behavior (LSP, DAP, plugins, terminal, SSH remote) works identically to before
  3. No CVE-2025-29787 vulnerable zip version remains in the dependency tree (`cargo tree -i zip` shows 2.x only)
  4. IPC single-instance detection still prevents duplicate app launches after interprocess 2.x migration

**Plans:** 2/2 plans complete
Plans:
**Wave 1**

- [x] 01-01-PLAN.md — Cargo.toml version pins (reqwest, tokio, zip, toml, sha2, tracing, alacritty, floem)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — Code-site fixes (tracing rename, interprocess 2.x IPC rewrite, regression tests)

### Phase 2: Async Runtime Introduction

**Goal**: A tokio multi-thread runtime is ambient in both binaries; the editor behaves identically to Phase 1
**Depends on**: Phase 1
**Requirements**: RT-01
**Success Criteria** (what must be TRUE):

  1. The editor launches without panicking; Floem retains main-thread ownership; the tokio runtime is ambient via `rt.enter()` guard for process lifetime
  2. The lapce-proxy binary also launches with a tokio runtime ambient before `mainloop()`
  3. No `#[tokio::main]` macro appears anywhere in the entry-point files
  4. All existing editor behavior (LSP, terminal, plugin install, SSH remote) continues to work with the runtime present but unused

**Plans:** 1/2 plans executed
Plans:
**Wave 1**

- [x] 02-01-PLAN.md — Cargo dep additions (lapce-app + lapce-proxy) and runtime construction in both bin/*.rs entry points

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 02-02-PLAN.md — Regression test: Handle::try_current() succeeds inside entered multi-thread context

### Phase 3: Download Pipeline + Crash Fixes

**Goal**: All network I/O runs on the async DownloadPipeline; no blocking download call sites remain; all panic sites are eliminated and errors reach the user
**Depends on**: Phase 2
**Requirements**: RT-02, RT-03, CRASH-01, CRASH-02, CRASH-03, CRASH-04, CRASH-05, TEST-01 (crash fix regression tests)
**Success Criteria** (what must be TRUE):

  1. `grep -rn "reqwest::blocking"` returns zero results across the workspace after the `blocking` feature is dropped
  2. Plugin install, self-update check, and SSH remote proxy bootstrap all complete successfully via the async `DownloadPipeline`
  3. Typing a compound keybinding (AND/OR/NOT condition) no longer causes the editor to crash; the keybinding evaluates or is skipped gracefully
  4. Triggering a git operation with no folder open surfaces a user-visible error notification instead of crashing the editor
  5. A malformed or corrupted zstd plugin archive surfaces an error notification instead of crashing the editor
  6. Each of the five crash/stability fixes ships with a regression test that asserts the error reaches the UI as a notification (not merely that no panic occurs)

**Plans**: TBD

### Phase 4: Integrity Verification + Archive Hardening

**Goal**: Every binary the editor downloads (plugin, self-update, remote proxy) is SHA256-verified before use; path-traversal attacks in archives are rejected; proxy env is validated
**Depends on**: Phase 3
**Requirements**: SEC-01, SEC-02, SEC-03, SEC-04, SEC-05, TEST-01 (security fix regression tests)

> **OPEN QUESTION (note; do not block on it):** Does `plugins.lapce.dev` return a `sha256` or `checksum` field in its API response? Confirm with `curl https://plugins.lapce.dev/api/v1/plugins/<id>/versions` before planning this phase in detail. If no hash field exists, options are: (a) use a `.sha256` companion URL, (b) request registry maintainer to add the field, or (c) scope SEC-01 to self-update + proxy binary only and treat plugin integrity as v2.

**Success Criteria** (what must be TRUE):

  1. Installing a plugin whose archive has been tampered with (hash mismatch) produces a user-visible error and no files are written to disk
  2. Applying a self-update archive with a wrong hash fails closed: error is shown, the existing binary is unchanged
  3. Connecting to a remote SSH host where the proxy binary hash does not match fails before execution; the binary is not run
  4. A zip plugin archive containing a path-traversal entry (e.g. `../../etc/evil`) is rejected during extraction with no files written outside the plugin directory
  5. Setting `https_proxy` to a value with an invalid scheme (e.g. `ftp://proxy`) is rejected with a logged error; the editor does not pass the invalid proxy to reqwest
  6. Each of the five security fixes ships with a regression test covering the fail-closed path

**Plans**: TBD

### Phase 5: Performance Caching + Allocation

**Goal**: Hot-path computations are cached and large enum variants are boxed; the editor is measurably more responsive under file-open and editing workloads
**Depends on**: Phase 1 (independent of Phases 2-4; can be parallelised with them)
**Requirements**: PERF-01, PERF-02, PERF-03, PERF-04, PERF-05
**Success Criteria** (what must be TRUE):

  1. Opening a directory with many files no longer recompiles the `GlobMatcher` on every file listing refresh; directory listing operations are visibly faster on large trees
  2. Switching between documents with different font configs no longer re-parses font families on every render; font-family parsing runs once per config change
  3. Completing a small edit in a large file invalidates only the affected line ranges, not the entire text cache; incremental updates are not measurably slower than before
  4. Oversized message-enum variants in plugin/DAP types are boxed; `cargo bloat` or clippy `large_enum_variant` lint reports no regressions versus Phase 4 baseline

**Plans**: TBD

---

## Progress Table

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Dependency Foundation | 2/2 | Complete   | 2026-06-07 |
| 2. Async Runtime Introduction | 1/2 | In Progress|  |
| 3. Download Pipeline + Crash Fixes | 0/? | Not started | - |
| 4. Integrity Verification + Archive Hardening | 0/? | Not started | - |
| 5. Performance Caching + Allocation | 0/? | Not started | - |

---

## Coverage Map

| Requirement | Phase | Notes |
|-------------|-------|-------|
| DEPS-01 | Phase 1 | reqwest 0.11 → 0.12 |
| DEPS-02 | Phase 1 | tokio added to workspace |
| DEPS-03 | Phase 1 | zip CVE fix |
| DEPS-04 | Phase 1 | interprocess 2.x + call-site migration |
| DEPS-05 | Phase 1 | toml pinned |
| DEPS-06 | Phase 1 | git-SHA pins → tagged releases |
| DEPS-07 | Phase 1 | sha2 promoted to workspace dep |
| RT-01 | Phase 2 | tokio runtime entry-point placement |
| RT-02 | Phase 3 | all reqwest::blocking removed |
| RT-03 | Phase 3 | shared DownloadPipeline built |
| CRASH-01 | Phase 3 | keybinding condition panic |
| CRASH-02 | Phase 3 | git no-workspace panic |
| CRASH-03 | Phase 3 | DAP stdio capture panic |
| CRASH-04 | Phase 3 | malformed zstd panic |
| CRASH-05 | Phase 3 | git errors surfaced to UI |
| SEC-01 | Phase 4 | plugin SHA256 verification |
| SEC-02 | Phase 4 | self-update SHA256 verification |
| SEC-03 | Phase 4 | proxy binary SHA256 verification |
| SEC-04 | Phase 4 | path-traversal guard in archive extraction |
| SEC-05 | Phase 4 | https_proxy scheme validation |
| PERF-01 | Phase 5 | GlobMatcher cache |
| PERF-02 | Phase 5 | font families cache |
| PERF-03 | Phase 5 | per-line cache invalidation |
| PERF-04 | Phase 5 | Arc/clone reduction in render paths |
| PERF-05 | Phase 5 | box oversized enum variants |
| TEST-01 | Phases 3+4 | regression test per crash/security fix (folded into relevant phases) |

**Total v1 requirements: 26 / 26 mapped. No orphans.**

---
*Created: 2026-06-07*
*Updated: 2026-06-07 — Phase 2 plans created (02-01, 02-02)*
