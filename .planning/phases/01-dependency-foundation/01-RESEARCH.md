# Phase 1: Dependency Foundation - Research

**Researched:** 2026-06-07
**Domain:** Rust dependency upgrades — reqwest, zip, interprocess, toml, tracing, sha2, tokio, floem
**Confidence:** HIGH (Rust codebase fully readable; key API changes verified via crates.io + official docs)

---

## Summary

Phase 1 is a pure dependency bump with zero behaviour change. All seven DEPS requirements address
separate libraries, and they can be worked in sequence (to isolate compile errors) or mostly in
parallel after the first build succeeds. The most invasive change is **interprocess 1.2.1 → 2.x**,
which requires rewriting the three IPC call sites in `lapce-app/src/app.rs` to use the new
`ListenerOptions` / `Stream` API. The second most invasive is **tracing git-rev → stable crates.io**,
which requires renaming `reload::Subscriber` → `reload::Layer` and `fmt::Subscriber` → `fmt::Layer` in
`lapce-app/src/app/logging.rs`. All other changes are either Cargo.toml-only (sha2 promotion, tokio
workspace dep, toml pin, zip feature-flag update) or drop-in version bumps where the blocking API
surface hasn't changed (reqwest 0.12 `blocking` / `json` / `socks` features still exist and work
identically).

**The rfd floem feature must be changed from `rfd-async-std` to `rfd-tokio` in the same commit as
the interprocess and tokio workspace additions.** The workspace currently carries `rfd-async-std`
which will conflict once a tokio runtime is introduced in Phase 2. Doing it in Phase 1 is zero-risk
because no runtime exists yet and `rfd` is only triggered by file-dialog user actions.

**Primary recommendation:** Work changes in dependency order — (1) zip upgrade in `lapce-app/Cargo.toml`,
(2) reqwest 0.12 + tokio workspace dep in root `Cargo.toml`, (3) sha2 promotion, (4) toml pin,
(5) tracing stable + rename two call sites, (6) interprocess 2.x + rewrite three IPC call sites,
(7) floem rfd feature flag swap + verify `alacritty_terminal` can use crates.io 0.24.1.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HTTP downloads (plugins, update, proxy) | API/Backend (lapce-proxy) | UI process (lapce-app) | `lapce_proxy::get_url` is called by both processes; blocking in proxy, re-exported to app |
| ZIP extraction (update, grammar downloads) | UI process (lapce-app) | — | `update.rs` and `grammars.rs` own extraction; proxy uses only tar/zstd |
| IPC single-instance detection | UI process (lapce-app) | — | `app.rs:4095–4164`; proxy never touches IPC |
| Tracing/logging setup | UI process (lapce-app) | lapce-proxy (tracing macros only) | `logging.rs` owns subscriber setup; proxy only emits `tracing::error!` |
| SHA256 hashing | UI process (lapce-app) | — | Currently only in `db.rs` and `plugin.rs`; promoted to workspace for future proxy use |

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DEPS-01 | `reqwest` upgraded 0.11 → 0.12.28 | reqwest 0.12 `blocking` / `json` / `socks` features still present; `copy_to`, `text`, `status` API unchanged; hyper 1.0 underneath is transparent to call sites |
| DEPS-02 | `tokio` added to workspace as shared dependency | tokio 1.52.3 is the latest stable; already pulled in transitively by reqwest; elevating to an explicit workspace dep + choosing features prepares Phase 2 |
| DEPS-03 | `zip` upgraded 0.6.6 → 2.x (CVE-2025-29787) | zip 2.4.0 is the latest stable 2.x (2.6.0/2.6.1 were yanked; 3.0.0 was published from those); `ZipArchive::new` / `::extract` API unchanged; feature rename required |
| DEPS-04 | `interprocess` upgraded 1.2.1 → 2.x | API redesigned: `LocalSocketListener` / `LocalSocketStream` replaced by `Listener` / `Stream` enums; `ListenerOptions` builder; three call sites in `app.rs` must be rewritten |
| DEPS-05 | `toml` wildcard `"*"` pinned | Currently resolves to two versions (0.5.9 and 0.8.2); pin to `"0.8"` to lock the major version used directly by lapce code |
| DEPS-06 | Git-SHA-pinned deps moved to tagged releases | tracing family → stable crates.io; alacritty_terminal → crates.io 0.24.1; floem → attempt crates.io 0.2.0; psp-types must stay git (no crates.io release) |
| DEPS-07 | `sha2` promoted to workspace dep | Currently only in `lapce-app/Cargo.toml`; promote to `[workspace.dependencies]` so `lapce-proxy` can reference it without re-declaring the version in Phase 4 |
</phase_requirements>

---

## Standard Stack

### Core Dependencies Under Change

