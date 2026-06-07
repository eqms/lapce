# Phase 1: Dependency Foundation - Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 8 files (6 modified, 2 new regression tests)
**Analogs found:** 8 / 8

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` (workspace) | config | — | self (lines 33–119) | self (existing file to edit) |
| `lapce-app/Cargo.toml` | config | — | self (lines 62–63) | self (existing file to edit) |
| `lapce-app/src/app.rs` | IPC call-site rewrite | request-response | self (lines 4095–4167) | self (existing file to edit) |
| `lapce-app/src/app/logging.rs` | logging shim rewrite | — | self (lines 33–34, 50, 59) | self (existing file to edit) |
| `lapce-app/src/update.rs` | zip call site | file-I/O | self (line 135) + `lapce-app/src/app/grammars.rs:129` | self (existing file; API-identical) |
| `lapce-app/src/app/grammars.rs` | zip call site | file-I/O | self (line 129) + `lapce-app/src/update.rs:135` | self (existing file; API-identical) |
| `lapce-app/src/app.rs` `#[cfg(test)] mod tests` *(inline)* | regression test | request-response | `lapce-proxy/src/cli.rs:93–219` | role-match (same in-module unit test style, inline in existing file) |
| `lapce-app/src/update.rs` `#[cfg(test)] mod tests` *(inline)* | regression test | file-I/O | `lapce-core/src/encoding.rs:98–147` | role-match (pure-function unit test style, inline in existing file) |

---

## Pattern Assignments

### `Cargo.toml` (workspace root, config)

**Analog:** self — read lines 33–119 before editing.

**Current state of each dep to change** (lines 47, 56, 65, 79–83, 101–119):

```toml
# line 47 — interprocess 1.x → 2.x
interprocess = { version = "1.2.1" }

# line 56 — reqwest 0.11 → 0.12
reqwest = { version = "0.11", features = ["blocking", "json", "socks"] }

# line 65 — toml wildcard → pinned
toml = { version = "*" }

# lines 79–83 — floem git rev → crates.io + feature swap
[workspace.dependencies.floem]
git = "https://github.com/lapce/floem"
rev = "31fa8f444c37f4c314f47d88c23ffdbc25f2ab53"
features = ["editor", "serde", "default-image-formats", "rfd-async-std"]

# lines 101–119 — tracing family git rev → stable crates.io
[workspace.dependencies.tracing]
git     = "https://github.com/tokio-rs/tracing"
rev     = "908cc432a5994f6e17c8f36e13c217dc40085704"
package = "tracing"
# (same pattern for tracing-log, tracing-subscriber, tracing-appender)

# lines 121–123 — alacritty_terminal git rev → crates.io
[workspace.dependencies.alacritty_terminal]
git = "https://github.com/alacritty/alacritty"
rev = "cacdb5bb3b72bad2c729227537979d95af75978f"
```

**Target state (apply all in one edit):**

```toml
# DEPS-04
interprocess = { version = "2.4.2" }

# DEPS-01
reqwest = { version = "0.12.28", features = ["blocking", "json", "socks"] }

# DEPS-02 (new entry — add after reqwest)
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }

# DEPS-05
toml = { version = "0.8" }

# DEPS-07 (new entry — add after toml)
sha2 = { version = "0.10.8" }

# DEPS-06 floem — attempt crates.io first; fall back to git+rfd-tokio if compile fails
[workspace.dependencies.floem]
version  = "0.2.0"
features = ["editor", "serde", "default-image-formats", "rfd-tokio"]

# DEPS-06 tracing family — replace all four git-rev stanzas
[workspace.dependencies.tracing]
version = "0.1.44"

[workspace.dependencies.tracing-log]
version = "0.2.0"

[workspace.dependencies.tracing-subscriber]
version  = "0.3.23"
features = ["fmt", "env-filter", "registry"]

[workspace.dependencies.tracing-appender]
version = "0.2.5"

# DEPS-06 alacritty_terminal
[workspace.dependencies.alacritty_terminal]
version = "0.24.1"
```

---

### `lapce-app/Cargo.toml` (config)

**Analog:** self — read lines 62–63 before editing.

**Current state:**
```toml
# line 62
sha2 = { version = "0.10.8" }
# line 63
zip  = { version = "0.6.6", default-features = false, features = ["deflate"] }
```

**Target state:**
```toml
# DEPS-07: sha2 now from workspace
sha2 = { workspace = true }
# DEPS-03: zip CVE fix
zip  = { version = "2.4.0", default-features = false, features = ["deflate"] }
```

---

### `lapce-app/src/app.rs` (IPC call-site rewrite, request-response)

**Analog:** self — read lines 4095–4167 before editing (the three IPC call sites).

