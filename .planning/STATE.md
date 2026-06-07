---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
last_updated: "2026-06-07T10:42:20.140Z"
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 2
  completed_plans: 0
  percent: 0
---

# Project State: Lapce Hardening Fork

**Last updated:** 2026-06-07
**Session:** Initialization — roadmap created

---

## Project Reference

**Core Value:** The editor must never panic on normal user actions, and every binary it downloads (plugin, self-update, remote proxy) must be integrity-verified before execution. Stability and supply-chain safety come first; everything else is secondary.

**Milestone:** Hardening v1 — crash fixes, async runtime, integrity verification, dependency upgrades, performance caching

---

## Current Position

Phase: 01 (dependency-foundation) — EXECUTING
Plan: 1 of 2
**Active Phase:** None (roadmap just created; not yet started)
**Active Plan:** None
**Status:** Executing Phase 01

```
Progress: [          ] 0% — 0 of 5 phases complete
```

**Phase summary:**

- [ ] Phase 1: Dependency Foundation
- [ ] Phase 2: Async Runtime Introduction
- [ ] Phase 3: Download Pipeline + Crash Fixes
- [ ] Phase 4: Integrity Verification + Archive Hardening
- [ ] Phase 5: Performance Caching + Allocation

---

## Performance Metrics

| Metric | Value |
|--------|-------|
| Requirements total | 26 |
| Requirements completed | 0 |
| Phases total | 5 |
| Phases completed | 0 |
| Plans created | 0 |
| Plans completed | 0 |

---

## Accumulated Context

### Key Decisions Logged

| Decision | Rationale | Phase |
|----------|-----------|-------|
| Do not use `#[tokio::main]`; use `rt.enter()` guard pattern | Floem owns main thread; `#[tokio::main]` calls `block_on` on main, incompatible with Floem's event loop | Phase 2 |
| Remove `reqwest::blocking` feature and introduce runtime in same commit batch | Keeping `blocking` after `rt.enter()` is active panics on first HTTP request — cannot coexist | Phase 3 |
| Verify SHA256 in-memory before any disk write (fail-closed by type) | Verify-after-extract is fail-open; TOCTOU window allows malicious content to touch filesystem | Phase 4 |
| Regression test per crash/security fix | Near-zero test coverage in codebase; reproduce-then-fix prevents silent regressions | Phases 3+4 |
| PERF-05 (box large enum variants) is safe to do any time after Phase 1 | Independent of async migration; no blocking deps | Phase 5 |

### Open Questions

| Question | Impact | Resolution Path |
|----------|--------|-----------------|
| Does `plugins.lapce.dev` return a `sha256` field in its API response? | Determines SEC-01 scope — plugin integrity verification may need alternate approach | `curl https://plugins.lapce.dev/api/v1/plugins/<id>/versions` before planning Phase 4 |
| Does floem rev `31fa8f4` post-date the 0.2.0 crates.io tag? | Affects DEPS-06 — whether to pin floem to tag or keep SHA | `git log --oneline v0.2.0..31fa8f4` on the floem repo (low risk either way) |
| Is `alacritty_terminal 0.26` API-compatible? | If not, fall back to 0.25 in Phase 1 | Attempt upgrade in Phase 1; compile errors will indicate if fallback needed |

### Critical Pitfalls (from research)

1. **Nested runtime panic** — `reqwest::blocking` inside an active tokio context panics. Mitigated by removing `blocking` feature and introducing runtime in the same Phase 3 batch.
2. **Verify-after-extract is fail-open** — always: download → hash in memory → verify → extract. The `Result` type enforces this.
3. **`unwrap()` to `?` without UI propagation** — replacing unwrap with `?` that is discarded at thread boundary is not a fix. Each fix must include RPC to a UI notification.
4. **rfd feature flag** — change floem `rfd-async-std` → `rfd-tokio` in Phase 1 or file dialogs will fail after runtime introduction.

### Architecture Constraints

- `lapce-core` must remain UI-framework-free; do not add tokio/reqwest to it
- `DownloadPipeline` lives in `lapce-app/src/download.rs`; proxy side gets a thin `get_url_async` in `lapce-proxy/src/lib.rs`
- `create_ext_action` is the only safe tokio-to-Floem bridge; never mutate `RwSignal` directly from tokio tasks
- One tokio runtime per binary; 2 worker threads; never create per-download runtimes

---

## Blockers

None currently.

---

## Session Continuity

To resume: read `.planning/ROADMAP.md` for phase structure and `.planning/REQUIREMENTS.md` for full requirement detail. Open questions above must be resolved before planning Phase 4.

---
*State initialized: 2026-06-07 after roadmap creation*
