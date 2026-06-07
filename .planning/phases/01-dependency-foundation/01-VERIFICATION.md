---
phase: 01-dependency-foundation
verified: 2026-06-07T14:30:00Z
status: human_needed
score: 6/7 must-haves verified
overrides_applied: 1
overrides:
  - must_have: "No git-rev source lines remain for tracing, alacritty_terminal, or floem in Cargo.toml"
    reason: "floem 0.2.0 on crates.io has incompatible API (palette::css, MouseButton, Renderer differ from git rev 31fa8f4 — 30+ compile errors). Documented fallback in PATTERNS.md and PLAN 01-01 task 1 was executed. rfd-tokio feature is correctly set on the git rev. The phase goal (clean build, no behaviour change) is achieved. floem crates.io migration is a separate future task."
    accepted_by: "verifier — documented fallback in plan"
    accepted_at: "2026-06-07T14:30:00Z"
human_verification:
  - test: "Launch the editor from the built binary and verify all existing behaviour works"
    expected: "Editor opens, LSP completions work, DAP debugging starts, plugin install/update runs, terminal renders, SSH remote connects — all identically to the pre-Phase-1 baseline"
    why_human: "Runtime behaviour cannot be verified by static analysis or automated unit tests. cargo build proves compilation; unit tests prove IPC roundtrip and zip safety; actual editor launch, LSP negotiation, DAP session, SSH proxy bootstrap, and terminal rendering require a running editor session."
---

# Phase 01: Dependency Foundation Verification Report