**Current state** (lines 4095–4167):
```rust
// get_socket — client connect (line 4095)
pub fn get_socket() -> Result<interprocess::local_socket::LocalSocketStream> {
    let local_socket = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    let socket =
        interprocess::local_socket::LocalSocketStream::connect(local_socket)?;
    Ok(socket)
}

// try_open_in_existing_process — arg type (line 4103)
pub fn try_open_in_existing_process(
    mut socket: interprocess::local_socket::LocalSocketStream,
    ...

// listen_local_socket — server listener (lines 4131–4141)
fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let local_socket = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    if local_socket.exists() {
        if let Err(err) = std::fs::remove_file(&local_socket) {
            tracing::error!("{:?}", err);
        }
    }
    let socket =
        interprocess::local_socket::LocalSocketListener::bind(local_socket)?;
    for stream in socket.incoming().flatten() { ...
```

**Target state (copy this pattern exactly — DEPS-04):**
```rust
// New imports to add at the existing interprocess import site:
use interprocess::local_socket::{
    GenericFilePath,
    ListenerOptions,
    Stream,
    prelude::*,   // brings ToFsName into scope
};

// get_socket — rewritten
pub fn get_socket() -> Result<Stream> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    let name = path.to_fs_name::<GenericFilePath>()?;
    let stream = Stream::connect(name)?;
    Ok(stream)
}

// try_open_in_existing_process — update arg type only; body unchanged
pub fn try_open_in_existing_process(
    mut socket: Stream,
    paths: &[PathObject],
) -> Result<()> { ...

// listen_local_socket — rewritten (manual remove-before-bind retained)
fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let path = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    if path.exists() {
        if let Err(err) = std::fs::remove_file(&path) {
            tracing::error!("{:?}", err);
        }
    }
    let name = path.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_sync()?;
    for stream in listener.incoming().filter_map(|r| r.ok()) { ...
```

**Body of `listen_local_socket` inner loop is unchanged** — `BufReader::new(stream)`,
`lapce_rpc::stdio::read_msg`, `stream_ref.write_all(b"received")`,
`stream_ref.flush()` all compile against `interprocess::local_socket::Stream`
because `Stream` implements `Read + Write`.

---

### `lapce-app/src/app/logging.rs` (logging shim, DEPS-06)

**Analog:** self — read lines 1–64 before editing (the entire file is the context).

**Current state — two lines that do not compile against stable tracing-subscriber 0.3.x:**
```rust
// line 34: reload::Subscriber does not exist in stable 0.3.x
let (log_file_filter, reload_handle) =
    reload::Subscriber::new(log_file_filter_targets);

// lines 50, 59: fmt::Subscriber — exists as a type alias in 0.3.18+
// but may need to be fmt::Layer if the alias was removed in 0.3.23
fmt::Subscriber::default()
```

**Target state (DEPS-06 tracing rename):**
```rust
// line 34 — mechanical rename, one token change
let (log_file_filter, reload_handle) =
    reload::Layer::new(log_file_filter_targets);

// lines 50, 59 — change only if compile fails; otherwise leave as-is
// Preferred fallback if fmt::Subscriber alias is gone in 0.3.23:
fmt::Layer::default()
```

All other usage in the file (`tracing_appender::rolling::Builder::new()`,
`Rotation::DAILY`, `non_blocking()`, `WorkerGuard`, `filter::Targets`,
`LevelFilter`, `tracing_subscriber::registry()`) is API-stable and requires no
changes.

---

### `lapce-app/src/update.rs` (zip call site, file-I/O)

**Analog:** self (line 135) and `lapce-app/src/app/grammars.rs:129` — both show
the identical `ZipArchive::new(reader)` + `archive.extract(path)` pattern.

**Current state** (line 125–136):
```rust
#[cfg(all(target_os = "windows", feature = "portable"))]
pub fn extract(src: &Path, process_path: &Path) -> Result<PathBuf> {
    // ...
    let mut archive = zip::ZipArchive::new(std::fs::File::open(src)?)?;
    archive.extract(parent)?;
```

**Target state:** No code change required. The `ZipArchive::new` / `extract`
API is identical in zip 2.4.0. The Cargo.toml version bump in
`lapce-app/Cargo.toml` is the only change.

---

### `lapce-app/src/app/grammars.rs` (zip call site, file-I/O)

**Analog:** self (line 128–130) and `lapce-app/src/update.rs:135`.

**Current state** (lines 128–130):
```rust
if asset.name.ends_with(".zip") {
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(&dir)?;
```

**Target state:** No code change required. Same reasoning as `update.rs`.

---

### `lapce-app/src/app.rs` `#[cfg(test)] mod tests` *(inline regression test)*

**Analog:** `lapce-proxy/src/cli.rs` lines 93–219

This is the closest existing test file: same crate style (`#[cfg(test)] mod tests`),
plain `#[test]` functions, `use super::*`, assertions via `assert_eq!`, platform
guards via `#[cfg(windows)]` / `#[cfg(unix)]`.

