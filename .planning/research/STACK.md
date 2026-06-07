# Stack Research

**Domain:** Rust desktop code editor — hardening milestone (async runtime, dep upgrades, integrity verification)
**Researched:** 2026-06-07
**Confidence:** HIGH (versions verified against crates.io API; rationale cross-checked with official docs and community sources)

---

## Recommended Stack

### Async Runtime

**Verdict: tokio 1.x — run on a dedicated background thread, not nested inside Floem's event loop.**

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `tokio` | `"1"` (1.52.3 current; 1.51.x LTS until 2027-03) | Async runtime for all network I/O (plugin download, self-update, proxy download) | Ecosystem lock-in: `reqwest` 0.12+ requires a Tokio context; `interprocess` 2.x tokio module requires Tokio; the tracing crate is already a tokio-rs project. Using smol would require async-channel bridges everywhere reqwest or interprocess tokio is touched. |

**Why not smol:** smol is lighter and more event-loop-friendly in theory. In practice, `reqwest` 0.12+ uses hyper 1 which is built on tokio I/O primitives and will panic if called outside a tokio runtime. Since reqwest is non-negotiable for this codebase, smol would require either a tokio compatibility shim (tokio-async-std bridge) or replacing reqwest — both are higher-risk than just adopting tokio. Smol's advantage (manual tick, no forced executor) is irrelevant here because Floem's event loop does not need to own the async executor.

**Floem interaction (critical):** Floem's own event loop is synchronous and owns the main thread. Its optional `tokio` feature flag (for `rfd` file dialogs) requires `features = ["sync", "rt"]` — it does not use `tokio::main` or a multi-threaded scheduler. The correct integration pattern is:

```rust
// In lapce binary entry point, before starting Floem:
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)   // network I/O only; keep footprint small
    .enable_io()
    .enable_time()
    .build()
    .expect("failed to build tokio runtime");
let _guard = rt.enter(); // makes rt ambient for reqwest::Client, etc.
// Now launch floem::launch(app_view) on main thread as usual
```

The `rt.enter()` guard makes the tokio context ambient so `reqwest::Client` and `tokio::spawn` work from any thread without `#[tokio::main]`. Floem never calls `block_on` or `Runtime::block_on` itself, so there is no nested-runtime footgun as long as lapce does not call `tokio::runtime::Handle::block_on` from inside a tokio async task (use `tokio::spawn` + channels instead).

**Floem feature flag:** Change `rfd-async-std` → `rfd-tokio` in the floem dependency once tokio is introduced, so the file dialog and network runtime agree.

**Minimum features for lapce:**
```toml
tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync", "macros"] }
```
Do not use `features = ["full"]` — it pulls in tokio's signal, process, and test utilities which are not needed and inflate the binary.

---

### HTTP Client Upgrade

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `reqwest` | `"0.12"` (0.12.28 latest in track) | HTTP client for plugin/update/proxy downloads | 0.12 is the stable hyper-1 track; API is nearly identical to 0.11 for the features lapce uses (GET, JSON, SOCKS proxy). Upgrade to `0.13` is deferred — see rationale below. |

**Why pin to 0.12, not 0.13:**
- reqwest 0.13 (released 2026-01) changes the default TLS backend from `native-tls` to `rustls` with `aws-lc` as the crypto provider. This requires `aws-lc-sys` which needs cmake and a C compiler on every platform — a meaningful CI and cross-compilation burden for a desktop app already using `vendored-openssl` for `git2`.
- reqwest 0.13 also makes `query` and `form` features opt-in (disabled by default), which may silently break any query-string construction.
- 0.12 still receives security patches (0.12.28 released 2025-12-22).
- Adopt 0.13 in a dedicated dependency milestone after verifying cmake availability on all build targets.

**Migration from 0.11 → 0.12:** Remove `reqwest::blocking::` from all call sites. Replace with `tokio::spawn` + `reqwest::Client` (async). The `get_url` function in `lapce-proxy/src/lib.rs` becomes the primary migration target. Feature flags `json` and `socks` are unchanged. Drop the `blocking` feature flag.

```toml
# workspace Cargo.toml
reqwest = { version = "0.12", features = ["json", "socks"] }
# remove "blocking" from features
```

---

