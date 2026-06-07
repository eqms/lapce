# Phase 2: Async Runtime Introduction - Context

**Gathered:** 2026-06-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Stand up a `tokio` multi-thread runtime that is **ambient but unused** in both binary entry points (`lapce-app`, `lapce-proxy`). The runtime is constructed at each binary entry and held alive for the process lifetime via an `rt.enter()` guard. No call sites are migrated to async, no network I/O changes, no behavior change — the editor must behave identically to Phase 1. This phase only makes the runtime *present* so Phase 3 can migrate blocking I/O onto it.

**In scope:** RT-01 only — runtime construction + `rt.enter()` guard in both binaries.
**Out of scope:** RT-02 (migrate off `reqwest::blocking`), RT-03 (`DownloadPipeline`) — both Phase 3. Any actual `.await` call sites.

</domain>

<decisions>
## Implementation Decisions

### Runtime Placement
- **D-01:** Construct the runtime and hold the `rt.enter()` guard in the thin `bin/*.rs` `main()` wrappers (`lapce-app/src/bin/lapce.rs`, `lapce-proxy/src/bin/lapce-proxy.rs`), not inside `launch()` / `mainloop()`. Pattern: `let rt = ...; let _guard = rt.enter(); launch();`. This guarantees the guard and the `Runtime` value live for the entire process lifetime, sits at the true entry point, and leaves the `launch()` / `mainloop()` signatures unchanged.
- **D-02:** Both the `Runtime` binding and the `EnterGuard` must be bound to named locals (e.g. `let _rt` / `let _guard`) so neither is dropped early. Dropping the `Runtime` shuts down worker threads; dropping the guard removes the ambient context.

### Runtime Configuration
- **D-03:** Use `tokio::runtime::Builder::new_multi_thread().enable_all()`. `enable_all()` (IO + time drivers) over selective drivers — avoids a missing-driver surprise when Phase 3 wires in HTTP/timeouts.
- **D-04:** Set explicit worker thread names via `.thread_name(...)` — `"lapce-app-worker"` for the GUI binary, `"lapce-proxy-worker"` for the proxy binary. Named threads improve stacktrace/profiler readability, which matters for a hardening fork.
- **D-05:** Worker thread count = tokio default (number of CPUs). No explicit cap in this phase.

### Build-Failure Handling
- **D-06:** Handle a `Runtime::new()` / `Builder::build()` error **fail-closed and gracefully**: log via `tracing::error!`, print a clear message to stderr, and exit the process with a non-zero code. Do NOT `.expect()` / panic. Rationale: aligns with the project core value ("never panic"); a runtime that cannot start makes the editor unusable anyway, so a clear diagnostic + clean exit beats a panic-hook trace. Note: the GUI binary calls `logging::panic_hook()` and `logging::logging()` inside `launch()`, so at `main()` time (before `launch()`) tracing may not yet be initialized — the stderr message is the reliable channel; emit both.

### Phase-3 Readiness
- **D-07:** Keep the runtime **purely ambient** — do NOT stash a `tokio::runtime::Handle` in `CommonData` or any shared state in this phase. Matches RT-01 ("ambient, unused, no call-site changes"). Phase 3 will obtain the runtime via `tokio::runtime::Handle::current()` from within the entered context, or receive a handle through a deliberate Phase-3 signature change.

### Claude's Discretion
- Exact ordering of runtime construction relative to other early `main()` setup (e.g., before vs. after `Cli::parse()` is invoked inside `launch()`/`mainloop()`) — the planner/executor decides, constrained by D-01 (guard must wrap the call into `launch()`/`mainloop()`).
- Whether to factor the runtime-build logic into a small shared helper (e.g., in `lapce-core` or a local `fn`) vs. inlining in each `main()` — both acceptable; a shared helper reduces duplication of the named-thread + fail-closed logic across the two binaries.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §"Async Runtime" — RT-01 (this phase), RT-02 / RT-03 (Phase 3 dependents, for awareness only). RT-01 text: "A `tokio` multi-thread runtime is constructed at each binary entry (`lapce-app`, `lapce-proxy`) and held alive via an `rt.enter()` guard — no `#[tokio::main]`, no nested runtime."