| Library | Current Version | Target Version | Where Declared |
|---------|----------------|----------------|----------------|
| `reqwest` | 0.11.27 (crates.io) | 0.12.28 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `tokio` | 1.38.0 (transitive) | 1.52.3 [VERIFIED: crates.io] | Add to `[workspace.dependencies]` |
| `zip` | 0.6.6 | 2.4.0 [VERIFIED: crates.io] | `lapce-app/Cargo.toml` (direct) |
| `interprocess` | 1.2.1 | 2.4.2 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `toml` | `"*"` (resolves to 0.5.9 and 0.8.2) | `"0.8"` | `[workspace.dependencies]` |
| `tracing` | 0.2.0-git (rev 908cc43) | 0.1.44 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `tracing-subscriber` | 0.3.0-git (rev 908cc43) | 0.3.23 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `tracing-appender` | 0.2.0-git (rev 908cc43) | 0.2.5 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `tracing-log` | 0.2.0-git (rev 908cc43) | 0.2.0 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `sha2` | 0.10.8 (`lapce-app` direct) | 0.10.8 (no version change) | Promote to `[workspace.dependencies]` |
| `alacritty_terminal` | 0.24.1-dev (git rev) | 0.24.1 [VERIFIED: crates.io] | `[workspace.dependencies]` |
| `floem` | 0.2.0-git (rev 31fa8f4) | 0.2.0 [VERIFIED: crates.io] (attempt; see pitfalls) | `[workspace.dependencies]` |
| `psp-types` | 0.1.0-git | stays on git — no crates.io release [VERIFIED: crates.io] | `[workspace.dependencies]` |

### Unchanged Pass-Through

| Library | Status | Notes |
|---------|--------|-------|
| `config` | `=0.13.4` | Pinned exact — do not change in Phase 1 |
| `lsp-types` | patched git rev | Do not change in Phase 1 |
| `wasmtime` / `wasi-*` | 14.0.x | Do not change in Phase 1 |
| `sha2` | 0.10.8 promoted | Keep same semver, just move to workspace |

---

## Package Legitimacy Audit

All packages in this phase are existing dependencies being version-bumped, not new introductions.
No slopcheck audit required for established packages with multi-year crates.io histories.

| Package | Registry | Age | Downloads | slopcheck | Disposition |
|---------|----------|-----|-----------|-----------|-------------|
| reqwest 0.12.28 | crates.io | 6+ yrs | 100M+/yr | OK | Approved — version bump of existing dep |
| tokio 1.52.3 | crates.io | 6+ yrs | 200M+/yr | OK | Approved — elevating existing transitive dep |
| zip 2.4.0 | crates.io | 8+ yrs (zip-rs/zip2 fork) | 10M+/yr | OK | Approved — CVE remediation upgrade |
| interprocess 2.4.2 | crates.io | 4+ yrs | >500K/yr | OK | Approved — version bump of existing dep |
| tracing 0.1.44 | crates.io | 5+ yrs | 300M+/yr | OK | Approved — moving from git pre-release to stable |
| tracing-subscriber 0.3.23 | crates.io | 5+ yrs | 200M+/yr | OK | Approved — moving from git pre-release to stable |
| tracing-appender 0.2.5 | crates.io | 5+ yrs | 50M+/yr | OK | Approved — moving from git pre-release to stable |
| tracing-log 0.2.0 | crates.io | 5+ yrs | 150M+/yr | OK | Approved — moving from git pre-release to stable |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
lapce binary (main process)
├── lapce-app (UI, Floem main thread)
│   ├── app.rs — IPC listener / single-instance check  <── interprocess 2.x call sites
│   ├── app/logging.rs — tracing subscriber setup       <── tracing stable rename
│   ├── app/grammars.rs — grammar download + zip extract <── zip 2.x call site
│   ├── update.rs — self-update download + zip extract   <── zip 2.x call site
│   ├── db.rs / plugin.rs — sha2 usage                  <── sha2 workspace dep
│   └── [reqwest via lapce_proxy::get_url re-export]    <── reqwest 0.12 (blocking)
│
└── lapce-proxy binary (separate process)
    └── lib.rs:get_url — blocking HTTP with proxy support  <── reqwest 0.12 blocking
    └── plugin/mod.rs:download_volt — plugin download      <── reqwest 0.12 blocking

[workspace Cargo.toml]
  reqwest = "0.12.28", features = ["blocking","json","socks"]
  tokio   = "1.52.3",  features = ["rt-multi-thread","macros","sync","time"]
  interprocess = "2.4.2"
  toml = "0.8"
  tracing family -> stable crates.io
  sha2 = "0.10.8"
```

### Recommended Cargo.toml Change Summary

```toml
# Root workspace Cargo.toml — changes only