**Phase Goal:** The workspace compiles cleanly with all target dependency versions, with no behaviour change
**Verified:** 2026-06-07T14:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build --workspace` exits 0 with reqwest 0.12, tokio workspace dep, zip 2.x, interprocess 2.x, toml pinned, tracing on versioned releases, sha2 as workspace dep | VERIFIED | `cargo build --workspace` exits 0; `Finished dev profile` confirmed. Workspace Cargo.toml contains reqwest 0.12.28, tokio 1.52.3, interprocess 2.4.2, toml 0.8, sha2 0.10.8, tracing 0.1.44 / tracing-subscriber 0.3.23 on crates.io. |
| 2 | The editor launches and all existing behavior (LSP, DAP, plugins, terminal, SSH remote) works identically to before | NEEDS HUMAN | Static analysis and unit tests cannot verify runtime behaviour. Routed to human verification. |
| 3 | No CVE-2025-29787 vulnerable zip version remains in the dependency tree (`cargo tree -i zip` shows 2.x only) | VERIFIED | `cargo tree -i zip` shows `zip v2.4.2` (Cargo resolved 2.4.2 from `version = "2.4.0"` semver range). No zip 0.6.x in tree. Regression test `zip_slip_traversal_rejected` passes. |
| 4 | IPC single-instance detection still prevents duplicate app launches after interprocess 2.x migration | VERIFIED | `cargo test -p lapce-app single_instance_ipc_roundtrip` passes (Unix). All 1.x types `LocalSocketListener`/`LocalSocketStream` removed from lapce-app and lapce-proxy sources. `ListenerOptions`, `GenericFilePath`, `Stream` (2.x API) present in app.rs (8 occurrences). Windows IPC is explicitly scoped to manual verification per plan decision. |

**Score:** 6/7 must-haves verified (1 override applied for floem git-rev fallback; 1 item routed to human)

---

### Plan Must-Haves Detail (01-01-PLAN.md)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | cargo build --workspace exits 0 after all Cargo.toml version pins are applied | VERIFIED | Build clean, exit 0 |
| 2 | cargo tree -i zip shows only zip 2.x in the dependency tree | VERIFIED | zip v2.4.2 only; no 0.6.x |
| 3 | cargo tree -i reqwest shows reqwest 0.12.x | VERIFIED | reqwest 0.12.28 in tree (plus transitive 0.11.27 from wasi-experimental-http-wasmtime git dep — see WARNING below) |
| 4 | No git-rev source lines remain for tracing, alacritty_terminal, or floem in Cargo.toml | PASSED (override) | tracing and alacritty_terminal on crates.io stable. floem reverted to git rev 31fa8f4 (documented fallback — crates.io 0.2.0 API incompatible); rfd-tokio feature is correctly set. Override accepted. |
| 5 | tokio is present as an explicit workspace dependency at version 1.52.x | VERIFIED | `tokio = { version = "1.52.3", features = [...] }` in Cargo.toml line 58 |
| 6 | sha2 is declared in [workspace.dependencies] and referenced via workspace = true in lapce-app | VERIFIED | `sha2 = { version = "0.10.8" }` in workspace; `sha2 = { workspace = true }` in lapce-app/Cargo.toml line 62 |
| 7 | toml version is pinned to 0.8 (not wildcard) | VERIFIED | `toml = { version = "0.8" }` in Cargo.toml line 67 |

### Plan Must-Haves Detail (01-02-PLAN.md)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | cargo build --workspace exits 0 — workspace compiles cleanly | VERIFIED | Build clean |
| 2 | cargo test -p lapce-app zip_slip exits 0 — traversal entry is rejected | VERIFIED | `zip_slip_traversal_rejected` passes |
| 3 | cargo test -p lapce-app single_instance exits 0 — IPC roundtrip completes | VERIFIED | `single_instance_ipc_roundtrip` passes |
| 4 | IPC single-instance detection prevents duplicate app launches after interprocess 2.x migration | VERIFIED | interprocess 2.x API wired; test passes |
| 5 | logging.rs compiles against tracing-subscriber 0.3.23 stable | VERIFIED | reload::Layer::new at line 38; no reload::Subscriber; build exits 0 |
| 6 | No LocalSocketListener or LocalSocketStream identifiers remain in app.rs | VERIFIED | grep count = 0 in lapce-app/src/app.rs, lapce-proxy/src/cli.rs, lapce-proxy/src/lib.rs |

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | reqwest 0.12.28 pin | VERIFIED | Line 56: `reqwest = { version = "0.12.28", features = ["blocking", "json", "socks"] }` |
| `Cargo.toml` | tokio 1.52.3 workspace dep | VERIFIED | Line 58: `tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }` |
| `Cargo.toml` | interprocess 2.4.2 | VERIFIED | Line 47: `interprocess = { version = "2.4.2" }` |
| `Cargo.toml` | toml pinned 0.8 | VERIFIED | Line 67: `toml = { version = "0.8" }` |
| `Cargo.toml` | sha2 workspace dep | VERIFIED | Line 57: `sha2 = { version = "0.10.8" }` |
| `Cargo.toml` | tracing on crates.io stable | VERIFIED | tracing 0.1.44, tracing-log 0.2.0, tracing-subscriber 0.3.23, tracing-appender 0.2.5 — all crates.io, no git refs |
| `Cargo.toml` | alacritty_terminal on crates.io | VERIFIED | `version = "0.24.1"` (resolves to 0.24.2 in lockfile — semver compatible) |
| `Cargo.toml` | floem rfd-tokio feature | VERIFIED | `features = ["editor", "serde", "default-image-formats", "rfd-tokio"]` |
| `lapce-app/Cargo.toml` | zip 2.4.0 | VERIFIED | Line 63: `zip = { version = "2.4.0", default-features = false, features = ["deflate"] }` |
| `lapce-app/Cargo.toml` | sha2 workspace ref | VERIFIED | Line 62: `sha2 = { workspace = true }` |
| `lapce-app/src/app/logging.rs` | reload::Layer::new | VERIFIED | Line 38: `reload::Layer::new(log_file_filter_targets)` |
| `lapce-app/src/app.rs` | ListenerOptions + GenericFilePath | VERIFIED | 8 occurrences of ListenerOptions/GenericFilePath; 0 occurrences of 1.x types |
| `lapce-app/src/update.rs` | zip_slip_traversal_rejected test | VERIFIED | Test at line 247; passes |
| `lapce-app/src/app.rs` | single_instance_ipc_roundtrip test | VERIFIED | Test at line 4340, `#[cfg(unix)]` guard; passes |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Cargo.toml [workspace.dependencies.sha2]` | `lapce-app/Cargo.toml` | `workspace = true` | WIRED | lapce-app line 62 confirmed |
| `lapce-app/src/app.rs get_socket()` | `interprocess::local_socket::Stream` | `path.to_fs_name::<GenericFilePath>()? + Stream::connect(name)?` | WIRED | GenericFilePath present, ListenerOptions present, 0 old types |
| `lapce-app/src/app.rs listen_local_socket()` | `ListenerOptions::new().name(name).create_sync()` | interprocess 2.x builder API | WIRED | 8 occurrences confirmed |
| `lapce-app/src/app/logging.rs` | `reload::Layer` | tracing-subscriber 0.3.23 | WIRED | reload::Layer::new at line 38; build succeeds |
| `lapce-proxy/src/cli.rs` | interprocess 2.x Stream | GenericFilePath path conversion | WIRED | 0 LocalSocketStream occurrences; build succeeds |
| `lapce-proxy/src/lib.rs` | interprocess 2.x ListenerOptions | Builder API | WIRED | 0 LocalSocketListener occurrences; build succeeds |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace compiles clean | `cargo build --workspace` | `Finished dev profile` exit 0 | PASS |
| zip 2.x only in tree | `cargo tree -i zip` | `zip v2.4.2` only | PASS |
| interprocess 2.4.2 in tree | `cargo tree -i interprocess` | `interprocess v2.4.2` | PASS |
| Regression test: zip slip rejected | `cargo test -p lapce-app zip_slip_traversal_rejected` | 1 passed, 0 failed | PASS |
| Regression test: IPC roundtrip | `cargo test -p lapce-app single_instance_ipc_roundtrip` | 1 passed, 0 failed | PASS |
| No old reload::Subscriber in logging.rs | `grep "reload::Subscriber" lapce-app/src/app/logging.rs` | 0 matches | PASS |
| No 1.x IPC types in any source | `grep -c "LocalSocketListener\|LocalSocketStream" lapce-app/src/app.rs lapce-proxy/src/cli.rs lapce-proxy/src/lib.rs` | all 0 | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DEPS-01 | 01-01 | reqwest upgraded 0.11 → 0.12.28 | SATISFIED | Workspace dep: reqwest 0.12.28 with blocking/json/socks features. Note: reqwest 0.11.27 still present as transitive dep of `wasi-experimental-http-wasmtime` (external git dep — not under lapce direct control). lapce-owned code uses 0.12.28 exclusively. |
| DEPS-02 | 01-01 | tokio added to workspace as shared dependency | SATISFIED | `tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }` in workspace |
| DEPS-03 | 01-01 | zip upgraded 0.6.6 → 2.x (CVE-2025-29787) | SATISFIED | zip 2.4.0 in lapce-app/Cargo.toml; tree shows only v2.4.2; regression test passes |
| DEPS-04 | 01-02 | interprocess upgraded 1.2.1 → 2.x with call-site rewrite | SATISFIED | interprocess 2.4.2; all 1.x types removed from app.rs, cli.rs, lib.rs; IPC test passes |
| DEPS-05 | 01-01 | toml wildcard pinned to major version | SATISFIED | `toml = { version = "0.8" }` |
| DEPS-06 | 01-01/02 | Git-SHA-pinned deps moved to tagged releases | SATISFIED (partial override) | tracing family and alacritty_terminal on crates.io stable. floem remains on git rev 31fa8f4 — crates.io 0.2.0 API incompatible (documented fallback executed). rfd-tokio correctly set. psp-types intentionally stays on git (no crates.io release — documented in RESEARCH.md). |
| DEPS-07 | 01-01 | sha2 promoted to workspace dependency | SATISFIED | `sha2 = { version = "0.10.8" }` in workspace; lapce-app uses `workspace = true` |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | No TBD/FIXME/XXX/placeholder markers found in phase-modified files | — | — |

