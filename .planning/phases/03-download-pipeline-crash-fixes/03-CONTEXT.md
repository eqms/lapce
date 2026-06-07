# Phase 3: Download Pipeline + Crash Fixes - Context

**Gathered:** 2026-06-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Two coupled deliverables for this phase:

1. **Async download migration (RT-02, RT-03):** Migrate *all* network I/O off `reqwest::blocking` onto the Phase-2 tokio runtime. After this phase `grep -rn "reqwest::blocking"` returns zero across the workspace. A shared async download core replaces the single blocking helper `lapce_proxy::get_url`; a `DownloadPipeline` wrapper is introduced for the app-side call sites.

2. **Crash / stability fixes (CRASH-01..05) + regression tests (TEST-01):** Eliminate the five audited panic/error-swallowing sites; failures reach the user as notifications. Each fix ships a regression test that asserts the error surfaces (not merely "no panic").

**In scope:** RT-02, RT-03, CRASH-01, CRASH-02, CRASH-03, CRASH-04, CRASH-05, TEST-01 (regression test per crash fix).
**Out of scope:** SHA256 integrity verification (SEC-01..03 — Phase 4), path-traversal/symlink guards in archive extraction (SEC-04 — Phase 4), `https_proxy` scheme validation (SEC-05 — Phase 4), all performance/caching work (PERF-* — Phase 5). Touch download/extraction code only as needed to migrate to async and stop panicking — do NOT add integrity checks here (Phase 4 builds on these call sites).

</domain>

<decisions>
## Implementation Decisions

### Download Migration Scope & Architecture
- **D-01:** Migrate **every** `get_url` caller in this phase — not only the three app-side sites named in RT-03 (update, plugin, proxy/remote). The proxy-side `download_volt` (`lapce-proxy/src/plugin/mod.rs:1561,1569`) and `lapce-app/src/app/grammars.rs` (lines 14, 113) are also migrated. Rationale: Success Criterion #1 requires **zero** `reqwest::blocking` workspace-wide; leaving any blocking island fails it.
- **D-02:** The async HTTP core **stays in `lapce-proxy`** (where `get_url` lives today). `get_url` is replaced by an async implementation in `lapce-proxy`. The `DownloadPipeline` introduced in `lapce-app/src/download.rs` (per RT-03) is a **thin wrapper** around that shared proxy-side core — it does NOT spin up a second independent `reqwest::Client` codepath. Rationale: the proxy is a separate process and cannot consume an app-side pipeline; `lapce-app` already depends on `lapce-proxy` (e.g. `update.rs` calls `lapce_proxy::get_url`), so a shared core in `lapce-proxy` is dependency-clean and avoids duplication.
  - **Planner note:** RT-03's literal wording ("`DownloadPipeline` ... wraps the async `reqwest::Client`") is satisfied by wrapping the shared proxy-side async core, not by creating a fresh client. This is a deliberate, recorded interpretation — not a deviation to flag.
- **D-03:** Bridge sync→async at the many sync call sites (proxy dispatch, app background threads) via `tokio::runtime::Handle::current().block_on(...)`, obtaining the ambient handle the Phase-2 `rt.enter()` guard makes available. Minimal-invasive: no `launch()`/`mainloop()` or call-site **signature** changes; blocking behaviour is preserved at the API boundary while only `reqwest` itself becomes async. Do NOT push `async`/`.await` up through call chains in this phase (larger blast radius, higher LSP/plugin/SSH regression risk).
  - This is the Phase-3 resolution of Phase-2 D-07 ("how does Phase 3 obtain the handle"): obtain via `Handle::current()`, no handle stashed in shared state.
- **Claude's discretion (recorded):** Preserve the existing transport semantics of `get_url` — 10s timeout and up-to-3 retry loop — in the async replacement unless the planner finds a reason to change them. Exact module/type names for the shared core and wrapper are the planner's call.

### Error Surfacing to UI (all five crash fixes)
- **D-04:** Use the existing `CoreNotification::ShowMessage { message: ShowMessageParams }` channel (`lapce-rpc/src/core.rs:78`) as the **single, uniform** channel for surfacing all proxy-side crash-fix errors to the UI. It already exists, is the LSP standard, the UI already renders it, and one notification type gives a single test seam.
- **D-05:** Severity for failed user-triggered operations (corrupt plugin archive, failed git op, DAP stdio capture failure) = `MessageType::ERROR`. These are real failures of actions the user initiated.

