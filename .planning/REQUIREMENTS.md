# Requirements: Lapce Hardening Fork

**Defined:** 2026-06-07
**Core Value:** The editor must never panic on normal user actions, and every binary it downloads (plugin, self-update, remote proxy) must be integrity-verified before execution.

## v1 Requirements

Requirements for this hardening milestone. Each maps to a roadmap phase. Ordered to respect the research-derived build dependency: **deps → runtime → call-site migration → integrity → perf**, with crash fixes runnable in parallel.

### Dependencies (foundation — blocks runtime & security work)

- [ ] **DEPS-01**: `reqwest` upgraded 0.11 → 0.12.28 (not 0.13) across the workspace
- [ ] **DEPS-02**: `tokio` added to the workspace as a shared dependency
- [ ] **DEPS-03**: `zip` upgraded 0.6.6 → 2.x to remediate CVE-2025-29787 (symlink path traversal)
- [ ] **DEPS-04**: `interprocess` upgraded 1.2.1 → 2.x with updated socket name-API call sites in `app.rs`
- [ ] **DEPS-05**: `toml` wildcard `"*"` pinned to a specific major version (`Cargo.toml:65`)
- [ ] **DEPS-06**: Git-SHA-pinned deps (`floem`, `tracing`, `alacritty_terminal`, `psp-types`) moved to tagged releases where a tag exists
- [ ] **DEPS-07**: `sha2` promoted to a workspace dependency so `lapce-proxy` can reuse it

### Async Runtime (performance enabler — blocks integrity pipeline)

- [ ] **RT-01**: A `tokio` multi-thread runtime is constructed at each binary entry (`lapce-app`, `lapce-proxy`) and held alive via an `rt.enter()` guard — no `#[tokio::main]`, no nested runtime
- [x] **RT-02**: All network I/O is migrated off `reqwest::blocking`; no blocking download call site remains once the runtime is active
- [x] **RT-03**: A shared `DownloadPipeline` component (`lapce-app/src/download.rs`) wraps the async `reqwest::Client`; the three app-side call sites (update, plugin, proxy/remote) use it

### Crash / Stability (parallel-safe)

- [ ] **CRASH-01**: Compound keybinding conditions (AND/OR/NOT) evaluate without panicking (`keypress/condition.rs:95,104,108`)
- [ ] **CRASH-02**: Git operations with no open workspace fail gracefully instead of panicking (`dispatch.rs:1343`)
- [ ] **CRASH-03**: DAP server stdio-capture failure returns an error instead of panicking (`plugin/dap.rs:104,105`)
- [ ] **CRASH-04**: A malformed zstd plugin archive returns an error instead of panicking (`plugin/mod.rs:1590`)
- [ ] **CRASH-05**: Failed git operations surface to the user via RPC instead of being swallowed by `eprintln!` (`dispatch.rs:358,369,377,385`)

### Security Hardening (depends on RT-03 for download paths)

- [ ] **SEC-01**: Plugin downloads are SHA256-verified against a trusted manifest before unpacking (`plugin/mod.rs:1555-1600`); fails closed on mismatch
- [ ] **SEC-02**: App self-update archives are integrity-verified before applying (`update.rs:55-85`); fails closed
- [ ] **SEC-03**: The remote proxy binary is integrity-verified before execution (`proxy/remote.rs:341-360`); fails closed
- [ ] **SEC-04**: Plugin archive extraction rejects path-traversal and symlink-escape entries before writing to disk (`plugin/mod.rs:1592,1596`)
- [ ] **SEC-05**: `https_proxy` env var is scheme-validated (`http`/`https`) before use (`lapce-proxy/src/lib.rs:193`)

### Performance (caching + allocation — parallel-safe with runtime work)