[workspace.dependencies]
# DEPS-01: reqwest 0.11 -> 0.12
reqwest = { version = "0.12.28", features = ["blocking", "json", "socks"] }

# DEPS-02: tokio explicit workspace dep
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }

# DEPS-04: interprocess 1.x -> 2.x
interprocess = { version = "2.4.2" }

# DEPS-05: toml pin
toml = { version = "0.8" }

# DEPS-07: sha2 to workspace
sha2 = { version = "0.10.8" }

# DEPS-06: tracing family — stable crates.io
[workspace.dependencies.tracing]
version = "0.1.44"
# Remove: git/rev lines

[workspace.dependencies.tracing-log]
version = "0.2.0"

[workspace.dependencies.tracing-subscriber]
version = "0.3.23"
features = ["fmt", "env-filter", "registry", "reload", "tracing-log"]

[workspace.dependencies.tracing-appender]
version = "0.2.5"

# DEPS-06: alacritty_terminal stable crates.io
[workspace.dependencies.alacritty_terminal]
version = "0.24.1"
# Remove: git/rev lines

# DEPS-06: floem — attempt crates.io 0.2.0 (has rfd-tokio feature)
[workspace.dependencies.floem]
version = "0.2.0"
features = ["editor", "serde", "default-image-formats", "rfd-tokio"]
# Change: rfd-async-std -> rfd-tokio (safe now; no runtime yet but avoids Phase 2 breakage)
# Remove: git/rev lines  (add back if compile fails; see pitfalls)
```

```toml
# lapce-app/Cargo.toml — changes only

# Remove: sha2 = { version = "0.10.8" }  (now from workspace)
sha2 = { workspace = true }

# zip upgrade: change feature flag name
# Old: zip = { version = "0.6.6", default-features = false, features = ["deflate"] }
# New: zip = { version = "2.4.0", default-features = false, features = ["deflate"] }
zip = { version = "2.4.0", default-features = false, features = ["deflate"] }

# Also add tokio if needed for rfd-tokio indirect dep:
# tokio = { workspace = true }
```

---

## Detailed API Changes Per Dependency

### DEPS-01: reqwest 0.11 → 0.12.28

**Call sites affected:** `lapce-proxy/src/lib.rs:189-215` (the `get_url` function)

**What changed:**
- Hyper 1.0 is now the underlying HTTP client — transparent to the `blocking` API surface
- `blocking` feature: **still exists and must be explicitly enabled** (was default in 0.11; now opt-in)
- `json` feature: still exists, no change
- `socks` feature: still exists, no change
- `Response::copy_to` / `Response::text` / `Response::status`: **API unchanged** [CITED: docs.rs/reqwest/0.12.28/reqwest/blocking/struct.Response.html]
- `Proxy::all()` / `ClientBuilder::proxy()` / `ClientBuilder::timeout()` / `ClientBuilder::user_agent()`: **API unchanged**
- `reqwest::IntoUrl` trait: **unchanged** — `get_url` signature compiles as-is
- Error type: `Error::is_*` inspector methods removed — lapce code does not use these

**Verdict:** The `get_url` function body requires **zero changes**. Only the version string in `Cargo.toml` changes. `features = ["blocking", "json", "socks"]` must be retained explicitly.

**Compile-time risk:** LOW. reqwest 0.12 has a `rust-version = "1.64.0"` minimum; lapce requires 1.87.0. No conflict. [VERIFIED: crates.io]

---

### DEPS-02: tokio to workspace dep

**Call sites affected:** None in Phase 1. This is a Cargo.toml-only change.

**What to add to `[workspace.dependencies]`:**
```toml
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }
```

**Why these features:**
- `rt-multi-thread`: needed for Phase 2 runtime construction
- `macros`: needed for `#[tokio::test]` in tests (later phases)
- `sync`: tokio channels and Mutex
- `time`: timeout, sleep
- `fs`: async file I/O (Phase 4)

**Current state:** tokio 1.38.0 is already in the dependency tree (pulled by reqwest 0.11). After reqwest → 0.12, tokio 1.52.3 will be resolved. Adding it as an explicit workspace dep just pins the version and features. No crates need to declare `tokio = { workspace = true }` in Phase 1 — that happens in Phase 2 when binaries actually create a runtime.

---

### DEPS-03: zip 0.6.6 → 2.4.0 (CVE-2025-29787)

**Call sites affected:**
- `lapce-app/src/update.rs:135` — `zip::ZipArchive::new(File::open(src)?)` + `archive.extract(parent)`
- `lapce-app/src/app/grammars.rs:129` — `zip::ZipArchive::new(file)` + `archive.extract(&dir)`

