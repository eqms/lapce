# External Integrations

**Analysis Date:** 2026-06-07

## APIs & External Services

**GitHub Releases API (auto-updater):**
- Purpose: Check for and download new Lapce releases
- Endpoints:
  - Stable: `https://api.github.com/repos/lapce/lapce/releases/latest`
  - Nightly: `https://api.github.com/repos/lapce/lapce/releases/tags/nightly`
  - Proxy binary download: `https://github.com/lapce/lapce/releases/download/{version}/{filename}.gz`
- Implementation: `lapce-app/src/update.rs`
- HTTP client: `reqwest` v0.11 (blocking)
- User-Agent: `"Lapce"` passed to `lapce_proxy::get_url()`
- Triggered by: `updater` Cargo feature (enabled by default)

**GitHub Releases API (tree-sitter grammars):**
- Purpose: Download syntax highlighting grammar `.so`/`.dll` files at runtime
- Endpoint: `https://api.github.com/repos/lapce/tree-sitter-grammars/releases?per_page=100`
- Implementation: `lapce-app/src/app/grammars.rs`
- HTTP client: `reqwest` v0.11 (blocking, via proxy)

**Lapce Plugin Registry API:**
- Purpose: Browse, search, and install plugins (Volts)
- Endpoints:
  - Plugin icon: `https://plugins.lapce.dev/api/v1/plugins/{author}/{name}/{version}/icon?id={id}`
  - Plugin readme: `https://plugins.lapce.dev/api/v1/plugins/{author}/{name}/{version}/readme`
  - Plugin search: `https://plugins.lapce.dev/api/v1/plugins?q={query}&offset={offset}`
  - Plugin download: `https://plugins.lapce.dev/api/v1/plugins/{author}/{name}/{version}/download`
- Implementation (app side): `lapce-app/src/plugin.rs`
- Implementation (proxy side): `lapce-proxy/src/plugin/mod.rs`
- HTTP client: `reqwest` v0.11, with optional SOCKS proxy support

## Language Server Protocol (LSP)

**Protocol:**
- LSP (Language Server Protocol) — industry-standard editor ↔ language server JSON-RPC protocol
- Types: `lsp-types` v0.95.1 (patched from `https://github.com/lapce/lsp-types`)
- Implementation: `lapce-proxy/src/plugin/lsp.rs`
- Servers are external processes launched by plugins; communication via stdin/stdout

## Debug Adapter Protocol (DAP)

**Protocol:**
- DAP (Debug Adapter Protocol) — VS Code standard debugger protocol
- Types: `lapce-rpc/src/dap_types.rs`
- Implementation: `lapce-proxy/src/plugin/dap.rs`
- Debug adapters are external processes; communication via stdin/stdout

## Plugin System (WASM/WASI)

**Runtime:**
- WebAssembly plugins via `wasmtime` v14.0.2 + `wasmtime-wasi`
- HTTP pass-through for plugins: `wasi-experimental-http-wasmtime` (git dep)
- Plugin format: `.wasm` files distributed via `plugins.lapce.dev`
- Implementation: `lapce-proxy/src/plugin/wasi.rs`, `lapce-proxy/src/plugin/catalog.rs`
- PSP (Plugin Server Protocol) types: `psp-types` (git from `https://github.com/lapce/psp-types`)

## Source Control

**Git Integration:**
- `git2` v0.20.0 — libgit2 Rust bindings
- Features: `vendored-openssl` (ships its own OpenSSL)
- Usage: File status, blame, diff, branch info in source control panel
- Implementation files: `lapce-app/src/source_control.rs`, used throughout `lapce-proxy/src/dispatch.rs`
- Remote URL detection for GitLab/GitHub file links: `lapce-proxy/src/dispatch.rs` (constructs blob URLs)

## Remote Development

**SSH:**
- Connection via system `ssh` binary (not a Rust SSH library)
- ControlMaster/ControlPath multiplexing: `~/.ssh/cm_%C`
- Uploads proxy binary to remote host via `scp`
- Implementation: `lapce-app/src/proxy/ssh.rs`
- Struct: `SshRemote { ssh: SshHost }`

**WSL (Windows Subsystem for Linux):**
- Implementation: `lapce-app/src/proxy/wsl.rs`
- Runs proxy inside WSL distribution

**Remote proxy bootstrap:**
- Detects remote platform/arch, downloads correct proxy binary from GitHub releases, uploads, and starts
- Implementation: `lapce-app/src/proxy/remote.rs`

## Data Storage

**Databases:**
- No SQL database
- Custom file-based persistence: JSON files written to OS config directory
- `LapceDb` struct (`lapce-app/src/db.rs`) serializes state to flat JSON files:
  - `app` — application state
  - `window` — window positions/state
  - `workspace_info` — per-workspace state
  - `workspace_files` — open files per workspace
  - `panel_orders` — panel layout
  - `disabled_volts` — disabled plugins
  - `recent_workspaces` — recent workspace list
- Write path: async via `crossbeam-channel` (dedicated `SaveEventHandler` thread)
- Config directory resolved via `directories` crate → `lapce-core/src/directory.rs`

**File Storage:**
- Local filesystem only
- Plugin installations: OS config dir `/grammars/`, `/plugins/`
- Auto-update downloads: OS config dir `/updates/`

**Caching:**
- None (no Redis/Memcached)
- File-level caching only (tree-sitter parse trees held in memory)

## Authentication & Identity

**Auth Provider:**
- None — no user accounts or cloud authentication
- Plugin registry access is unauthenticated (public API)
- GitHub API used unauthenticated (rate-limited to 60 req/hr)

## Monitoring & Observability

**Error Tracking:**
- No external service (no Sentry, Datadog, etc.)
- Crash notification shown to user on Unix (see recent commit `30cfb663`)

**Logs:**
- `tracing` framework (pinned git from `tokio-rs/tracing`)
- Log files written via `tracing-appender` to OS log/config directory
- Structured logging with `tracing-subscriber`

## CI/CD & Deployment

**Hosting:**
- GitHub Releases (binaries: `.dmg`, `.tar.gz`, `.msi`, `.zip`)
- Plugin registry hosted at `plugins.lapce.dev` (external service, not in this repo)

**CI Pipeline:**
- GitHub Actions: `.github/workflows/ci.yml`
  - Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`
  - Jobs: `fmt` (rustfmt + typos), `clippy`, `build` (compile + doc tests)
  - Cache: `Swatinem/rust-cache`
- GitHub Actions: `.github/workflows/release.yml`
  - Triggers: daily schedule (nightly), version tags (`v*.*.*`), manual dispatch
  - Produces: platform-specific installers uploaded to GitHub Releases

**Dependabot:**
- Config: `.github/dependabot.yml`

## HTTP Proxy Support

- `reqwest` configured to support SOCKS proxies
- `lapce-proxy/src/lib.rs` → `get_url(url, user_agent)` wraps reqwest and applies optional proxy settings from user config

## Webhooks & Callbacks

**Incoming:** None

**Outgoing:** None (no webhook endpoints defined)

## Environment Configuration

**Required at build time:**
- `CARGO_PKG_VERSION` — embedded in binary via `env!()` macro
- `OUT_DIR` — Cargo output dir for `meta.rs` code generation (`lapce-core/src/meta.rs`)

**Runtime config files (user-editable):**
- `~/.config/lapce/settings.toml` (Linux) / equivalent per OS
- `~/.config/lapce/keymaps.toml`
- Defaults embedded via `include_dir!` macro at compile time from `defaults/`

**No secrets or API keys required** — all external APIs are public/unauthenticated.

---

*Integration audit: 2026-06-07*
