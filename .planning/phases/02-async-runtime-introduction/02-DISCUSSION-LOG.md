# Phase 2: Async Runtime Introduction - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-07
**Phase:** 2-async-runtime-introduction
**Areas discussed:** Runtime Placement, Runtime Configuration, Build-Failure Handling, Phase-3 Readiness

---

## Runtime Placement

| Option | Description | Selected |
|--------|-------------|----------|
| In bin/*.rs main() | Bind runtime + guard in the main() wrappers; holds for process lifetime, true entry point, leaves launch()/mainloop() signatures unchanged | ✓ |
| In launch()/mainloop() | Build runtime at the top of launch()/mainloop(); closer to use but less explicit lifetime | |

**User's choice:** In bin/*.rs main()
**Notes:** Guard must wrap the entire call into launch()/mainloop() so the ambient context covers the Floem loop / proxy loop.

---

## Runtime Configuration

| Option | Description | Selected |
|--------|-------------|----------|
| enable_all + named threads | new_multi_thread().enable_all() with thread_name (lapce-app-worker/lapce-proxy-worker), default worker count | ✓ |
| enable_all, default naming | enable_all() without explicit thread names | |
| Selective enable_io+time | Only IO + time drivers instead of enable_all() | |

**User's choice:** enable_all + named threads
**Notes:** Named threads aid stacktrace/profiler readability for the hardening fork; enable_all avoids missing-driver surprises in Phase 3.

---

## Build-Failure Handling

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-closed: message + exit | On Err: tracing::error + stderr message + non-zero exit; no panic | ✓ |
| .expect()-panic | Panic caught by existing panic_hook, written to logs dir | |

**User's choice:** Fail-closed: message + exit
**Notes:** Aligns with project core value "never panic". Note that at main() time (before launch()) tracing may not be initialized, so the stderr message is the reliable channel — emit both.

---

## Phase-3 Readiness

| Option | Description | Selected |
|--------|-------------|----------|
| Purely ambient, no handle | Only runtime + enter() guard, no Handle in shared state; matches RT-01 "ambient, unused" | ✓ |
| Handle in CommonData | Pre-stash a tokio Handle in shared state for Phase 3 to dock onto | |

**User's choice:** Purely ambient, no handle
**Notes:** Phase 3 will obtain the runtime via Handle::current() from within the entered context, or via a deliberate Phase-3 signature change.

---

## Claude's Discretion

- Exact ordering of runtime construction relative to other early main() setup (constrained by the guard wrapping the launch()/mainloop() call).
- Whether to factor runtime-build logic into a shared helper vs. inlining in each main().

## Deferred Ideas

- RT-02 (migrate off reqwest::blocking) and RT-03 (DownloadPipeline) — Phase 3.
- Worker thread count capping / runtime sizing — opportunistic, profiling-driven only.
- Stashing a runtime Handle in shared state — explicitly rejected for Phase 2; revisit in Phase 3.