### Codebase Maps
- `.planning/codebase/ARCHITECTURE.md` — process model (UI app vs proxy as separate processes), threading constraints (Floem single-threaded main loop; heavy work offloaded via `crossbeam_channel`).
- `.planning/codebase/CONCERNS.md` — original audit motivating the async runtime (blocking `reqwest::blocking` network I/O).
- `.planning/codebase/CONVENTIONS.md` — error handling (`anyhow`, `?` propagation, `.expect()` only for programmer errors), logging (`tracing::error!`).

### Prior Phase
- `.planning/phases/01-dependency-foundation/01-SUMMARY.md` (via `01-01`/`01-02` summaries) — `tokio` 1.52.3 was added as a workspace dependency in Phase 1 (DEPS-02); it is available to both crates.

No external ADRs/specs — requirements fully captured in REQUIREMENTS.md + decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tokio` 1.52.3 is already a workspace dependency (added Phase 1, DEPS-02). Both `lapce-app` and `lapce-proxy` need to declare it in their `[dependencies]` with `workspace = true` plus the `rt-multi-thread` feature (and whatever `enable_all()` requires).
- `lapce-app/src/app/logging.rs` — `panic_hook()` and `logging()` exist; the GUI panic hook writes traces to the logs directory. Relevant to D-06's "tracing not yet up at main() time" note.

### Established Patterns
- Both binaries are thin wrappers: `lapce-app/src/bin/lapce.rs` → `app::launch()`; `lapce-proxy/src/bin/lapce-proxy.rs` → `mainloop()`. Adding the runtime in `main()` is a minimal, localized change.
- `launch()` (`lapce-app/src/app.rs`) does `Cli::parse()`, optional `--wait` child handling, panic-hook + logging init, font loading, then the Floem app loop. The `rt.enter()` guard in `main()` must wrap the entire `launch()` call so the context is ambient through the Floem loop.
- `mainloop()` (`lapce-proxy/src/lib.rs`) does `Cli::parse()` then runs the proxy loop. Guard must wrap the entire `mainloop()` call.
- Error-handling convention: `anyhow` + `?`; `.expect()` reserved for programmer errors. D-06's fail-closed path fits the convention (graceful, not panic).

### Integration Points
- `main()` in each binary is the only integration point this phase touches. No changes to `launch()`/`mainloop()` signatures, no shared-state changes (per D-07).
- Floem owns the main thread — the tokio multi-thread runtime's worker threads are separate; `rt.enter()` only sets the thread-local context so future `Handle::current()` calls resolve. This must not interfere with Floem's main-thread loop (success criterion: Floem retains main-thread ownership).

</code_context>

<specifics>
## Specific Ideas

- Thread names are explicit: `"lapce-app-worker"` (GUI) and `"lapce-proxy-worker"` (proxy) — distinguishable per binary.
- The verification for this phase is behavioral parity (editor launches without panic; LSP/DAP/plugins/terminal/SSH unchanged) plus three structural checks: (1) no `#[tokio::main]` anywhere in entry-point files, (2) runtime ambient via `rt.enter()` guard, (3) proxy binary also has the runtime ambient before `mainloop()`. A regression test asserting `Handle::try_current()` succeeds inside the entered context (and that no nested runtime is created) would catch accidental removal of the guard.

</specifics>

<deferred>
## Deferred Ideas

- Migrating network I/O off `reqwest::blocking` and building the shared `DownloadPipeline` — RT-02 / RT-03, **Phase 3**. Do not touch download call sites here.
- Capping worker thread count / tuning runtime sizing — opportunistic, only if profiling later justifies it (out of scope per PROJECT.md scaling note).
- Stashing a runtime `Handle` in shared state — explicitly rejected for Phase 2 (D-07); revisit as a deliberate Phase 3 design choice.

None of these change Phase 2 scope.

</deferred>

---

*Phase: 2-async-runtime-introduction*
*Context gathered: 2026-06-07*
