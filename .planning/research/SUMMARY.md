# Project Research Summary

**Project:** Lapce Hardening Fork
**Domain:** Brownfield Rust desktop code editor — stability, supply-chain integrity, async runtime, dependency hardening
**Researched:** 2026-06-07
**Confidence:** HIGH

## Executive Summary

This is a brownfield hardening milestone on an existing Rust codebase (Lapce editor: Floem GUI, `lapce-proxy` backend, LSP/DAP/plugin/terminal/SSH). The work is not greenfield product development — it resolves four specific engineering-quality clusters surfaced by a codebase audit: runtime panics, missing download integrity verification, blocking network I/O on OS threads, and fragile/insecure dependency pins. Experts build this type of work in strict dependency order: you cannot wire integrity verification before the async download pipeline exists, and you cannot build the async pipeline before the dependency upgrades compile.

The recommended approach is a five-cluster build sequence anchored by tokio 1.x (background runtime, not `#[tokio::main]`) and reqwest 0.12 (async, `blocking` feature removed). The single most important architectural decision is runtime placement: construct the tokio runtime before `floem::launch()`, hold the `rt.enter()` guard for process lifetime, and never use `#[tokio::main]` — Floem owns the main thread. All three download paths (plugin, self-update, remote proxy binary) converge on a shared `DownloadPipeline` abstraction with fail-closed SHA256 verification (download → verify in memory → extract; never extract-then-verify). The crash fixes (`unwrap()` → `Result`) are only meaningful if the error propagates to the UI via RPC — replacing `unwrap()` with `?` that is silently dropped at a thread boundary is not a fix.

The primary risk is the atomicity constraint on the blocking-to-async migration: `reqwest::blocking` and an active tokio context cannot coexist — if any blocking call site remains after `rt.enter()` is introduced, the first request on that path panics with "Cannot start a runtime from within a runtime". The mitigation is Cluster 1 (remove `blocking` feature) and Cluster 2 (introduce runtime) being executed together with a green `cargo build --workspace` gate before any logic changes land. The secondary risk is the open integrity gap: `plugins.lapce.dev` may not expose a SHA256 field in its API — this must be confirmed before Phase 4 (integrity verification) is planned.

---

## Key Findings

### Recommended Stack

The workspace needs six changes to reach a consistent, secure dependency baseline. All are mechanical except the reqwest/tokio migration which touches logic. Tokio 1.x is non-negotiable: reqwest 0.12 requires a tokio context and cannot be replaced with smol without rewriting the HTTP client. The floem `rfd-async-std` feature flag must be changed to `rfd-tokio` once tokio is introduced, or file dialogs will fail.

**Core technologies:**

- `tokio = "1"` (`rt-multi-thread`, `net`, `time`, `sync`, `macros` only; no `full`) — async executor for all network I/O; held as ambient context via `rt.enter()` guard; Floem retains main-thread ownership
- `reqwest = "0.12"` (remove `blocking` feature) — async HTTP for plugin/update/proxy downloads; 0.13 deferred due to cmake/aws-lc CI burden from its TLS backend switch
- `zip = "2"` (8.6.0) — mandatory CVE fix; `zip = "0.6.6"` is vulnerable to CVE-2025-29787 (symlink path traversal); no workaround exists at 0.6.x
- `sha2 = "0.10"` (already present; promote to workspace dep) — SHA256 integrity verification; pure Rust, zero new deps
- `interprocess = "2"` — IPC/single-instance detection; sync API preserved; `ToFsName` call sites in `app.rs` require mechanical migration
- `tracing`/`tracing-subscriber`/`tracing-log`/`tracing-appender` — replace git-SHA pins with versioned crates.io releases (0.1/0.3/0.2/0.2)

**Defer:** `reqwest = "0.13"` (cmake burden), `alacritty_terminal = "0.26"` (verify API compat first), floem pin-to-tag (verify 0.2.0 tag covers current SHA first), `toml = "1"` (API differences; pin to `"0.8"` now, upgrade later).

### Expected Features

All hardening features are internal quality improvements — no new user-facing capability. The prioritisation below reflects the audit's Core Value: stability and supply-chain safety first.

**Must have (table stakes for this milestone):**