### CRASH-02 / CRASH-05 — git no-workspace & swallowed git errors
- **D-06:** The `.unwrap()` at `dispatch.rs:1343` is inside `handle_workspace_fs_event` — a **background filesystem-event handler** that spawns the git-diff polling thread, NOT a user-triggered git command. Fix = guard the `self.workspace` `Option` and **return early / no-op** when `None`. No toast from this path (a notification fired by an internal fs-event handler would be spurious).
- **D-07:** Criterion #4 ("git operation with no folder open surfaces a user-visible notification") is satisfied at the **user-triggered** git command arms (`dispatch.rs:358–385`: `GitCheckout`, `GitDiscardFilesChanges`, `GitDiscardWorkspaceChanges`, `GitInit`). When invoked with no open workspace these currently skip silently (`if let Some(workspace)` with no else); add an else-branch that emits `ShowMessage` "No folder open". This co-locates with CRASH-05.
- **D-08:** CRASH-05: replace the `eprintln!("{e:?}")` error-swallowing in the same git command arms (`dispatch.rs:359,369,377,385`) with `ShowMessage` (ERROR) so failed git operations reach the user instead of stderr.

### CRASH-01 — keybinding condition evaluation
- **D-09:** The current `check_condition` evaluator (`lapce-app/src/keypress.rs:524`) is **already panic-free** — unknown/unparseable condition tokens flow through `Condition::from_str(...)` → `Err` → handled (positive token → `false`, negated `!token` → `true`); `parse_first` byte-slices only at ASCII `||`/`&&` positions (always valid boundaries). The REQUIREMENTS line numbers `condition.rs:95,104,108` reference an older audit snapshot; upstream already refactored the panicking `.unwrap()`. **Phase work for CRASH-01 = the regression test (TEST-01) locking the non-panic guarantee, plus the load-time diagnostic below — not removing a live panic.**
- **D-10:** Malformed condition feedback: at **keymap load time** (`KeyMapLoader::load_from_str`), emit `tracing::warn!` for unparseable conditions (include the offending keymap token), consistent with the loader's existing `trace!`-level error handling. **Eval-time stays a silent skip** (no per-keystroke UI spam).
- **D-11:** Lock the eval-time semantics as a test contract: unknown condition → `false` (binding skipped); `!unknown` → `true` (permissive). The regression test asserts exactly this — **no behaviour change**, only protection against regression.