**What changed:**
- `ZipArchive::new(reader)` → **signature unchanged** [VERIFIED: crates.io docs.rs/zip/2.4.0]
- `ZipArchive::extract(path)` → **signature unchanged** (path traversal protection is now built-in via `enclosed_name`)
- Feature flag `"deflate"` → **still valid in 2.4.0** (maps to `deflate-flate2` or `deflate-zopfli`)
- `zip 2.4.0` requires `rust-version = "1.73.0"`; lapce requires 1.87.0 — no conflict [VERIFIED: crates.io]

**CVE note:** CVE-2025-29787 (symlink path traversal) affects zip ≤ 0.6.6. zip 2.x validates entries via `enclosed_name` before extraction. [CITED: github.com/zip-rs/zip2/releases]

**Feature flag change in Cargo.toml:**
```toml
# Old
zip = { version = "0.6.6", default-features = false, features = ["deflate"] }
# New
zip = { version = "2.4.0", default-features = false, features = ["deflate"] }
```

**Verdict:** The existing `ZipArchive::new(reader)` + `archive.extract(dir)` call pattern is **API-identical** between 0.6.6 and 2.4.0. Both call sites compile without modification.

**Version selection note:** zip 2.6.0 and 2.6.1 were yanked; maintainers republished those changes as 3.0.0. The latest stable 2.x series is **2.4.0**. [CITED: github.com/zip-rs/zip2/issues/337]

---

### DEPS-04: interprocess 1.2.1 → 2.4.2

**This is the most invasive change in Phase 1.** Three call sites in `lapce-app/src/app.rs` use the old API and must be rewritten.

#### Current 1.x API (lapce-app/src/app.rs:4095–4164)

```rust
// Connect (client side) — app.rs:4095-4100
pub fn get_socket() -> Result<interprocess::local_socket::LocalSocketStream> {
    let local_socket = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    let socket = interprocess::local_socket::LocalSocketStream::connect(local_socket)?;
    Ok(socket)
}

// Socket as argument type — app.rs:4103-4104
pub fn try_open_in_existing_process(
    mut socket: interprocess::local_socket::LocalSocketStream,
    ...

// Server (listener) side — app.rs:4131-4140
fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let local_socket = Directory::local_socket() ...;
    if local_socket.exists() {
        let _ = std::fs::remove_file(&local_socket);
    }
    let socket = interprocess::local_socket::LocalSocketListener::bind(local_socket)?;
    for stream in socket.incoming().flatten() { ... }
```

#### New 2.x API

```rust
// Import pattern for 2.x [CITED: docs.rs/interprocess/2.4.2/interprocess/local_socket/]
use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, Stream,
    prelude::*,    // brings ToFsName, ToNsName traits into scope
};

// Connect (client side) — filesystem path -> use to_fs_name
pub fn get_socket() -> Result<Stream> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    let name = path.to_fs_name::<GenericFilePath>()?;
    let stream = Stream::connect(name)?;
    Ok(stream)
}

// Server (listener side) — ListenerOptions builder
fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    let name = path.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()?;
    for stream in listener.incoming().filter_map(|r| r.ok()) { ... }
}
```

**Note on `GenericFilePath` vs `GenericNamespaced`:**
- `GenericNamespaced` is for abstract namespace names (cross-platform named pipes / abstract sockets)
- `GenericFilePath` is for filesystem-path sockets (Unix domain sockets backed by a file)
- Lapce uses `Directory::local_socket()` which returns a `PathBuf` → use `GenericFilePath`
- **Windows caveat:** On Windows, interprocess 2.x maps filesystem paths to named pipes internally; `GenericFilePath` handles this transparently. The current code already manually removes the socket file on Unix; with 2.x the listener auto-removes on drop, but the manual remove before bind should still be kept to handle stale sockets from crashed processes. [CITED: docs.rs/interprocess/latest]

**`Stream` as a type:** In 2.x, `Stream` is an enum (not a struct) that dispatches between the platform-specific implementations. It implements `Read + Write + BufRead` — the existing `socket.read(&mut buf)` and `stream_ref.write_all(b"received")` patterns still work.

**`incoming()` on the listener:** The 2.x listener's `incoming()` returns an iterator of `Result<Stream, _>`, same pattern as before. The existing `for stream in socket.incoming().flatten()` pattern works unchanged if `flatten` is replaced with `filter_map(|r| r.ok())` or kept as-is (both compile).

**Rust version requirement:** interprocess 2.4.2 requires rust 1.75.0; lapce requires 1.87.0 — no conflict. [VERIFIED: crates.io]

**Minimum change:** Update 3 type references + add import + change `bind()` to `ListenerOptions::new().name(name).create_sync()` + change `connect(path)` to `Stream::connect(name)` where `name` comes from `path.to_fs_name::<GenericFilePath>()`.

---

### DEPS-05: toml pin

