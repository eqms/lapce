# Lapce Hardening Fork

## What This Is

A hardening-focused fork of [Lapce](https://github.com/lapce/lapce), the Rust-native code editor (Floem GUI, `lapce-proxy` backend, LSP/DAP support, plugins, terminal, remote SSH). This project systematically resolves the engineering-quality concerns surfaced in the codebase audit (`.planning/codebase/CONCERNS.md`): runtime panics, missing download integrity verification, performance bottlenecks, and outdated/unsafe dependency pins. The audience is the maintainer of this fork — improvements may later be offered upstream, but mergeability is a secondary goal.

## Core Value

The editor must never panic on normal user actions, and every binary it downloads (plugin, self-update, remote proxy) must be integrity-verified before execution. Stability and supply-chain safety come first; everything else is secondary.

## Requirements

### Validated

<!-- Existing capabilities inferred from the codebase — already shipped and relied upon. -->

- ✓ Text editor with multi-tab/multi-window editing (`lapce-app/src/editor.rs`, `doc.rs`) — existing
- ✓ LSP integration: completion, diagnostics, hover, inlay hints (`lapce-app/src/completion.rs`, `lapce-proxy/src/plugin/`) — existing
- ✓ DAP debugging support (`lapce-proxy/src/plugin/dap.rs`) — existing
- ✓ Plugin system with registry download/install from `plugins.lapce.dev` (`lapce-proxy/src/plugin/mod.rs`) — existing
- ✓ Integrated terminal via `alacritty_terminal` (`lapce-app/src/terminal/`) — existing
- ✓ Source control / git operations (`lapce-proxy/src/dispatch.rs`) — existing
- ✓ Remote SSH workspace via downloadable proxy binary (`lapce-app/src/proxy/remote.rs`) — existing
- ✓ File explorer, command palette, themes, self-update (`lapce-app/src/file_explorer/`, `palette.rs`, `update.rs`) — existing

**Dependency Foundation** — Validated in Phase 1: Dependency Foundation (2026-06-07)
- ✓ `reqwest` upgraded 0.11 → 0.12.28 for lapce-controlled crates (DEPS-01); `reqwest 0.11` remains only as a transitive dep of the external `wasi-experimental-http-wasmtime` git dep
- ✓ `tokio` 1.52.3 added as workspace dependency (DEPS-02)
- ✓ `zip` upgraded 0.6 → 2.4.2, closing CVE-2025-29787 — `cargo tree -i zip` shows 2.x only (DEPS-03)
- ✓ `interprocess` upgraded 1.2.1 → 2.4.2 with rewritten IPC call sites (`app.rs`, `lapce-proxy/src/lib.rs`, `cli.rs`); single-instance detection verified via `single_instance_ipc_roundtrip` test (DEPS-04)
- ✓ `toml` wildcard `"*"` pinned to `"0.8"` (DEPS-05)
- ✓ `tracing` family and `alacritty_terminal` moved from git revs to versioned crates.io releases (DEPS-06); `floem` retained on git rev `31fa8f4` per documented fallback (crates.io 0.2.0 API-incompatible)
- ✓ `sha2` promoted to workspace dependency (DEPS-07)
- ✓ Workspace builds cleanly (`cargo build --workspace` exit 0); zip-slip path-traversal guard added with `zip_slip_traversal_rejected` regression test
- ⏳ Runtime behaviour parity (LSP/DAP/plugins/terminal/SSH) pending human verification — tracked in `01-HUMAN-UAT.md`

**Async Runtime Introduction** — Validated in Phase 2: Async Runtime Introduction (2026-06-07)
- ✓ A tokio multi-thread runtime is ambient in both binaries via the named-local `_rt` + `_guard = _rt.enter()` pattern, wrapping `app::launch()` (GUI) and `mainloop()` (proxy) for the full process lifetime (RT-01)
- ✓ Runtime built with `Builder::new_multi_thread().enable_all()`, named worker threads (`lapce-app-worker` / `lapce-proxy-worker`), tokio-default worker count; build failure fails closed (`eprintln!` + `process::exit(1)`, no panic)
- ✓ No `#[tokio::main]` in either entry point; no `Handle` stored in shared state — runtime is purely ambient, no call sites changed (Phase 3 will migrate blocking I/O onto it)
- ✓ Regression test `handle_current_succeeds_inside_entered_context` (`lapce-app/src/runtime_tests.rs`); editor launch + behaviour parity confirmed by maintainer smoke test
- ⓘ Follow-up (non-blocking): the regression test validates the tokio contract but does not structurally guard the binary's `rt.enter()` — code review WR-01 suggests a `debug_assert!(Handle::try_current().is_ok())` in `app::launch()` for a true entry-point guard

**Download Pipeline + Crash Fixes** — Validated in Phase 3: Download Pipeline + Crash Fixes (2026-06-08)
- ✓ All 11 `reqwest::blocking` call sites migrated onto the Phase-2 tokio runtime via an async `get_url_async` core + sync `get_url` shim (`Handle::current().block_on`); `blocking` Cargo feature dropped in the same atomic commit — `grep -rn "reqwest::blocking"` returns zero workspace-wide (RT-02)
- ✓ `DownloadPipeline` wrapper added in `lapce-app/src/download.rs`, delegating to the shared proxy-side core (RT-03)
- ✓ Compound keybinding conditions evaluate without panicking — regression tests lock unknown→false / !unknown→true; load-time `tracing::warn!` for unparseable `when` tokens (CRASH-01)
- ✓ Git operations with no open workspace no longer panic — `let-else` guard in the background fs-event handler (CRASH-02)
- ✓ DAP stdio-capture failure returns an error surfaced via `ShowMessage` instead of panicking (CRASH-03)
- ✓ Malformed zstd plugin archive returns an error surfaced to the UI instead of panicking (CRASH-04)
- ✓ Failed git operations surface via `ShowMessage` ERROR instead of `eprintln!` swallowing; user-triggered git arms also emit "No folder open" when no workspace is open (CRASH-05)
- ✓ Each of the five crash fixes ships a regression test asserting the error reaches the UI as a notification, via the `CoreRpcHandler::new()` + `rx().recv_timeout()` zero-mock seam (TEST-01)
- ✓ Fail-closed gap fixed mid-verification: SSH remote proxy download now returns `Err` on a non-2xx response instead of continuing with an unwritten binary (`proxy/remote.rs:361-367`)
- ⏳ SSH remote proxy bootstrap fail-closed behaviour pending human verification — tracked in `03-HUMAN-UAT.md`
- ⓘ Follow-up (non-blocking, before Phase 4): CR-02 `get_url` shim should move to `tokio::task::block_in_place`; CR-01/CR-04 `target_commitish[..7]` slice panics (`update.rs`, `grammars.rs`) and `.expect()` calls in `remote.rs` are out-of-scope tech debt

### Active

<!-- This milestone: resolve the four engineering-quality concern clusters. Hypotheses until shipped. -->

**Security Hardening**
- [ ] Plugin downloads verified against a published SHA256 before unpacking (`plugin/mod.rs:1555-1600`)
- [ ] App self-update archives integrity-verified before applying (`update.rs:55-85`)
- [ ] Remote proxy binary integrity-verified before execution (`proxy/remote.rs:341-360`)
- [ ] Plugin archive extraction rejects path-traversal entries (`plugin/mod.rs:1592,1596`)
- [ ] `https_proxy` env var validated (scheme check) before use (`lapce-proxy/src/lib.rs:193`)

**Performance**
- [ ] Compiled glob matcher cached for directory listings (`file_explorer/data.rs:207`)
- [ ] Parsed font families cached, invalidated only on config change (`doc.rs:1951`)
- [ ] Granular per-line cache invalidation for completion/diagnostic updates (`doc.rs:1139,1146,1430`)
- [ ] Clone reduction in render hot paths via `Arc`/structural sharing where profiling justifies it (`doc.rs`, `editor.rs`, `window_tab.rs`)

**Testing (cross-cutting)**
- [ ] Each crash and security fix ships with a regression test that would have caught the original defect

### Out of Scope

- Missing-feature work (logging panel, WrapColumn wrap style, Markdown math/HTML rendering, palette pagination) — separate feature milestone; this milestone is hardening, not new capability
- Large structural refactor of monolith files (`app.rs` 4,321 lines, `editor.rs`, `window_tab.rs`) — high-risk module extraction deferred to a dedicated refactor milestone; touch only as needed for the fixes above
- Diff-view word-wrap and vline-count correctness (`editor.rs:3622+`) — feature-level rendering work, not a stability/security concern
- `lsp-types` patch removal / upstreaming — depends on external PR acceptance, out of our control
- Scaling concerns (concurrent plugin download pool, bounded channels) — opportunistic only; not a milestone goal
- Upstream PR submission — fork-first; mergeability is secondary, not a deliverable

## Context

- **Brownfield Rust workspace.** Crates: `lapce-app` (Floem reactive GUI — signals require owned data in closures, hence the 1,112 `.clone()` calls), `lapce-proxy` (backend: LSP/DAP/plugins/git/file-watcher), `lapce-core`, `lapce-rpc`. Full audit in `.planning/codebase/` (STACK, ARCHITECTURE, STRUCTURE, CONVENTIONS, TESTING, INTEGRATIONS, CONCERNS).
- **No async runtime today.** All network I/O is `reqwest::blocking` inside `std::thread::spawn`. Introducing tokio/smol is the chosen path (per Key Decisions) and is the largest architectural change in this milestone.
- **Near-zero test coverage.** `editor.rs`, `doc.rs`, `window_tab.rs`, `app.rs` have no unit tests; only `color_theme.rs`/`icon_theme.rs` do. Regression-test-per-fix is the agreed mitigation.
- **Supply-chain gap.** None of the three download paths (plugin, self-update, remote proxy) verify integrity — only HTTPS transport. This is the Core Value driver.
- **Dependency fragility.** Critical deps pinned to arbitrary git SHAs; `reqwest` on EOL 0.11/`hyper` 0.14; `toml` on wildcard `"*"`.

## Constraints

- **Tech stack**: Rust, Floem GUI, Cargo workspace — no language/framework change; fixes stay idiomatic to existing patterns (see `.planning/codebase/CONVENTIONS.md`).
- **Compatibility**: Must not break existing editor/LSP/DAP/plugin/terminal/remote behavior — these are Validated capabilities.
- **Security**: Integrity verification must fail-closed (reject + alert on mismatch), never fail-open.
- **Testing**: Every crash/security fix requires a reproducing regression test (Key Decision).
- **Dependencies**: `interprocess` 2.x and `reqwest` 0.12 upgrades carry API-migration risk; verify single-instance IPC and proxy handling still work after each bump.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Scope = 4 hardening clusters (crash, security, performance, deps); defer features & big refactors | Keep milestone coherent around engineering quality, not new capability | — Pending |
| Fork-first; upstream mergeability secondary | Captain wants freedom for larger architectural changes (async runtime) | — Pending |
| Adopt an async runtime (tokio/smol) for network I/O | Cleanest fix for blocking-HTTP bottleneck; biggest perf lever; fork allows the depth | — Pending |
| Regression test per crash/security fix | Codebase has near-zero tests; reproduce-then-fix prevents silent regressions | — Pending |
| Integrity verification fails closed (SHA256, reject on mismatch) | Supply-chain safety is the Core Value; fail-open would defeat the purpose | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-08 after Phase 3 (Download Pipeline + Crash Fixes) completion*
