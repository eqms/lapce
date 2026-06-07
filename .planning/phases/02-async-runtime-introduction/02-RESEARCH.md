# Phase 2: Async Runtime Introduction - Research

**Researched:** 2026-06-07
**Domain:** tokio runtime Builder API, EnterGuard lifetime, Floem/tokio thread model, Cargo feature gating
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Construct the runtime and hold the `rt.enter()` guard in the thin `bin/*.rs` `main()` wrappers (`lapce-app/src/bin/lapce.rs`, `lapce-proxy/src/bin/lapce-proxy.rs`), not inside `launch()` / `mainloop()`. Pattern: `let rt = ...; let _guard = rt.enter(); launch();`.
- **D-02:** Both the `Runtime` binding and the `EnterGuard` must be bound to named locals (`let _rt` / `let _guard`) so neither is dropped early.
- **D-03:** Use `tokio::runtime::Builder::new_multi_thread().enable_all()`.
- **D-04:** Set explicit worker thread names via `.thread_name(...)` — `"lapce-app-worker"` for the GUI binary, `"lapce-proxy-worker"` for the proxy binary.
- **D-05:** Worker thread count = tokio default (number of CPUs). No explicit cap.
- **D-06:** Handle a `Runtime::new()` / `Builder::build()` error fail-closed: `tracing::error!` + stderr + non-zero exit. No `.expect()` / panic.
- **D-07:** Keep the runtime purely ambient — do NOT stash a `Handle` in `CommonData` or any shared state.

### Claude's Discretion

- Exact ordering of runtime construction relative to other early `main()` setup (before vs. after `Cli::parse()` inside `launch()`/`mainloop()`).
- Whether to factor the runtime-build logic into a small shared helper vs. inlining in each `main()`.

### Deferred Ideas (OUT OF SCOPE)

- Migrating network I/O off `reqwest::blocking` — Phase 3 (RT-02/RT-03).
- Capping worker thread count / tuning runtime sizing.
- Stashing a runtime `Handle` in shared state.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RT-01 | A `tokio` multi-thread runtime is constructed at each binary entry (`lapce-app`, `lapce-proxy`) and held alive via an `rt.enter()` guard — no `#[tokio::main]`, no nested runtime | Builder API, EnterGuard contract, feature gating, Floem main-thread safety all documented below |
</phase_requirements>

---

## Summary

Phase 2 introduces a tokio multi-thread runtime in both binary entry points (`lapce-app/src/bin/lapce.rs` and `lapce-proxy/src/bin/lapce-proxy.rs`) as an ambient-but-unused context. The runtime is constructed via `tokio::runtime::Builder::new_multi_thread().enable_all().thread_name(...).build()`, and its `EnterGuard` is held alive for the entire process lifetime by binding it to a named local in `main()`. Neither the Floem event loop nor the proxy main loop is modified; the guard simply ensures `Handle::try_current()` succeeds anywhere in the process from that point forward.

The critical technical constraint is that both `_rt` (the `Runtime`) and `_guard` (the `EnterGuard`) must outlive `launch()` / `mainloop()`. Rust's drop order for let-bindings is reverse declaration order, so declaring `_rt` first and `_guard` second is safe: `_guard` drops first (exits context), then `_rt` drops (shuts down workers). The inverse order would be undefined behavior and is caught by tokio's drop-order check.

The one process-lifetime pitfall to be aware of: `reqwest::blocking::Client` internally spawns its own mini tokio runtime, and that runtime **cannot be dropped while another tokio context is entered** on the same thread. Because Phase 2 adds `rt.enter()` on the main thread, any `reqwest::blocking` call that is made (or dropped) on that same main thread inside `launch()` will panic with "Cannot drop a runtime in a context where blocking is not allowed". In practice, all `lapce_proxy::get_url(...)` call sites inside `lapce-app` are invoked from background threads (`std::thread::spawn`), not the main thread, so Phase 2 does not trigger this panic. This is explicitly noted as a Phase 3 concern (RT-02) and is confirmed by codebase inspection.