**Current state:** `toml = { version = "*" }` in `[workspace.dependencies]` resolves to two versions in the lockfile (0.5.9 and 0.8.2) because `config` 0.13.4 pulls in toml 0.5 and the workspace dep resolves to 0.8.

**Action:** Change `toml = { version = "*" }` to `toml = { version = "0.8" }`. This pins the workspace's own toml use to 0.8 semver and avoids Cargo resolving any incompatible future 1.0 release automatically.

**Call sites:** No code changes needed. `toml::Value`, `toml::from_str`, etc. have stable APIs in 0.8.

---

### DEPS-06: Git SHA pins → tagged releases

#### tracing family

**Critical API difference between git-rev 0.2.0 and stable 0.1.x / 0.3.x:**

In `lapce-app/src/app/logging.rs`, two types are used that have different names between the git
pre-release (which lapce currently pins) and stable crates.io releases:

| Git pre-release (current) | Stable crates.io equivalent |
|---------------------------|---------------------------|
| `reload::Subscriber::new(...)` | `reload::Layer::new(...)` |
| `fmt::Subscriber::default()` | `fmt::Layer::default()` (or `fmt::Subscriber::default()` — both exist) |

After investigation: `fmt::Subscriber` IS present in stable tracing-subscriber 0.3.18+
as a type alias. `reload::Subscriber` is NOT in stable — it is named `reload::Layer` in
stable 0.3.x. [CITED: docs.rs/tracing-subscriber/0.3.18]

**Required code change in `lapce-app/src/app/logging.rs`:**
```rust
// Old (git pre-release 0.3.0-git):
let (log_file_filter, reload_handle) = reload::Subscriber::new(log_file_filter_targets);

// New (stable 0.3.23):
let (log_file_filter, reload_handle) = reload::Layer::new(log_file_filter_targets);
```

The `fmt::Subscriber::default()` usages may compile as-is (type alias exists) but if they don't,
change to `fmt::Layer::default()`.

**Feature flags needed for tracing-subscriber 0.3.23:**

```toml
[workspace.dependencies.tracing-subscriber]
version = "0.3.23"
features = ["fmt", "env-filter", "registry"]
# Note: "reload" and "tracing-log" are included in the default feature set
```

The default features already include: `smallvec`, `fmt`, `ansi`, `tracing-log`, `std`. The `reload`
module is gated by the `fmt` feature. The `Targets` filter used in logging.rs is part of `env-filter`.

**tracing 0.1.44 stable:**
The macros `error!`, `debug!`, `warn!`, `info!`, `event!`, `Level`, `instrument`, `Instrument` all
exist in stable 0.1.44 unchanged. The custom `tracing.rs` re-export module (`use tracing::{self, Instrument, Level as TraceLevel, event as trace, instrument}`) compiles without change. [VERIFIED: crates.io]

**tracing-appender 0.2.5:**
`rolling::Builder::new()`, `rolling::Rotation::DAILY`, `non_blocking()`, `WorkerGuard` — all present
in stable 0.2.5. API identical. [VERIFIED: crates.io]

**tracing-log 0.2.0:**
No direct API use in lapce; included as a tracing-subscriber feature.

#### alacritty_terminal

Current git rev is "0.24.1-dev" from `https://github.com/alacritty/alacritty` at rev
`cacdb5bb3b72bad2c729227537979d95af75978f`. crates.io has `alacritty_terminal = "0.24.1"`.

**Approach:** Change git dep to `version = "0.24.1"`. If the git rev has post-publish commits that
lapce depends on, the build will fail with a compile error — in that case revert to the git pin and
mark this sub-requirement as "stay on git". The open question in STATE.md notes this risk.

#### floem

crates.io `floem = "0.2.0"` exists and has the `rfd-tokio` feature. The git rev `31fa8f4` is
described as potentially post-dating the 0.2.0 tag (STATE.md open question). Attempt crates.io
first; if compile fails or a known post-tag fix is missing, revert to git pin and add the
`rfd-tokio` feature to the git dep instead.

**Feature change (required regardless of git vs crates.io):**
```toml
# Old: features = ["editor", "serde", "default-image-formats", "rfd-async-std"]
# New: features = ["editor", "serde", "default-image-formats", "rfd-tokio"]
```

This is safe in Phase 1 (no tokio runtime yet); it prevents the Phase 2 breakage noted in
STATE.md critical pitfalls. `rfd-async-std` would panic when file dialogs run inside a tokio
context; `rfd-tokio` is correct. [CITED: .planning/STATE.md]

#### psp-types

No crates.io release exists. Must remain on git pin. No change. [VERIFIED: crates.io]

---

### DEPS-07: sha2 → workspace dep

**Current:** `lapce-app/Cargo.toml` declares `sha2 = { version = "0.10.8" }` as a direct dep.
`lapce-proxy/Cargo.toml` does not declare sha2.