- SHA256 verification on all three download paths (plugin, self-update, proxy binary), fail-closed — Core Value driver
- Path-traversal validation on plugin archive extraction (`zip` + `tar` paths)
- `unwrap()`/`unimplemented!()` to `Result` with error propagation to UI: `dispatch.rs:1343`, `dap.rs:104,105`, `plugin/mod.rs:1590`, `condition.rs:95,104,108`
- Git operation errors surfaced to user (`dispatch.rs:358,369,377,385`) — currently swallowed by `eprintln!`
- `https_proxy` env var scheme validation before passing to `reqwest::Proxy::all()`
- Async download pipeline (tokio + reqwest 0.12) — prerequisite for verification and progress UX
- Regression test per crash/security fix

**Should have (validated after core hardening):**

- Proxy binary version check before re-download (low complexity perf win)
- Compiled `GlobMatcher` cache in `file_explorer/data.rs:207`
- Parsed font families cache in `doc.rs:1951`

**Defer (v2+):**

- Cryptographic signature verification (minisign/ed25519) — requires registry-side key publication
- Plugin sandboxing (Wasm/WASI) — multi-month architectural work
- TUF-style trusted manifest — requires server infrastructure beyond this fork's control
- Concurrent plugin installs with bounded semaphore — first-launch UX, not a hardening concern

### Architecture Approach

The central architectural change is introducing a single tokio `Runtime` per binary (`lapce-app` and `lapce-proxy`), constructed in each binary's `main()` entry point before any framework launch, with the ambient context held via `rt.enter()`. All three download paths converge on a new `DownloadPipeline` helper in `lapce-app/src/download.rs` (and a parallel thin `get_url_async` in `lapce-proxy/src/lib.rs`) that encapsulates async fetch, optional SHA256 verification, and structured `DownloadError` types. Results flow back to Floem via `create_ext_action` — the only safe tokio-to-Floem bridge. The existing crossbeam-channel + JSON-RPC architecture between `lapce-app` and `lapce-proxy` is unchanged.

**Major components:**

1. **Tokio Runtime** (`lapce-app/src/bin/lapce.rs`, `lapce-proxy/src/bin/lapce-proxy.rs`) — one per binary; `new_multi_thread().worker_threads(2)`; ambient via `rt.enter()` guard; Floem keeps main thread
2. **DownloadPipeline** (`lapce-app/src/download.rs`) — shared async HTTP + SHA256 verification; fail-closed by type (`Result<Bytes, DownloadError>`; `Bytes` unreachable from `Err` branch)
3. **Integrity Verifier** (inline in `DownloadPipeline`) — `sha2 0.10`; hashes in-memory buffer before any disk write; no separate crate needed
4. **Migrated call sites** — `update.rs`, `proxy/remote.rs`, `plugin.rs` (app side); `lib.rs:get_url_async`, `plugin/mod.rs:download_volt` (proxy side); all replaced with `tokio::spawn` + pipeline
5. **Error propagation** (`dispatch.rs`, `dap.rs`, `plugin/mod.rs`) — `Result` returned up stack to RPC handler; error sent as notification to Floem UI; `eprintln!` sites eliminated

### Critical Pitfalls

1. **Nested runtime panic** — `reqwest::blocking` inside an active tokio context panics immediately. Remove the `blocking` feature from reqwest and introduce the runtime in the same commit batch. Gate with `grep -rn "reqwest::blocking"` returning zero.

2. **Verify-after-extract is fail-open** — extracting before hashing lets malicious entries touch the filesystem before the check fails. Enforce strictly: download to bytes buffer → hash in memory → verify → extract. The `Result` type enforces this if `DownloadPipeline::fetch_verified` is designed so `Bytes` are only reachable from the `Ok` branch.

3. **TOCTOU race on disk hash** — hashing a file re-read from disk after writing it creates a race window. Always hash the in-memory `Bytes` buffer, never `std::fs::read(&path)` after `std::fs::write(&path, &bytes)`.

4. **Hash fetched over same channel as binary** — fetching a `.sha256` companion file from the same CDN/GitHub origin provides no supply-chain protection. For plugins, use the registry API metadata hash. For the proxy binary, embed the expected hash at compile time.

5. **`unwrap()` to `?` without UI propagation** — replacing `unwrap()` with `?` in a function whose return value is discarded at the thread boundary is behaviourally identical to the original panic. The fix must include sending the error through the RPC response channel to a UI notification. Test criterion: assert the notification is received, not just that no panic occurs.

---

## Implications for Roadmap

Based on the five-cluster build dependency chain identified in ARCHITECTURE.md, the roadmap maps to four phases. The dependency chain is strict: each cluster unblocks the next.

### Phase 1: Dependency Foundation

**Rationale:** Nothing else can compile or be tested until the workspace builds cleanly with the target dependency versions. This is the highest-risk phase for unexpected compile breaks. Do it first, in isolation, with no logic changes — makes rollback safe.