### IPC Library Upgrade

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `interprocess` | `"2"` (2.4.2 current) | Single-instance detection, local socket IPC between app instances | The 1.2.1 sync API is available in 2.x unchanged as the default; no forced async adoption. MSRV bumped to 1.75 (lapce already requires 1.87). The name/trait API changed — migration is mechanical. |

**API migration — what changes:**

In 1.2.1, bind/connect take a `PathBuf` or `&str` directly:
```rust
LocalSocketListener::bind(local_socket)?
LocalSocketStream::connect(local_socket)?
```

In 2.x, the name must be converted via the `ToFsName` trait:
```rust
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::GenericFilePath;

let name = local_socket.to_fs_name::<GenericFilePath>()?;
LocalSocketListener::bind(name)?
LocalSocketStream::connect(name)?
```

The `incoming()` iterator, `BufReader` wrapping, and `Read`/`Write` traits on `LocalSocketStream` are all preserved in 2.x sync API. Call sites in `app.rs` (`get_socket`, `listen_local_socket`) require only the name conversion plus `use interprocess::local_socket::prelude::*`.

**Interprocess tokio async module:** Available behind the `tokio` feature flag but NOT required for lapce's use case. The existing sync pattern (blocking reads on background threads) is correct for single-instance detection. Do not enable the interprocess `tokio` feature — the tokio async local socket types panic outside a tokio context if misused, and the sync API is sufficient.

```toml
interprocess = { version = "2", default-features = true }
# Do NOT add features = ["tokio"] unless lapce-proxy needs async IPC in a future milestone
```

---

### SHA256 Integrity Verification

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `sha2` | `"0.10"` (0.10.9 latest stable in 0.10 track) | SHA256 hash computation for integrity verification | Already in `lapce-app/Cargo.toml` at 0.10.8; already used in `db.rs` and `plugin.rs` via `sha2::{Digest, Sha256}`. Zero new dependencies. The `0.11.0` release (2026-03-25) is new — hold at 0.10 until 0.11 stabilizes further and downstream crates catch up. |

**Usage pattern (already established in codebase):**
```rust
use sha2::{Digest, Sha256};
let mut hasher = Sha256::new();
hasher.update(&downloaded_bytes);
let hash = hasher.finalize();
let hex = format!("{:x}", hash);
// compare against published hash string, reject if mismatch
```

Move the `sha2` dependency to workspace-level (`Cargo.toml` `[workspace.dependencies]`) so `lapce-proxy` can use it without a separate pin.

**Why not ring:** `ring` is a heavier dependency (C code via bindgen, FIPS concerns, larger binary footprint). For simple hash verification, `sha2` from RustCrypto is idiomatic, pure Rust, and already present.

---

### Safe Archive Extraction

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `zip` | `"2"` (8.6.0 current — note: major version is the version number; the semver is 8.x) | ZIP extraction for self-update and grammar archives | CVE-2025-29787 (symlink path traversal) was present in 1.3.0–2.2.0; fixed in 2.3.0. Current 8.6.0 (2026-04-25) is fully patched. The path-safety fix uses `ZipFile::enclosed_name` internally — no manual entry-path validation needed at 8.x. |

**Important:** Lapce pins `zip = "0.6.6"`. The major version bumped significantly. The 0.6.x API is **not** safe against CVE-2025-29787. Upgrade is security-mandatory.

```toml
# lapce-app/Cargo.toml
zip = { version = "2", default-features = false, features = ["deflate"] }
```

**API change:** `ZipArchive::new` and iteration are API-stable. `ZipFile::enclosed_name()` (returns `Option<&Path>`) replaces manual path validation — callers should check `None` (path traversal detected) and skip or error. Review `update.rs:135` and `app/grammars.rs:129` to add the `enclosed_name` guard explicitly even though 8.x validates internally — defense-in-depth.

**For tar.gz (plugin downloads via zstd):** No new crate needed. The existing `tar` + `zstd` path in `lapce-proxy/src/plugin/mod.rs` should validate each entry path against the target directory manually using `entry.path()?.components()` — reject any component that is `..` or starts with `/`. This is two lines of validation, not a new crate.

---

### Dependency Pinning Cleanup

#### toml

| Current | Recommended | Rationale |
|---------|-------------|-----------|
| `"*"` | `"0.8"` | Wildcard accepts breaking major versions silently. The 1.x line (current latest 1.1.2) has API differences from 0.8.x. Pin to `"0.8"` first as a safe mechanical change; upgrade to `"1"` in a dedicated dependency milestone after auditing all `toml::` call sites. |