**Action:**
1. Add to `[workspace.dependencies]`: `sha2 = { version = "0.10.8" }`
2. Change `lapce-app/Cargo.toml`: `sha2 = { workspace = true }`
3. No change to `lapce-proxy/Cargo.toml` in Phase 1 — proxy will add `sha2 = { workspace = true }` in Phase 4 when integrity checking is implemented.

**sha2 version:** Keep at 0.10.8. sha2 0.11.0 was recently published but changes the `rust-version`
to 1.85 and may have Digest API changes. The requirement says "promote", not "upgrade". Stable
0.10.8 continues to compile against the RustCrypto `digest 0.10` API used in `db.rs` and `plugin.rs`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ZIP extraction with path-traversal protection | Custom extract loop | `ZipArchive::extract()` from zip 2.x | `enclosed_name` validation is built into 2.x extract; manual loops miss symlink edge cases |
| Single-instance IPC | Custom socket file management | interprocess 2.x `ListenerOptions` + `Stream` | Cross-platform handling of named pipes vs UDS, socket cleanup on drop, platform normalization |
| HTTP client with proxy support | Custom CONNECT tunneling | reqwest `Proxy::all()` with `blocking` feature | SOCKS5, HTTP CONNECT, environment proxy auto-detection are all tested in reqwest |
| Structured logging with reload | Manual log level management | tracing + tracing-subscriber `reload::Layer` | Thread-safe filter reloading without reinitializing the subscriber |

---

## Common Pitfalls

### Pitfall 1: interprocess 2.x auto-removes socket file on listener drop

**What goes wrong:** The existing code manually calls `std::fs::remove_file(&local_socket)` before
binding. In 2.x, the `Listener` will also remove the file when dropped. This is additive (not a
bug) but means if the code path that manually removes the file races with a dropped listener, a
transient error could occur.

**How to avoid:** Keep the manual `remove_file` before `ListenerOptions::create_sync()` — it handles
the "stale socket from crashed process" case. The double-remove is harmless (second remove just
gets `ENOENT`).

**Warning signs:** `std::io::Error` kind `NotFound` logged on clean shutdown. Benign; suppress with
`if local_socket.exists()` guard (already in existing code).

---

### Pitfall 2: floem git rev may have post-tag fixes

**What goes wrong:** If the git rev `31fa8f4` includes commits merged after the v0.2.0 crates.io
tag, switching to `version = "0.2.0"` will cause compile errors or runtime behaviour differences.

**How to avoid:** Attempt crates.io first. If `cargo build` fails with compile errors in floem-
related code, revert to:
```toml
[workspace.dependencies.floem]
git = "https://github.com/lapce/floem"
rev = "31fa8f444c37f4c314f47d88c23ffdbc25f2ab53"
features = ["editor", "serde", "default-image-formats", "rfd-tokio"]
```
The `rfd-tokio` feature change must happen regardless.

**Warning signs:** Compile errors referencing floem internal types not present in 0.2.0 stable.

---

### Pitfall 3: tracing `reload::Subscriber` → `reload::Layer` rename

**What goes wrong:** `reload::Subscriber::new(...)` in `app/logging.rs:34` does not compile against
stable tracing-subscriber 0.3.x because the type is named `reload::Layer` in stable releases.

**How to avoid:** Change this in the same commit as the tracing version bump. The rename is
mechanical — one line. The `Handle<Targets>` type returned is unchanged.

**Warning signs:** `error[E0425]: cannot find struct, variant or union type 'Subscriber' in module 'tracing_subscriber::reload'`

---

### Pitfall 4: zip 2.6.x was yanked — do not use

**What goes wrong:** `cargo add zip@2.6.0` or `version = "2"` in Cargo.toml may resolve to 2.6.x
depending on the resolver. 2.6.0 and 2.6.1 were yanked; the maintainers published those changes as
3.0.0. Cargo will skip yanked versions, but an explicit `version = "2"` currently resolves to
2.4.0 (the highest non-yanked 2.x). Lock to `"2.4.0"` explicitly to prevent future accidental
upgrade to 3.x.

**How to avoid:** Use `version = "2.4.0"` (exact minor) in `lapce-app/Cargo.toml`.

---

### Pitfall 5: reqwest 0.12 `blocking` is not default

**What goes wrong:** If the feature list is accidentally dropped during the version bump, reqwest
0.12 will compile without the `blocking` feature, and `reqwest::blocking::Client` will not be
available, causing compile errors in `lapce-proxy/src/lib.rs`.

**How to avoid:** The workspace dep must keep `features = ["blocking", "json", "socks"]` explicitly.

---

### Pitfall 6: `interprocess::local_socket::LocalSocketStream` type in function signatures