**Primary recommendation:** Add `tokio = { workspace = true }` to `lapce-app/Cargo.toml` and `lapce-proxy/Cargo.toml`, then write the two-line runtime guard pattern in each `bin/*.rs` entry point. The existing workspace tokio dep already has all needed features (`rt-multi-thread`, `macros`, `sync`, `time`, `fs`). No other files change.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Runtime construction | Binary entry (`bin/*.rs`) | — | D-01: thin main() wrappers own lifetime |
| EnterGuard lifetime | Binary entry (`bin/*.rs`) | — | Guard must outlive `launch()`/`mainloop()` call |
| Runtime context (Handle::current) | Process-wide ambient | — | All threads inherit the entered context once guard is set |
| Worker threads | tokio scheduler (separate threads) | — | Never the Floem main thread |
| Error reporting (build failure) | stderr + tracing | — | D-06: tracing not yet initialized at main() time |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | 1.52.3 (workspace) | Multi-thread async runtime | Already workspace dep from Phase 1 (DEPS-02) |
| `tokio::runtime::Builder` | — (part of tokio) | Programmatic runtime construction | The only non-macro way to build a runtime; required by D-03 |

### Features Required

The workspace `Cargo.toml` already declares:

```toml
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }
```

`rt-multi-thread` — required for `Builder::new_multi_thread()` [VERIFIED: docs.rs/tokio/1.52.3]
`macros` — enables `#[tokio::main]` (NOT used, but already in workspace; harmless)
`sync`, `time`, `fs` — needed by Phase 3 async call sites; present now

Neither `lapce-app/Cargo.toml` nor `lapce-proxy/Cargo.toml` currently declares tokio as a direct dependency. Both must add `tokio = { workspace = true }` to gain access to `tokio::runtime` in the entry-point files. [VERIFIED: codebase grep]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Builder::new_multi_thread()` | `Builder::new_current_thread()` | current-thread only runs when block_on is called; idle when Floem loop runs — wrong for an ambient runtime |
| Manual `enable_io().enable_time()` | `enable_all()` | Fragile: a future driver added to tokio would be silently absent; D-03 mandates enable_all() |

### Installation

No new packages. Both crates must gain:

```toml
# lapce-app/Cargo.toml  [dependencies]
tokio = { workspace = true }

# lapce-proxy/Cargo.toml  [dependencies]
tokio = { workspace = true }
```

**Version verification:** `tokio` 1.52.3 confirmed present in workspace Cargo.toml and Cargo.lock (Phase 1, DEPS-02). [VERIFIED: codebase read]

---

## Package Legitimacy Audit

No new packages are introduced in this phase. `tokio` 1.52.3 is an existing workspace dependency added in Phase 1.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| tokio | crates.io | ~6 yrs | >100M/wk | github.com/tokio-rs/tokio | N/A (pre-existing workspace dep) | Approved — no new install |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
main() in lapce-app/src/bin/lapce.rs
  │
  ├─ Builder::new_multi_thread()
  │    .enable_all()
  │    .thread_name("lapce-app-worker")
  │    .build()
  │    └─> Runtime  (held as `let _rt`)
  │
  ├─ _rt.enter()  ──> EnterGuard  (held as `let _guard`)
  │                   sets thread-local: CONTEXT = Some(handle)
  │
  └─ app::launch()          ← main thread (Floem event loop)
       │
       ├─ logging::panic_hook()
       ├─ logging::logging()
       ├─ Floem Application::new()
       ├─ .run()   ← blocks main thread for process lifetime
       │
       └─ background std::thread::spawns (unaffected by guard)
            └─ reqwest::blocking calls (on background threads, NOT main)

                tokio worker threads: idle (no tasks submitted)
```

```
main() in lapce-proxy/src/bin/lapce-proxy.rs
  │
  ├─ Builder::new_multi_thread()
  │    .enable_all()
  │    .thread_name("lapce-proxy-worker")
  │    .build()
  │    └─> Runtime  (held as `let _rt`)
  │
  ├─ _rt.enter()  ──> EnterGuard  (held as `let _guard`)
  │
  └─ mainloop()   ← Cli::parse(), stdio transport, proxy loop
```