```toml
toml = { version = "0.8", features = ["display"] }
```

Note: `toml_edit` is already pinned to `"0.20.2"` which is in the 0.8-compatible family. Verify they remain compatible if/when upgrading to toml 1.x.

#### tracing (and tracing-log, tracing-subscriber, tracing-appender)

Current state: all four crates pinned to git SHA `908cc43` from `https://github.com/tokio-rs/tracing`.

| Crate | Latest crates.io | Notes |
|-------|-----------------|-------|
| `tracing` | 0.1.44 | Stable, actively maintained |
| `tracing-subscriber` | 0.3.x (latest in 0.3 track) | API used in lapce is stable |
| `tracing-log` | 0.2.x | |
| `tracing-appender` | 0.2.x | |

```toml
# Replace all four git+SHA blocks with:
tracing            = { version = "0.1" }
tracing-log        = { version = "0.2" }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender   = { version = "0.2" }
```

Confidence: HIGH — the 0.1 tracing API has been stable since 2021; the git SHA was likely pinned to catch a pre-release fix that is now in a tagged release.

#### alacritty_terminal

Current state: git SHA from `https://github.com/alacritty/alacritty`, described as `v0.24.1-dev`.

| Status | Detail |
|--------|--------|
| crates.io latest | 0.26.0 (2026) |
| lapce SHA equivalent | Approximately 0.24.x |

Switch to `alacritty_terminal = "0.26"`. Verify that the terminal API used by lapce's `lapce-app/src/terminal/` compiles cleanly — the alacritty_terminal API does see breaking changes between minor versions. If 0.26 breaks, pin to `"0.25"` (0.25.1, Oct 2025) as a stable fallback.

#### floem + floem-editor-core

Current state: git SHA `31fa8f4` from `https://github.com/lapce/floem`. The floem crate is available on crates.io at `0.2.0` — this is the latest tagged release.

```toml
[workspace.dependencies.floem]
version  = "0.2"
features = ["editor", "serde", "default-image-formats", "rfd-tokio"]
# Change rfd-async-std → rfd-tokio once tokio is introduced
```

However: floem 0.2.0 on crates.io may lag behind the git HEAD used by lapce (SHA `31fa8f4`). Before switching, verify the 0.2.0 tag includes all features lapce relies on — in particular the `editor` and `rfd-*` features. If the SHA is ahead of the tag, keep the git pin but add `tag = "v0.2.0"` instead of `rev = "31fa8f4..."`. Check whether the current SHA post-dates the 0.2.0 tag:

```bash
cd /tmp && git clone --no-checkout https://github.com/lapce/floem
git -C floem log --oneline v0.2.0..31fa8f444c37f4c314f47d88c23ffdbc25f2ab53
```
If the SHA is ahead, stay on git+rev for now. Tag pinning is still better than arbitrary SHA — create a `v0.2.1` tag on the floem repo if the delta is small.

#### psp-types

Current state: git pin from `https://github.com/lapce/psp-types`. The crate is available on crates.io at `0.1.1`.

```toml
psp-types = { version = "0.1" }
```

Verify the 0.1.1 release matches the SHA being used. Since lapce owns this repo, cutting a new tag is trivial if there are unreleased changes.

---

## Version Compatibility Matrix