**Delivers:** Green `cargo build --workspace` with reqwest 0.12, tokio in workspace deps, zip 2.x, interprocess 2.x, pinned tracing and toml. No behaviour change.

**Addresses:** All dependency upgrade requirements (`reqwest 0.11→0.12`, `interprocess 1.2.1→2.x`, `toml "*"→"0.8"`, git-SHA tracing pins, zip CVE upgrade, sha2 promoted to workspace dep).

**Avoids:** reqwest body-API regression (Pitfall 4), interprocess name-API panic (Pitfall 5), zip symlink escape CVE-2025-29787 (Pitfall 9).

**Research flag:** Standard patterns — STACK.md provides complete migration instructions for all deps. No additional research phase needed.

### Phase 2: Async Runtime Introduction

**Rationale:** The runtime must exist before any async call sites are written. This phase is infrastructure-only (entry-point changes only; no `tokio::spawn` calls yet). Minimal blast radius — if Floem main-thread interaction breaks, this single commit is the obvious culprit.

**Delivers:** `tokio::Runtime` ambient in both binaries via `rt.enter()` guard; floem feature changed from `rfd-async-std` to `rfd-tokio`; app launches and all existing behaviour works.

**Avoids:** Nested runtime panic (Pitfall 1), runtime dropped while tasks in flight (Pitfall 3), `#[tokio::main]` + Floem main-thread conflict.

**Research flag:** Standard patterns — tokio bridging docs and ARCHITECTURE.md fully specify the pattern. No additional research phase needed.

### Phase 3: Download Pipeline + Crash Fixes

**Rationale:** With the runtime ambient, all three download call sites can be migrated to async and the `DownloadPipeline` abstraction built. The crash fixes share the same prerequisite (async task infrastructure for sending notifications back to Floem) and the same test gate (regression test per fix).

**Delivers:** All download call sites on async reqwest 0.12; `blocking` feature dropped; `DownloadPipeline` with structured error types and `create_ext_action` bridges to Floem; five panic/unwrap sites converted to `Result` with user-visible errors; `condition.rs` `unimplemented!()` removed; git errors surfaced in UI; `https_proxy` scheme validation added.

**Addresses:** All crash/stability requirements and the performance network-I/O requirement.

**Avoids:** Blocking calls starving the tokio runtime (Pitfall 2; `spawn_blocking` for extract/hash), `unwrap()` to `?` without UI propagation (Pitfall 10).

**Research flag:** Confirm `create_ext_action` API surface in the current floem git SHA before coding. One targeted code read, not a full research phase.

### Phase 4: Integrity Verification + Archive Hardening

**Rationale:** Integrity verification is wired into `DownloadPipeline::fetch_verified` — the pipeline must exist first (Phase 3). This is the Core Value payoff phase.

**Delivers:** SHA256 verification on all three download paths (fail-closed, in-memory hash before any disk write); `ZipFile::enclosed_name()` guard in all zip extraction loops; tar entry path validation in `download_volt`; regression test for each.

**Addresses:** All security hardening requirements.

**Avoids:** Verify-after-extract fail-open (Pitfall 8), SHA256 TOCTOU (Pitfall 6), hash fetched over same channel as binary (Pitfall 7), zip slip CVE (Pitfall 9 — zip upgrade in Phase 1 is necessary but `enclosed_name()` call is also required).

**Research flag (open gap): Does `plugins.lapce.dev` return a SHA256 hash in its API response?** This is the only unresolved external dependency. Check with: `curl https://plugins.lapce.dev/api/v1/plugins/<id>/versions` and inspect the response schema. If no hash field exists, options are: (a) request registry maintainer add it, (b) use a `.sha256` companion URL from same origin (acceptable for accidental-corruption protection, not CDN-compromise protection), (c) scope Phase 4 to self-update + proxy binary only and treat plugin integrity as v2. This question must be answered before Phase 4 is planned.

### Phase 5: Performance Optimisation (opportunistic)

**Rationale:** Independent of the async migration; can be worked in parallel with Phases 2–4 or deferred. Low regression risk. Profile before any clone-reduction work in render hot paths.

**Delivers:** `GlobMatcher` cache in `file_explorer/data.rs:207`; font-family parse cache in `doc.rs:1951`; `spawn_blocking` wrappers confirmed for archive decompression and SHA256 hashing on large buffers.