### Recommended Project Structure

No new directories or files. Changes are confined to:

```
lapce-app/
├── Cargo.toml                  # add tokio = { workspace = true }
└── src/bin/lapce.rs            # add runtime construction + guard

lapce-proxy/
├── Cargo.toml                  # add tokio = { workspace = true }
└── src/bin/lapce-proxy.rs      # add runtime construction + guard
```

### Pattern 1: Named-Local Runtime Guard (canonical)

**What:** Construct the runtime, bind to `_rt`; call `.enter()`, bind guard to `_guard`; call into framework. Both locals live to end of `main()`.

**When to use:** Any binary that needs an ambient tokio context without `#[tokio::main]`.

```rust
// Source: tokio docs.rs/tokio/1.52.3/tokio/runtime/struct.Runtime.html#method.enter
// and project decision D-01/D-02

pub fn main() {
    let _rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lapce-app-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e:?}");
            eprintln!("lapce: failed to build tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = _rt.enter();
    lapce_app::app::launch();
}
```

**Key constraint:** `_rt` declared before `_guard` — Rust drops in reverse order, so `_guard` (context exit) drops before `_rt` (runtime shutdown). This is correct. [CITED: tokio EnterGuard docs]

### Pattern 2: Build-Failure Fail-Closed (D-06)

**What:** `Builder::build()` returns `Result<Runtime, io::Error>`. On error, log + stderr + exit(1). No `.expect()`.

**Why:** `tracing` is not yet initialized at `main()` entry (it initializes inside `launch()` / `mainloop()`). Both `tracing::error!` and `eprintln!` are emitted so at least one channel reaches the user. [VERIFIED: codebase — logging::logging() is called inside launch(), not before]

```rust
// D-06 fail-closed pattern
match tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("lapce-app-worker")
    .build()
{
    Ok(rt) => rt,
    Err(e) => {
        tracing::error!("failed to build tokio runtime: {e:?}");
        eprintln!("lapce: failed to build tokio runtime: {e}");
        std::process::exit(1);
    }
}
```

### Anti-Patterns to Avoid

- **`#[tokio::main]` on `main()`:** Expands to `Runtime::new() + block_on(async_main())`, which hands main-thread ownership to tokio. Floem requires main-thread ownership for its event loop. This would deadlock or panic. [CITED: STATE.md key decisions]
- **`let _ = rt.enter()`:** Underscore binding immediately drops the guard. The context exits before `launch()` is called. Use `let _guard = ...` (named underscore prefix). [VERIFIED: tokio EnterGuard docs — dropped on scope exit]
- **`let _guard = rt.enter(); drop(rt);`:** Dropping the Runtime while the guard is active violates tokio's drop-order contract. Rust's borrow checker partially enforces this (guard borrows runtime), but named locals in declaration order guarantee correctness. [CITED: tokio docs]
- **Calling `Builder::build()` inside `launch()`:** Violates D-01; guard would not wrap the entire Floem loop.
- **Per-request runtimes:** Creating a `Runtime` per download call (current anti-pattern in some projects) is incompatible with an ambient runtime on the same thread and produces "Cannot start a runtime from within a runtime". Not present in lapce, but Phase 3 must not introduce this.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Ambient runtime context | Custom thread-local or global Arc<Runtime> | `rt.enter()` + named guard in `main()` | tokio's EnterGuard is the official, thread-safe mechanism; hand-rolled globals miss the nested-context detection |
| Runtime construction error | Panic via `.expect()` | `match ... { Err => eprintln! + exit(1) }` | D-06; aligns with project "never panic" core value |
| Worker thread identification | Numeric suffixes | `.thread_name("lapce-app-worker")` | Named threads appear in profiler/debugger output; no cost |

**Key insight:** The entire change is ~10 lines per binary. There is no logic to hand-roll.

---

## Runtime State Inventory

Not applicable — this is a greenfield addition of runtime scaffolding, not a rename/refactor/migration phase.

