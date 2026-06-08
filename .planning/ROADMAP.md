# Roadmap: Lapce Hardening Fork

**Milestone:** Hardening v1 — stability, supply-chain integrity, async runtime, dependency hardening
**Granularity:** Coarse (3-5 phases)
**Created:** 2026-06-07
**Coverage:** 26/26 v1 requirements mapped

---

## Phases

- [x] **Phase 1: Dependency Foundation** - Upgrade all dependency versions; get workspace building cleanly before any logic touches (completed 2026-06-07)
- [x] **Phase 2: Async Runtime Introduction** - Stand up tokio runtime in both binaries; no call-site changes yet (completed 2026-06-07)
- [x] **Phase 3: Download Pipeline + Crash Fixes** - Migrate all blocking HTTP to async DownloadPipeline; eliminate all panic sites; surface errors to UI (completed 2026-06-08)
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

**Plans:** 2/2 plans complete
Plans:
**Wave 1**

- [x] 02-01-PLAN.md — Cargo dep additions (lapce-app + lapce-proxy) and runtime construction in both bin/*.rs entry points

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-02-PLAN.md — Regression test: Handle::try_current() succeeds inside entered multi-thread context

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

**Plans:** 4/4 plans complete

**Wave 1** *(can execute in parallel)*

- [x] 03-01-PLAN.md — Async download pipeline: get_url_async core, DownloadPipeline wrapper, drop blocking feature, migrate all 11 call sites (RT-02, RT-03)
- [x] 03-02-PLAN.md — CRASH-02 guard + CRASH-05 git error surfacing in dispatch.rs, regression tests (CRASH-02, CRASH-05, TEST-01)

**Wave 2** *(blocked on 03-01 completion)*

- [x] 03-03-PLAN.md — CRASH-03 DAP stdio fix + CRASH-04 zstd panic fix + regression tests (CRASH-03, CRASH-04, TEST-01)
- [x] 03-04-PLAN.md — CRASH-01 regression tests + D-10 load-time warn for keymap conditions (CRASH-01, TEST-01)

### Phase 4: Integrity Verification + Archive Hardening

**Goal**: The remote proxy binary is SHA256-verified before execution on stable releases; plugin archives reject path-traversal and symlink-escape entries; the `https_proxy` env is scheme-validated. A reusable verify-before-write integrity primitive is built; plugin (SEC-01) and self-update (SEC-02) verification is deferred to v2 pending a trusted hash source.
**Depends on**: Phase 3
**Requirements**: SEC-03, SEC-04, SEC-05, TEST-01 (live this phase). SEC-01, SEC-02 deferred to v2 — no published SHA256 source exists upstream (see 04-CONTEXT.md).

> **RESOLVED (2026-06-08):** Neither `plugins.lapce.dev` (no `sha256`/`checksum` field) nor `github.com/lapce/lapce` releases (no `SHA256SUMS`/`.sha256` asset) publish an expected hash. Per discussion (option c), SEC-01 plugin + SEC-02 self-update integrity → v2; only proxy-binary verification (SEC-03, stable) goes live, alongside SEC-04 and SEC-05.

**Success Criteria** (what must be TRUE):

  1. Connecting to a remote SSH host on a stable release where the proxy binary hash does not match the build-time pinned value fails before execution; the binary is not written or run (nightly is a documented unverified exception)
  2. A plugin tar archive (zstd or gz) containing a path-traversal (`../`), absolute-path, or symlink/hardlink-escape entry is rejected before any extraction, with no files written outside the plugin directory
  3. Setting `https_proxy` to a disallowed scheme (anything other than http/https/socks5/socks5h, e.g. `ftp://proxy`) causes the request to fail with a logged error; the invalid proxy is never passed to reqwest
  4. A reusable verify-before-write integrity primitive (download → SHA256 in memory → compare → then write) exists and is unit-tested; the SEC-01 and SEC-02 call sites are structured to adopt it but are not gated this milestone
  5. Each security fix shipped this phase (proxy verification, archive guard, proxy-scheme validation) ships with a regression test covering the fail-closed path

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
| 2. Async Runtime Introduction | 2/2 | Complete   | 2026-06-07 |
| 3. Download Pipeline + Crash Fixes | 4/4 | Complete   | 2026-06-08 |
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
*Updated: 2026-06-08 — Phase 3 plans created (03-01 through 03-04)*