**Research flag:** Standard patterns — no research phase needed. Profile with `cargo-flamegraph` before touching `doc.rs`/`editor.rs` clone paths.

---

### Phase Ordering Rationale

- Phase 1 before Phase 2: tokio dep must be present in workspace before `tokio::runtime::Builder` can be called in the entry point.
- Phase 2 before Phase 3: `rt.enter()` must be active before any `tokio::spawn` call site; otherwise spawn panics with "no reactor running".
- Phase 3 before Phase 4: integrity verification lives inside `DownloadPipeline::fetch_verified` — the pipeline must exist. Also, the `blocking` feature must be dropped (Phase 3 completion) before a download-verify integration test can run without nested-runtime panics.
- Phase 5 is independent: can be parallelised with Phases 2–4; no blocking dependencies.

### Research Flags

Phases needing external confirmation during planning:

- **Phase 4:** Confirm `plugins.lapce.dev` API response schema includes a `sha256` or `checksum` field. This is the single open gap that could change Phase 4 scope. Must be resolved before planning Phase 4.

Phases with standard, well-documented patterns (no additional research phase needed):

- **Phase 1:** All dep migration steps are fully specified in STACK.md with version rationale and API migration notes.
- **Phase 2:** Tokio bridging pattern is fully specified in ARCHITECTURE.md and official tokio docs.
- **Phase 3:** Download pipeline design and `create_ext_action` bridge are fully specified in ARCHITECTURE.md. Minor floem API surface confirmation recommended — not a full research phase.
- **Phase 5:** Standard profiling workflow.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against crates.io API; reqwest 0.13 TLS change cross-checked with official changelog; CVE-2025-29787 confirmed against Snyk advisory |
| Features | HIGH | Derived directly from CONCERNS.md audit with file:line references; competitor comparison grounded in public documentation |
| Architecture | HIGH | Build-order dependency chain derived from actual codebase call sites (read directly); tokio bridging pattern from official tokio docs; floem feature flags from floem Cargo.toml |
| Pitfalls | HIGH | All 10 pitfalls grounded in documented crate behaviour and lapce-specific codebase evidence with line numbers |

**Overall confidence:** HIGH

### Gaps to Address

- **`plugins.lapce.dev` SHA256 field (OPEN):** Registry API may not expose a checksum. Must be confirmed before Phase 4 planning. Resolution: `curl` the API and inspect the response schema.

- **`floem` git SHA vs 0.2.0 tag delta:** Verify whether rev `31fa8f4` post-dates the 0.2.0 crates.io tag before switching to a versioned pin. Resolution: `git log --oneline v0.2.0..31fa8f4...` on the floem repo. Low risk either way.

- **`alacritty_terminal` API compat at 0.26:** Attempt `alacritty_terminal = "0.26"` in Phase 1; if it fails to compile, fall back to `"0.25"` (0.25.1, Oct 2025) as the stable step.

---

## Sources

### Primary (HIGH confidence)

- `.planning/codebase/CONCERNS.md` — codebase audit; primary source for all file:line references
- `.planning/PROJECT.md` — Core Value, requirements, constraints, key decisions
- crates.io API — version verification (tokio 1.52.3, reqwest 0.12.28, interprocess 2.4.2, sha2 0.10.9, zip 8.6.0, tracing 0.1.44, alacritty_terminal 0.26.0, psp-types 0.1.1, floem 0.2.0)
- https://tokio.rs/tokio/topics/bridging — runtime-enter pattern, `block_on` nesting constraints, `spawn_blocking`
- https://github.com/zip-rs/zip2/security/advisories/GHSA-2rxp-6h9h-hm8j — CVE-2025-29787 scope and patch version
- floem `Cargo.toml` main branch — `rfd-tokio` / `rfd-async-std` feature flags; tokio optional dep features

### Secondary (MEDIUM confidence)

- https://seanmonstar.com/blog/reqwest-v013-rustls-default/ — reqwest 0.13 TLS backend switch rationale; form/query feature changes
- https://github.com/kotauskas/interprocess/blob/main/CHANGELOG.md — interprocess 2.x sync API preservation; name type API migration
- corrode.dev "The State of Async Rust: Runtimes" — smol vs tokio tradeoffs (cross-checked against reqwest docs)

### Tertiary (LOW confidence — needs validation)

- `plugins.lapce.dev` API response schema — SHA256 field presence unknown; must be confirmed before Phase 4 planning

---
*Research completed: 2026-06-07*
*Ready for roadmap: yes — pending resolution of plugins.lapce.dev SHA256 gap before Phase 4 planning*