### CRASH-03 / CRASH-04 — DAP stdio & malformed zstd (pattern-covered, not deep-discussed)
- **D-12:** CRASH-04: replace `zstd::Decoder::new(&mut resp).unwrap()` (`plugin/mod.rs:1590`) with `?` propagation; the resulting error surfaces via the D-04 channel (or the existing plugin-install error path `VoltInstalling { error }`, planner's choice — both reach the UI). A malformed/corrupt archive must produce a notification, not a crash (Criterion #5).
- **D-13:** CRASH-03: replace the DAP stdio-capture `.unwrap()`s (`plugin/dap.rs:104,105`) with error returns surfaced via the same pattern. Failure returns an error instead of panicking.

### Claude's Discretion
- Whether CRASH-04's plugin error uses `ShowMessage` directly vs. the existing `VoltInstalling { error }` install-feedback path — both reach the UI; planner picks the one that fits the plugin-install flow.
- Exact test-seam mechanism for asserting "error reached the UI as a notification" (capturing the core RPC channel / mock handler). The maintainer deferred this (Regression-Test-Seam area not selected) — planner chooses, but the assertion MUST verify the notification is emitted, not merely that no panic occurs (TEST-01 / Criterion #6).
- Whether to factor the async download core + retry/timeout logic into a shared helper vs. inline — both acceptable.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` §"Async Runtime" (RT-02, RT-03) and §"Crash / Stability" (CRASH-01..05) and §"Testing" (TEST-01) — the authoritative requirement text and the audited file:line locations.
- `.planning/ROADMAP.md` §"Phase 3" — the six success criteria this phase is verified against (note Criterion #1 = zero `reqwest::blocking`; Criterion #4 reconciliation is D-06/D-07; Criterion #6 = error-reaches-UI test).

### Codebase Maps
- `.planning/codebase/CONCERNS.md` — the original audit motivating both halves (blocking I/O + the five panic/error-swallow sites).
- `.planning/codebase/CONVENTIONS.md` — error handling (`anyhow` + `?`, `.expect()` only for programmer errors), logging (`tracing::error!`/`warn!`). The `.unwrap()`→`?` fixes must follow these.
- `.planning/codebase/ARCHITECTURE.md` — process model (app vs proxy as separate processes — the reason the async core lives in `lapce-proxy`, D-02), threading constraints (Floem single-thread; offload via `crossbeam_channel`), and the `CoreNotification` UI-surfacing path.

### Prior Phase
- `.planning/phases/02-async-runtime-introduction/02-CONTEXT.md` — D-07 there (runtime is ambient, no `Handle` stashed; Phase 3 obtains it via `Handle::current()`). D-03 here is the direct continuation. The tokio runtime is already ambient in both binaries via `rt.enter()`.

No external ADRs/specs — requirements fully captured in REQUIREMENTS.md + the decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `lapce_proxy::get_url` (`lapce-proxy/src/lib.rs:196`) — the **single** blocking HTTP helper; replacing it async-side fixes every downstream call site at once. Current semantics to preserve: `https_proxy` env handling, 10s timeout, up-to-3 retry loop. (Note: `https_proxy` *scheme validation* is SEC-05 / Phase 4 — do not add it here, but be aware Phase 4 will extend this exact function.)
- `CoreNotification::ShowMessage { message: ShowMessageParams }` (`lapce-rpc/src/core.rs:78`) — ready-made UI-surfacing channel; `core_rpc` handles are already present in `Dispatcher` (`self.core_rpc`) and the plugin paths.
- `CoreNotification::VoltInstalling { error }` (`lapce-rpc/src/core.rs:298`) — existing plugin-install error feedback path, an alternative surface for CRASH-04.
- Phase-2 ambient tokio runtime — `Handle::current()` resolves inside both binaries; `block_on` is available at every sync call site (D-03).

### Established Patterns
- Single shared HTTP entry point: all of `update.rs`, `plugin.rs`, `grammars.rs`, `proxy/remote.rs` (app) and `plugin/mod.rs` (proxy) funnel through `get_url`. Migration is centralised, call sites mostly just keep calling the (now async-backed, `block_on`-wrapped) helper.
- Git command arms in `dispatch.rs` already gate on `if let Some(workspace)` — D-07 only adds the missing else-branch; D-08 only swaps `eprintln!` for `ShowMessage`.
- `KeyMapLoader::load_from_str` already does `trace!(TraceLevel::ERROR, ...)` on load failures — D-10's load-time `warn!` is consistent with this.
- Error-handling convention: `anyhow` + `?`; the `.unwrap()`/`.expect("request failed")` (e.g. `proxy/remote.rs:353`) sites convert to `?` propagation.

### Integration Points
- `lapce-proxy/src/lib.rs` (get_url → async core) — the keystone change; everything else depends on it.
- `lapce-app/src/download.rs` (NEW) — thin `DownloadPipeline` wrapper (RT-03).
- Call sites to migrate: `lapce-app/src/update.rs` (33, 75), `lapce-app/src/plugin.rs` (298, 433, 460, 474), `lapce-app/src/app/grammars.rs` (14, 113), `lapce-app/src/proxy/remote.rs` (353), `lapce-proxy/src/plugin/mod.rs` (1561, 1569).
- Panic/error sites: `keypress.rs:524` (+ load-time warn in keymap loader), `dispatch.rs:1343` (guard), `dispatch.rs:358–385` (no-workspace notify + un-swallow errors), `plugin/dap.rs:104,105`, `plugin/mod.rs:1590`.

</code_context>

<specifics>
## Specific Ideas

- "Zero blocking island" is the literal bar for the migration: `grep -rn "reqwest::blocking"` must return nothing in lapce-controlled crates. (The only remaining `reqwest 0.11` is a transitive dep of the external `wasi-experimental-http-wasmtime` git dep — out of our control, not a `reqwest::blocking` call site.)
- The CRASH-02/Criterion-#4 split (background fs-handler = silent guard; user git commands = visible "No folder open") is the deliberate reconciliation of the audit's panic location with the roadmap's UX wording. The `gsd-verifier` checks Criterion #4 against the *user-triggered* git arms, not line 1343.
- Every crash fix's test must assert the **notification is emitted**, not just absence of panic (TEST-01 / Criterion #6).

</specifics>

<deferred>
## Deferred Ideas

- **SHA256 integrity verification** of plugin/self-update/proxy downloads (SEC-01..03) — **Phase 4**. Phase 4 extends the exact call sites this phase migrates to async; do not add hashing here.
- **Path-traversal / symlink-escape guards** in plugin archive extraction (SEC-04) — **Phase 4**. The zstd/gz `archive.unpack(&plugin_dir)` paths (`plugin/mod.rs:1590–1596`) get hardened there; this phase only stops the `.unwrap()` panic.
- **`https_proxy` scheme validation** (SEC-05) — **Phase 4**, extends `get_url`/its async successor.
- **User-notification (vs. log) for malformed keymap conditions** — considered (D-10 chose load-time `warn!` to log); a visible prompt was rejected as too intrusive. Revisit only if users report silent keymap breakage.
- **Pushing async up through call chains / concurrent download pool / bounded channels** — explicitly rejected for this phase (D-03 keeps `block_on`); REF-04 / v2 scaling work.

None of these change Phase 3 scope.

</deferred>

---

*Phase: 3-download-pipeline-crash-fixes*
*Context gathered: 2026-06-07*