Debt-marker scan on phase-modified files (Cargo.toml, lapce-app/Cargo.toml, lapce-app/src/app/logging.rs, lapce-app/src/app.rs, lapce-app/src/update.rs, lapce-proxy/src/cli.rs, lapce-proxy/src/lib.rs): no unresolved markers.

---

### Warnings (Non-Blocking)

**WARNING: reqwest 0.11.27 transitive dependency**

`cargo tree -i reqwest` shows two versions:
- `reqwest v0.12.28` — used by lapce-app and lapce-proxy directly (correct)
- `reqwest v0.11.27` — pulled in by `wasi-experimental-http-wasmtime v0.10.0` (external git dep at `https://github.com/lapce/wasi-experimental-http`)

DEPS-01 ("reqwest upgraded across the workspace") is satisfied for all lapce-owned code. The 0.11.27 instance is a transitive dep from an external git crate that lapce cannot control without forking or replacing `wasi-experimental-http-wasmtime`. This does not block the phase goal (workspace builds clean; lapce code uses 0.12.28) but is worth noting for future supply-chain hardening.

**WARNING: Windows IPC single-instance verification is manual-only**

The `single_instance_ipc_roundtrip` test is `#[cfg(unix)]` — this is an explicit, documented scope decision in the plan ("Windows named-pipe CI unreliable; Windows coverage for single-instance IPC is verified manually in Phase 1"). No automated coverage for Windows IPC exists.

---

### Human Verification Required

#### 1. Editor Runtime Behaviour

**Test:** Build the lapce binary (`cargo build --release`), launch the editor, and exercise:
1. Open a directory and verify file tree loads
2. Open a Rust/Python/TypeScript file and verify LSP completions appear
3. Start a DAP debug session and verify breakpoints work
4. Install a plugin from the plugin registry
5. Open an integrated terminal and verify it renders and accepts input
6. Connect to a remote host via SSH and verify the workspace opens

**Expected:** All behaviours work identically to the pre-Phase-1 baseline; no regressions in LSP, DAP, plugin, terminal, or SSH remote functionality

**Why human:** Runtime behaviour — editor launch, process spawning, LSP negotiation, DAP protocol, SSH proxy bootstrap, terminal emulation — cannot be verified by static analysis or unit tests. Compilation success and unit test coverage prove the dependency migration is correct at the code level; runtime integration requires a live editor session.

---

### Gaps Summary

No blocking gaps. All automated checks pass. The phase goal is achieved for the verifiable portion of the success criteria.

The single human verification item (runtime editor behaviour) is the standard boundary between static verification and runtime integration testing — it is not a code defect.

---

_Verified: 2026-06-07T14:30:00Z_
_Verifier: Claude (gsd-verifier)_