- [ ] **PERF-01**: The compiled glob matcher is cached for directory listings, invalidated only when `files_exclude` changes (`file_explorer/data.rs:207`)
- [ ] **PERF-02**: Parsed font families are cached, invalidated only on `config.editor.font_family` change (`doc.rs:1951`)
- [ ] **PERF-03**: Completion/diagnostic updates invalidate only affected line ranges, not the entire text cache (`doc.rs:1139,1146,1430`)
- [ ] **PERF-04**: Clones in render hot paths are reduced via `Arc`/structural sharing where flamegraph profiling justifies it (`doc.rs`, `editor.rs`, `window_tab.rs`)
- [ ] **PERF-05**: Oversized message-enum variants are boxed instead of carried inline (`plugin/psp.rs`, `plugin/mod.rs`, `plugin/dap.rs`, `debug.rs`)

### Testing (cross-cutting)

- [ ] **TEST-01**: Every crash and security fix ships with a regression test that reproduces the original defect (for `unwrap()`→`?` fixes, the test asserts the error reaches the UI as a notification, not merely that no panic occurs)

## v2 Requirements

Deferred to a future milestone. Tracked but not in this roadmap.

### Features

- **FEAT-01**: Logging panel UI with runtime log-level control (`window_tab.rs:1587`)
- **FEAT-02**: `WrapStyle::WrapColumn` column-width wrap style (`config.rs:171`)
- **FEAT-03**: Markdown InlineHtml / InlineMath / DisplayMath rendering in hover docs (`markdown.rs:187-189`)
- **FEAT-04**: Palette next/previous page pagination (`palette.rs:1514,1518`)

### Refactor / Scaling

- **REF-01**: Extract IPC, update, and window-management out of monolithic `app.rs` (4,321 lines)
- **REF-02**: Isolate `screen_lines` computation out of `editor.rs`
- **REF-03**: Decompose `window_tab.rs` cross-cutting concerns into owned structs
- **REF-04**: Concurrent plugin-download pool; bounded channels with backpressure for high-frequency paths
- **REF-05**: Diff-view word-wrap + correct vline counting (`editor.rs:3622+`)

## Out of Scope

Explicitly excluded for this milestone. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Missing-feature work (logging panel, WrapColumn, Markdown math, palette paging) | New capability, not hardening — deferred to v2 (FEAT-*) |
| Large structural refactor of monolith files | High-risk; touch only as needed for fixes — deferred to v2 (REF-*) |
| `lsp-types` patch removal / upstreaming | Depends on external PR acceptance, out of our control |
| Custom PKI / code-signing infrastructure | Over-engineering; SHA256 from a trusted manifest is the table-stakes bar |
| Upstream PR submission | Fork-first; mergeability is secondary, not a deliverable |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEPS-01 | Phase 1 | Pending |
| DEPS-02 | Phase 1 | Pending |
| DEPS-03 | Phase 1 | Pending |
| DEPS-04 | Phase 1 | Pending |
| DEPS-05 | Phase 1 | Pending |
| DEPS-06 | Phase 1 | Pending |
| DEPS-07 | Phase 1 | Pending |
| RT-01 | Phase 2 | Pending |
| RT-02 | Phase 3 | Complete |
| RT-03 | Phase 3 | Complete |
| CRASH-01 | Phase 3 | Pending |
| CRASH-02 | Phase 3 | Pending |
| CRASH-03 | Phase 3 | Pending |
| CRASH-04 | Phase 3 | Pending |
| CRASH-05 | Phase 3 | Pending |
| SEC-01 | Phase 4 | Pending |
| SEC-02 | Phase 4 | Pending |
| SEC-03 | Phase 4 | Pending |
| SEC-04 | Phase 4 | Pending |
| SEC-05 | Phase 4 | Pending |
| PERF-01 | Phase 5 | Pending |
| PERF-02 | Phase 5 | Pending |
| PERF-03 | Phase 5 | Pending |
| PERF-04 | Phase 5 | Pending |
| PERF-05 | Phase 5 | Pending |
| TEST-01 | Phases 3+4 | Pending (folded into crash/security phases) |

**Coverage:**
- v1 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0

---
*Requirements defined: 2026-06-07*
*Last updated: 2026-06-07 after roadmap creation*