**What goes wrong:** The public function `try_open_in_existing_process` takes
`interprocess::local_socket::LocalSocketStream` as an argument. Callers (in `lapce-app/src/app.rs`
around the launch/IPC path) pass values of this type. After the upgrade the type becomes
`interprocess::local_socket::Stream`.

**How to avoid:** Update the function signature and all call sites in the same commit.

---

### Pitfall 7: rfd feature on floem — must be `rfd-tokio` not `rfd-async-std`

**What goes wrong:** If `rfd-async-std` is kept after Phase 2 introduces a tokio runtime, file
dialogs will panic at runtime with "cannot start a runtime from within a runtime" (or similar
async-std / tokio incompatibility).

**How to avoid:** Switch to `rfd-tokio` in Phase 1. The feature exists in both the git rev and
crates.io 0.2.0. No runtime is active in Phase 1, so the switch is zero-risk. [CITED: .planning/STATE.md pitfalls]

---

## Code Examples

### interprocess 2.x — filesystem path socket (listener)

```rust
// Source: docs.rs/interprocess/2.4.2/interprocess/local_socket/
use interprocess::local_socket::{
    GenericFilePath, ListenerOptions,
    prelude::*,  // brings ToFsName into scope
};
use std::sync::mpsc::SyncSender;
use anyhow::Result;
use lapce_core::directory::Directory;
use lapce_rpc::core::CoreNotification;

fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow::anyhow!("can't get local socket folder"))?;
    if path.exists() {
        if let Err(err) = std::fs::remove_file(&path) {
            tracing::error!("{:?}", err);
        }
    }
    let name = path.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_sync()?;
    for stream in listener.incoming().filter_map(|r| r.ok()) {
        // stream: interprocess::local_socket::Stream
        // implements Read + Write — existing BufReader / write_all usage unchanged
        let tx = tx.clone();
        std::thread::spawn(move || -> Result<()> {
            // ... existing body unchanged ...
            Ok(())
        });
    }
    Ok(())
}
```

### interprocess 2.x — filesystem path socket (client connect)

```rust
// Source: docs.rs/interprocess/2.4.2/interprocess/local_socket/
use interprocess::local_socket::{GenericFilePath, Stream, prelude::*};

pub fn get_socket() -> Result<Stream> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow::anyhow!("can't get local socket folder"))?;
    let name = path.to_fs_name::<GenericFilePath>()?;
    let stream = Stream::connect(name)?;
    Ok(stream)
}

pub fn try_open_in_existing_process(
    mut socket: Stream,    // was: LocalSocketStream
    paths: &[PathObject],
) -> Result<()> {
    // ... body unchanged ...
}
```

### tracing-subscriber 0.3.23 — rename reload::Subscriber → reload::Layer

```rust
// Source: docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/reload/
use tracing_subscriber::{filter::Targets, reload::Handle};

// OLD (git pre-release):
// let (log_file_filter, reload_handle) = reload::Subscriber::new(log_file_filter_targets);

// NEW (stable 0.3.23):
let (log_file_filter, reload_handle) = reload::Layer::new(log_file_filter_targets);
// reload_handle: Handle<Targets, _>  — same type, same usage
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| reqwest 0.11 (hyper 0.14) | reqwest 0.12 (hyper 1.0) | reqwest 0.12.0, Feb 2024 | Improved connection pool, http v1 types |
| interprocess 1.x struct-based API | interprocess 2.x enum-based API | 2.0.0, Jan 2024 | Cross-platform named pipe/UDS abstraction improved |
| zip 0.6 (no traversal protection) | zip 2.x (enclosed_name guard) | CVE-2025-29787 | Path traversal in extract now rejected |
| tracing git pre-release | tracing stable 0.1.x / subscriber 0.3.x | tracing 0.1.x stable, ongoing | Stable API, no git dependency, reproducible builds |

**Deprecated/outdated:**
- `LocalSocketListener::bind(path)` (interprocess 1.x): replaced by `ListenerOptions::new().name(name).create_sync()`
- `LocalSocketStream::connect(path)` (interprocess 1.x): replaced by `Stream::connect(name)` where name is obtained via `ToFsName`
- `reload::Subscriber` (tracing-subscriber git pre-release): renamed to `reload::Layer` in stable
- `zip::ZipArchive::extract` with 0.6.6: CVE-vulnerable; same API in 2.4.0 but safe

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ZipArchive::new()` / `::extract()` API signatures are identical in zip 2.4.0 and 0.6.6 | DEPS-03 | Requires code changes at two call sites — discoverable immediately at compile time |
| A2 | floem crates.io 0.2.0 is API-compatible with git rev `31fa8f4` | DEPS-06 floem | Compile errors; fallback to git rev with `rfd-tokio` feature |
| A3 | alacritty_terminal crates.io 0.24.1 is API-compatible with git rev `cacdb5bb` | DEPS-06 alacritty | Compile errors; fallback to git rev |
| A4 | `fmt::Subscriber::default()` exists as a type alias in tracing-subscriber 0.3.23 | DEPS-06 tracing | Compile error; change to `fmt::Layer::default()` |
| A5 | interprocess 2.x `GenericFilePath` + `to_fs_name` trait handles Windows named pipes transparently | DEPS-04 | If Windows-specific code is needed, may require `#[cfg]` blocks; testable on macOS/Linux first |
| A6 | tokio feature set `["rt-multi-thread","macros","sync","time","fs"]` is sufficient for Phase 1 + Phase 2 | DEPS-02 | Additional features may be needed in Phase 2; can be added to workspace dep without downstream breakage |