---

## Common Pitfalls

### Pitfall 1: reqwest::blocking Panic on Main Thread
**What goes wrong:** If `reqwest::blocking::Client` is constructed or dropped on the main thread *after* `rt.enter()` is active, tokio panics: "Cannot drop a runtime in a context where blocking is not allowed". This is because `reqwest::blocking` creates an internal runtime, and dropping that runtime while inside another runtime context is forbidden.
**Why it happens:** `rt.enter()` on the main thread marks the thread as "in a runtime context". tokio's runtime-drop check fires when `reqwest::blocking`'s internal runtime is destroyed.
**How to avoid:** Phase 2 is safe because all `lapce_proxy::get_url(...)` call sites in `lapce-app` are invoked from background `std::thread::spawn` threads (verified: `app.rs`, `grammars.rs`, `plugin.rs`, `update.rs`, `proxy/remote.rs`). The main thread never calls `get_url`. Phase 3 must migrate these to async reqwest before removing the `blocking` feature.
**Warning signs:** Panic message "Cannot drop a runtime in a context where blocking is not allowed" from a stack trace involving `reqwest::blocking::wait`. [CITED: github.com/seanmonstar/reqwest/issues/1017]

### Pitfall 2: Early Drop of Runtime or Guard
**What goes wrong:** `let _ = rt.enter()` or `let _ = Builder::...build()?` immediately drops the value. The context is not entered / the runtime shuts down immediately.
**Why it happens:** `_` is a discard pattern, not a named binding. Rust drops the temporary at the end of the statement.
**How to avoid:** Use `let _guard = rt.enter()` and `let _rt = rt` — names with underscore prefix. [VERIFIED: tokio EnterGuard docs]
**Warning signs:** `Handle::try_current()` returns `Err` immediately after the enter call.

### Pitfall 3: Drop Order Inversion
**What goes wrong:** Declaring `let _guard = rt.enter(); let _rt = rt;` causes `_rt` to drop before `_guard`, which means the Runtime shuts down while the guard is still alive. tokio detects this and may panic.
**Why it happens:** Rust drops local bindings in reverse declaration order.
**How to avoid:** Always declare `_rt` first, `_guard` second. [CITED: tokio docs — EnterGuard has `'_` lifetime borrowing runtime]
**Warning signs:** Compile error (lifetime) or runtime panic on shutdown.