| Crate | Requires | Notes |
|-------|----------|-------|
| `reqwest = "0.12"` | `tokio = "1"` | reqwest 0.12 will panic without a tokio runtime context |
| `interprocess = "2"` | `rust-version = "1.75"` | lapce requires 1.87, no conflict |
| `interprocess` tokio module | `tokio = "1"` | Only if `features = ["tokio"]` is enabled — not recommended for this milestone |
| `zip = "2"` | None (pure Rust) | CVE-2025-29787 patched at 2.3.0; 8.x is current |
| `sha2 = "0.10"` | None | Pure Rust; already in lapce-app |
| `tracing = "0.1"` | None | tokio-rs project but no runtime dep |
| `floem` with `rfd-tokio` | `tokio = "1"` with `features = ["sync", "rt"]` | floem only needs sync+rt, not rt-multi-thread |
| `alacritty_terminal = "0.26"` | Verify terminal API compatibility | May need minor call-site updates |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `reqwest::blocking` | Monopolizes an OS thread for the full download duration; incompatible with adding tokio | `reqwest::Client` (async) + `tokio::spawn` |
| `tokio` `features = ["full"]` | Pulls signal, process, test utilities not needed; inflates binary | Explicit features: `rt-multi-thread`, `net`, `time`, `sync`, `macros` |
| `#[tokio::main]` on `main()` | Incompatible with Floem owning the main thread | `tokio::runtime::Builder::new_multi_thread()` + `rt.enter()` guard, then call `floem::launch()` |
| smol/async-std | reqwest 0.12+ requires tokio; mixing runtimes causes panics; the compat bridges add complexity | tokio |
| `ring` for SHA256 | Heavy C+bindgen dependency; overkill for hash-verify-only use | `sha2 = "0.10"` (already present) |
| `zip = "0.6"` | CVE-2025-29787: symlink path traversal vulnerability; not patched until 2.3.0 | `zip = "2"` (8.6.0) |
| `toml = "*"` | Wildcard silently accepts breaking major version bump | `toml = "0.8"` |
| `reqwest = "0.13"` (this milestone) | Default TLS switch to rustls+aws-lc requires cmake on all platforms; form/query features opt-in by default | `reqwest = "0.12"` |

---

## Cargo.toml Changes Summary

```toml
# Workspace Cargo.toml changes:

# 1. ADD tokio
tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync", "macros"] }

# 2. UPGRADE reqwest (remove "blocking")
reqwest = { version = "0.12", features = ["json", "socks"] }

# 3. UPGRADE interprocess
interprocess = { version = "2" }

# 4. PIN toml (replace "*")
toml = { version = "0.8" }

# 5. REPLACE tracing git pins with versioned releases
tracing            = { version = "0.1" }
tracing-log        = { version = "0.2" }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender   = { version = "0.2" }

# 6. REPLACE floem git pin with tagged release (verify first)
# [workspace.dependencies.floem]
# version  = "0.2"
# features = ["editor", "serde", "default-image-formats", "rfd-tokio"]

# lapce-app/Cargo.toml changes:

# 7. UPGRADE zip (CVE fix, mandatory)
zip = { version = "2", default-features = false, features = ["deflate"] }

# 8. KEEP sha2 at 0.10.x (already present, move to workspace deps)
sha2 = { version = "0.10" }

# 9. psp-types: replace git pin
psp-types = { version = "0.1" }

# 10. alacritty_terminal: replace git pin (verify API compat first)
alacritty_terminal = { version = "0.26" }
```

---

## Sources

- crates.io API — version verification for all crates listed (tokio 1.52.3, reqwest 0.12.28/0.13.4, interprocess 2.4.2, sha2 0.10.9/0.11.0, zip 8.6.0, tracing 0.1.44, toml 1.1.2, alacritty_terminal 0.26.0, psp-types 0.1.1, floem 0.2.0): HIGH confidence
- [reqwest v0.13 changelog](https://seanmonstar.com/blog/reqwest-v013-rustls-default/) — TLS default switch, query/form feature changes: HIGH confidence
- [interprocess source `src/local_socket.rs`](https://github.com/kotauskas/interprocess) — sync API preserved in 2.x; tokio module is feature-gated and panics outside tokio context: HIGH confidence
- [floem/Cargo.toml main branch](https://github.com/lapce/floem/blob/main/Cargo.toml) — confirmed `rfd-tokio` and `rfd-async-std` feature flags; tokio optional dep with `features = ["sync", "rt"]`: HIGH confidence
- [CVE-2025-29787 / Snyk](https://security.snyk.io/vuln/SNYK-RUST-ZIP-9460813) — zip 0.6.6 through 2.2.x vulnerable; patched at 2.3.0+: HIGH confidence
- tokio documentation on bridging with sync code: https://tokio.rs/tokio/topics/bridging — background thread runtime pattern: HIGH confidence
- Community analysis: "The State of Async Rust: Runtimes" (corrode.dev) — smol vs tokio tradeoffs: MEDIUM confidence (cross-checked against reqwest/interprocess docs)

---

*Stack research for: Lapce hardening milestone — async runtime, dep upgrades, integrity verification*
*Researched: 2026-06-07*