**Test idiom to copy** (`lapce-proxy/src/cli.rs:93–135`):
```rust
#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use super::parse_file_line_column;
    use crate::cli::PathObject;

    #[test]
    #[cfg(windows)]
    fn test_absolute_path() {
        assert_eq!(
            parse_file_line_column("C:\\Cargo.toml:55").unwrap(),
            PathObject::new(PathBuf::from("C:\\Cargo.toml"), false, 55, 1),
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_absolute_path() {
        assert_eq!(
            parse_file_line_column("/tmp/Cargo.toml:55").unwrap(),
            PathObject::new(PathBuf::from("/tmp/Cargo.toml"), false, 55, 1),
        );
    }
}
```

**What the IPC regression test must cover:**

The test should verify that `get_socket()` + `listen_local_socket()` can complete
a round-trip using the new interprocess 2.x API. Because the listener spawns a
real OS socket, the test requires a tmpdir and `#[cfg(unix)]` / `#[cfg(windows)]`
guards. The test module placement options are:

1. Inline `#[cfg(test)] mod tests` block appended to `lapce-app/src/app.rs` — matches
   the `snippet.rs` / `condition.rs` in-file pattern.
2. Separate file `lapce-app/src/tests/ipc_roundtrip.rs` with `#[path]` attribute —
   use this if the test requires imports not already in scope in `app.rs`.

**Assertion style from analog:**
```rust
// from lapce-proxy/src/cli.rs:103–106
assert_eq!(
    parse_file_line_column("C:\\Cargo.toml:55").unwrap(),
    PathObject::new(PathBuf::from("C:\\Cargo.toml"), false, 55, 1),
);
```

For IPC: assert `Result::is_ok()` on connect, assert `b"received"` bytes echoed
back, assert clean shutdown (listener drops without panic).

---

### `lapce-app/src/update.rs` — inline `#[cfg(test)] mod tests` *(zip regression test)*

**Analog:** `lapce-core/src/encoding.rs` lines 98–147

Flat `#[cfg(test)] mod tests` with `use crate::...` import, multiple `#[test]`
functions, `assert_eq!` + `assert!(result.is_ok())` assertions.

**Test idiom to copy** (`lapce-core/src/encoding.rs:98–115`):
```rust
#[cfg(test)]
mod tests {
    use crate::encoding::{offset_utf8_to_utf16_str, offset_utf16_to_utf8_str};

    #[test]
    fn utf8_to_utf16() {
        let text = "hello world";
        assert_eq!(offset_utf8_to_utf16_str(text, 0), 0);
        assert_eq!(offset_utf8_to_utf16_str("", 0), 0);
        // ...
    }
}
```

**What the zip regression test must cover:**

Verify CVE-2025-29787 is closed by confirming `ZipArchive::extract()` in zip 2.4.0
rejects a `../` path traversal entry. The test must:

1. Build a minimal in-memory ZIP with a crafted entry name (`../escape.txt`) using
   `zip::write::ZipWriter`.
2. Call `ZipArchive::extract(tmpdir)` on it.
3. Assert the result is `Err(_)` (traversal rejected) OR assert the file was not
   written outside `tmpdir`.

**Module placement:** Inline `#[cfg(test)] mod tests` at the bottom of
`lapce-app/src/update.rs` (mirrors the in-file convention used throughout the
codebase), or inline in `lapce-app/src/app/grammars.rs` for the grammar-download
path.

---

## Shared Patterns

### Error handling in modified files

**Source:** Throughout `lapce-app/src/app.rs` and `lapce-proxy/src/lib.rs`

All call sites use `?` propagation with `anyhow::Result` and
`.ok_or_else(|| anyhow!("message"))` for `Option`-to-`Result` conversion. No
`.unwrap()` in production paths. Retain exactly this pattern in the rewritten IPC
functions.

```rust
// Pattern from app.rs:4096-4097
let local_socket = Directory::local_socket()
    .ok_or_else(|| anyhow!("can't get local socket folder"))?;
```

### Logging pattern in modified files

**Source:** `lapce-app/src/app.rs:4136–4138`

```rust
if let Err(err) = std::fs::remove_file(&local_socket) {
    tracing::error!("{:?}", err);
}
```

Use `tracing::error!("{:?}", err)` (not `eprintln!`, not `log::error!`) for all
non-fatal errors in the rewritten IPC code.

### Test `use` imports

**Source:** `lapce-proxy/src/cli.rs:95–98`

```rust
use std::{env, path::PathBuf};
use super::parse_file_line_column;
use crate::cli::PathObject;
```

New tests use `use super::*` or explicit `use super::function_name` — never
qualified paths inside the test body.

---

## No Analog Found

All files in Phase 1 have direct analogs. No file requires falling back to
RESEARCH.md patterns exclusively.

---

## Metadata

**Analog search scope:** `lapce-app/src/`, `lapce-proxy/src/`, `lapce-core/src/`,
`lapce-rpc/src/`, `Cargo.toml`, `lapce-app/Cargo.toml`

**Files scanned:** 13 source files + 2 Cargo manifests

**Pattern extraction date:** 2026-06-07