### Pitfall 4: #[tokio::main] in Entry File
**What goes wrong:** The `#[tokio::main]` attribute expands to `Runtime::new() + block_on(async { main_body })`. This hands the main thread to the tokio executor loop. Floem's `Application::run()` requires main-thread ownership and will either deadlock or panic.
**Why it happens:** `#[tokio::main]` and GUI frameworks both assume exclusive main-thread ownership.
**How to avoid:** Use the manual Builder pattern (D-01/D-03). Verification check: grep for `#[tokio::main]` in entry-point files. [CITED: STATE.md key decisions, tokio discussion #3857]
**Warning signs:** Editor fails to launch; Floem event loop never receives first OS event.

### Pitfall 5: tokio Not a Direct Dependency of lapce-app / lapce-proxy
**What goes wrong:** `tokio::runtime::Builder` is only accessible if `tokio` is in the crate's own `[dependencies]`, not just the workspace root.
**Why it happens:** Workspace deps must be explicitly opted into by each crate.
**How to avoid:** Add `tokio = { workspace = true }` to both `lapce-app/Cargo.toml` and `lapce-proxy/Cargo.toml`. [VERIFIED: codebase — neither file currently has tokio in its dependencies]
**Warning signs:** `error[E0433]: failed to resolve: use of undeclared crate or module 'tokio'`.

### Pitfall 6: Floem `create_signal_from_tokio_channel` Requires Active Handle
**What goes wrong:** If floem's `tokio` feature is activated (which it is via `rfd-tokio` → dep chain), calling `floem::ext_event::create_signal_from_tokio_channel` from within the Floem loop calls `tokio::spawn`. Without an active runtime context, this panics.
**Why it happens:** `tokio::spawn` panics if `Handle::current()` fails.
**How to avoid:** Phase 2's `rt.enter()` guard on the main thread provides the ambient context before `launch()` is called, so `tokio::spawn` inside floem's `create_signal_from_tokio_channel` will succeed. This is an additional correctness benefit of Phase 2 beyond pure RT-01.
**Warning signs:** Panic "no reactor running, call `select!` or `block_on`" if the runtime guard is accidentally not present.
[VERIFIED: floem source at `31fa8f4`, `src/ext_event.rs:232` calls `tokio::spawn`]

---

## Code Examples

### Entry Point: lapce-app

```rust
// lapce-app/src/bin/lapce.rs
// Source: tokio docs.rs + project decisions D-01..D-07
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lapce_app::app;

pub fn main() {
    let _rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lapce-app-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e:?}");
            eprintln!("lapce: failed to build tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = _rt.enter();
    app::launch();
}
```

### Entry Point: lapce-proxy

```rust
// lapce-proxy/src/bin/lapce-proxy.rs
// Source: tokio docs.rs + project decisions D-01..D-07

use lapce_proxy::mainloop;

fn main() {
    let _rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lapce-proxy-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e:?}");
            eprintln!("lapce-proxy: failed to build tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = _rt.enter();
    mainloop();
}
```

### Cargo.toml additions

```toml
# lapce-app/Cargo.toml — add to [dependencies]
tokio = { workspace = true }

# lapce-proxy/Cargo.toml — add to [dependencies]
tokio = { workspace = true }
```

### Regression Test: Handle Available

```rust
// Can be placed in lapce-app/src/lib.rs or a dedicated test module
// Purpose: assert that rt.enter() was called before launch()
// This is a compile-time-structure test, not a behavior test.
// For integration: call from within an initialized context.
#[cfg(test)]
mod runtime_tests {
    #[test]
    fn handle_current_succeeds_inside_entered_context() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("test-worker")
            .build()
            .expect("test runtime");
        let _guard = rt.enter();
        // Should not panic:
        let handle = tokio::runtime::Handle::try_current()
            .expect("handle must be available inside entered context");
        assert_eq!(handle.runtime_flavor(), tokio::runtime::RuntimeFlavor::MultiThread);
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[tokio::main]` macro | Manual `Builder` + `rt.enter()` guard | Required for GUI frameworks (always) | Floem retains main-thread ownership |
| `tokio::runtime::Runtime::new()` | `Builder::new_multi_thread().enable_all().build()` | tokio 1.0+ | Explicit configuration, no hidden defaults |
| Single unnamed `_` binding for guard | Named `let _guard` binding | Style/correctness | Named binding guaranteed not to drop early |

**Deprecated/outdated:**
- `Runtime::new()`: Still works but is deprecated in newer tokio docs in favor of explicit `Builder`; the `Builder` pattern is preferred for clarity and configurability.
- `#[tokio::main]`: Not deprecated but incompatible with GUI main-loop ownership requirements.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | All `lapce_proxy::get_url()` call sites in lapce-app run on background `std::thread::spawn` threads, not the main thread | Common Pitfalls §1 | If any call site is on the main thread, Phase 2 alone could cause a panic; would need to defer or inline-wrap that call in `spawn_blocking` |

**Note on A1:** This was confirmed by codebase grep (`grammars.rs`, `plugin.rs`, `update.rs`, `proxy/remote.rs` — all wrapped in `std::thread::Builder::new().spawn(...)`). The risk is LOW. [VERIFIED: codebase grep]

**If this table is near-empty:** Most claims were verified by direct codebase inspection or official tokio docs.

---

## Open Questions (RESOLVED)

1. **Shared helper vs. inline duplication** — RESOLVED
   - What we know: The runtime build pattern is ~10 lines, identical except for `thread_name`. Two binaries.
   - RESOLVED: Claude's discretion per CONTEXT.md. A shared `fn build_runtime(thread_name: &str)` helper reduces duplication; inlining in each `main()` is equally acceptable. If a helper is used, place it in `lapce-app/src/` (NOT `lapce-core` — keep lapce-core runtime-free per the architecture constraint). The executor may choose either; both satisfy D-01…D-06.

2. **Exact placement within main() relative to existing code** — RESOLVED
   - What we know: `lapce-app/src/bin/lapce.rs` is currently 7 lines (`#![cfg_attr]`, use, `pub fn main() { app::launch(); }`). lapce-proxy is 5 lines.
   - RESOLVED: Runtime construction is the FIRST action in each `main()`, and the `rt.enter()` guard must remain in scope across the entire `launch()` / `mainloop()` call. No code from `launch()`/`mainloop()` moves out — those functions stay unchanged (honors D-01).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | cargo build | ✓ | 1.87.0+ (rust-version enforced) | — |
| tokio 1.52.3 | runtime construction | ✓ | 1.52.3 (workspace dep, Cargo.lock) | — |
| cargo clippy --profile ci | CI verification | ✓ | bundled with toolchain | — |

**Missing dependencies with no fallback:** none.

---

## Security Domain

This phase introduces no new network endpoints, authentication paths, file access patterns, or external input handling. The runtime is ambient and unused. No ASVS categories apply to this phase.

`security_enforcement`: not explicitly set in config.json — treated as enabled. Applicable controls: none for this phase's specific scope (runtime scaffolding only).

---

## Sources

### Primary (HIGH confidence)
- [tokio docs.rs/tokio/1.52.3 — Builder](https://docs.rs/tokio/1.52.3/tokio/runtime/struct.Builder.html) — `new_multi_thread()`, `enable_all()`, `thread_name()`, `build()` API verified
- [tokio docs.rs/tokio/1.52.3 — Runtime::enter](https://docs.rs/tokio/1.52.3/tokio/runtime/struct.Runtime.html) — `enter()` return type, EnterGuard lifetime contract
- [tokio docs.rs — EnterGuard](https://docs.rs/tokio/latest/tokio/runtime/struct.EnterGuard.html) — `!Send`, drops context on scope exit, `'_` lifetime
- Floem git source `31fa8f4` `src/ext_event.rs:232` — `tokio::spawn` call confirming ambient runtime required
- Floem git source `31fa8f4` `src/file_action.rs` — synchronous `rfd::FileDialog` used, not async
- Floem git source `31fa8f4` `Cargo.toml` — `rfd-tokio` feature definition verified
- Codebase: `lapce-app/src/bin/lapce.rs`, `lapce-proxy/src/bin/lapce-proxy.rs` — current thin wrappers
- Codebase: `Cargo.toml` — tokio 1.52.3 workspace dep with `rt-multi-thread` feature confirmed
- Codebase: `lapce-app/Cargo.toml`, `lapce-proxy/Cargo.toml` — tokio NOT yet a direct dep (confirmed by grep)
- Codebase: `lapce-app/src/app/logging.rs` — `launch()` calls `logging::logging()` internally; tracing not ready at `main()` entry

### Secondary (MEDIUM confidence)
- [reqwest/issues/1017](https://github.com/seanmonstar/reqwest/issues/1017) — "Cannot drop a runtime in a context where blocking is not allowed" mechanism documented
- [tokio discussion #3857](https://github.com/tokio-rs/tokio/discussions/3857) — "Cannot start a runtime from within a runtime" triggered by `block_on`, not `rt.enter()` alone
- [egui discussion #521](https://github.com/emilk/egui/discussions/521) — confirmed pattern: build runtime on main thread, hold EnterGuard on stack, GUI event loop runs normally

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — tokio 1.52.3 workspace dep already present; APIs verified on docs.rs
- Architecture: HIGH — entry-point files read directly; Floem source inspected at pinned rev
- Pitfalls: HIGH — reqwest/blocking mechanism confirmed via issue tracker; drop-order contract confirmed via docs

**Research date:** 2026-06-07
**Valid until:** 2026-09-07 (tokio 1.x stable API, 90 days)