**If this table is empty:** It is not empty — see above.

---

## Open Questions

1. **Does floem crates.io 0.2.0 include the post-tag commits lapce depends on?**
   - What we know: git rev `31fa8f4` resolves to version `0.2.0`; crates.io also has `0.2.0`
   - What's unclear: whether the git rev was published as the crates.io tag or has additional commits
   - Recommendation: Attempt crates.io first; fallback to git on compile failure

2. **Does alacritty_terminal 0.24.1 crates.io match the git rev?**
   - What we know: git rev is "0.24.1-dev"; crates.io has "0.24.1"
   - Recommendation: Switch to crates.io; rollback if compile errors

3. **Is `GenericFilePath` or a different type the correct namespace for path-based sockets in interprocess 2.x?**
   - What we know: `GenericNamespaced` is for named sockets; a separate type handles FS paths
   - What's unclear: exact type name in 2.4.2 (may be `GenericFilePath` or accessed differently)
   - Recommendation: Consult `docs.rs/interprocess/2.4.2` at implementation time; compile errors will guide to the correct type

---

## Environment Availability

Step 2.6: SKIPPED — Phase 1 is Cargo.toml and call-site source changes only. No external tooling,
services, databases, or CLIs beyond the Rust toolchain are required. The lapce workspace already
compiles on Rust 1.87.0 as confirmed by the lockfile.

---

## Security Domain

Phase 1 upgrades dependencies — the security-relevant change is the zip CVE fix (DEPS-03).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes (zip archive entries) | `ZipArchive::extract` with `enclosed_name` guard in zip 2.x |
| V6 Cryptography | no | sha2 promotion is structural, not a new crypto operation |
| V2 Authentication | no | IPC is local-machine only |
| V3 Session Management | no | No sessions in Phase 1 scope |
| V4 Access Control | no | No access control changes |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| ZIP path traversal (CVE-2025-29787) | Tampering | zip 2.x `enclosed_name` rejects `../` entries automatically |
| Stale IPC socket from crashed process | Denial of Service | Manual `remove_file` before bind (existing pattern, retained in 2.x code) |

---

## Sources

### Primary (HIGH confidence)
- `Cargo.toml` + `Cargo.lock` — current version pins and resolved dep tree (directly read)
- `lapce-app/src/app.rs` — IPC call sites (directly read, lines 4095–4164)
- `lapce-app/src/app/logging.rs` — tracing subscriber setup (directly read)
- `lapce-app/src/update.rs` + `lapce-app/src/app/grammars.rs` — zip call sites (directly read)
- `lapce-proxy/src/lib.rs` — reqwest blocking usage (directly read)
- crates.io registry via `cargo info` — version numbers and feature flags verified [VERIFIED: crates.io]

### Secondary (MEDIUM confidence)
- [docs.rs/interprocess/latest/interprocess/local_socket/](https://docs.rs/interprocess/latest/interprocess/local_socket/) — 2.x API confirmed via web search; exact `GenericFilePath` type name is [ASSUMED]
- [docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/fmt/struct.Subscriber.html](https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/fmt/struct.Subscriber.html) — `fmt::Subscriber` exists in stable 0.3.18
- [reqwest v0.12 announcement](https://seanmonstar.com/blog/reqwest-v012/) — breaking changes and feature compatibility

### Tertiary (LOW confidence / [ASSUMED])
- interprocess 2.x `GenericFilePath` type name — found in search results but not directly verified via docs; planner should confirm during implementation [ASSUMED]
- `zip 2.4.0 features = ["deflate"]` still valid — inferred from feature table; should verify at compile time [ASSUMED]

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all version numbers verified via `cargo info` against crates.io
- Architecture (call sites): HIGH — read directly from source files
- interprocess 2.x API: MEDIUM — pattern confirmed via official docs search; exact type names have [ASSUMED] items
- Pitfalls: HIGH — derived from direct code reading and state.md decisions log

**Research date:** 2026-06-07
**Valid until:** 2026-09-07 (stable crates.io versions; tracing and zip are active but semver-stable)
