# Technology Stack

**Analysis Date:** 2026-06-07

## Languages

**Primary:**
- Rust (Edition 2024) - All application code across all workspace crates

**Secondary:**
- TOML - Configuration files, themes, keymaps (`defaults/`, `Cargo.toml`)

## Runtime

**Environment:**
- Native binary — no interpreter or managed runtime
- Minimum Rust version: 1.87.0 (enforced via `rust-version` in `Cargo.toml`)

**Package Manager:**
- Cargo (workspace layout)
- Lockfile: `Cargo.lock` present and committed

## Workspace Crates

The repository is a Cargo workspace with four members:

| Crate | Path | Purpose |
|-------|------|---------|
| `lapce` | root `Cargo.toml` | Binary entry point — bootstraps `lapce-app` + `lapce-proxy` |
| `lapce-app` | `lapce-app/` | Full GUI application, UI rendering, plugin management, config |
| `lapce-proxy` | `lapce-proxy/` | Headless backend: LSP, DAP, file I/O, WASM plugin host |
| `lapce-rpc` | `lapce-rpc/` | Shared RPC message types between app and proxy |
| `lapce-core` | `lapce-core/` | Syntax highlighting, language definitions, rope text model |

Entry points:
- `lapce-app/src/bin/lapce.rs` — main GUI binary
- `lapce-proxy/src/bin/lapce-proxy.rs` — headless proxy binary

## Frameworks

**GUI / Rendering:**
- `floem` v0.2.0 — Lapce's own reactive UI framework (pinned git rev `31fa8f4`)
  - Source: `https://github.com/lapce/floem`
  - Features: `editor`, `serde`, `default-image-formats`, `rfd-async-std`
  - Internally uses `wgpu` (GPU rendering), `tiny-skia` (software fallback), `vello`/`peniko`, `softbuffer`
- `floem-editor-core` — editor primitives from the same floem repo

**Terminal Emulation:**
- `alacritty_terminal` v0.24.1-dev — pinned git rev from `https://github.com/alacritty/alacritty`

**Syntax Highlighting:**
- `tree-sitter` v0.22.6 — incremental parsing library
- Grammars downloaded at runtime from `https://github.com/lapce/tree-sitter-grammars`

**Plugin Host (WASM):**
- `wasmtime` v14.0.2 — WebAssembly runtime for plugins
- `wasmtime-wasi` v14.0.2
- `wasi-common` v14.0.2
- `wasi-experimental-http-wasmtime` — git dep from `https://github.com/lapce/wasi-experimental-http`

**Text / Rope:**
- `lapce-xi-rope` v0.3.2 — rope data structure for text editing

**Testing (dev):**
- `criterion` v0.5 — benchmarking (used in `lapce-app`)

**Build/Dev:**
- `rustfmt` — formatting enforced in CI; config: `.rustfmt.toml` (`max_width = 85`)
- `clippy` — linting enforced in CI
- `typos` (crate-ci/typos) — spell checking in CI

## Key Dependencies

**Critical:**
- `lsp-types` v0.95.1 — LSP protocol types (patched via git: `https://github.com/lapce/lsp-types`)
- `psp-types` — Plugin Server Protocol types (git: `https://github.com/lapce/psp-types`)
- `git2` v0.20.0 — libgit2 bindings for source control panel (features: `vendored-openssl`)
- `reqwest` v0.11 — HTTP client for plugin downloads, auto-update, GitHub API calls (features: `blocking`, `json`, `socks`)
- `serde` / `serde_json` v1.0 — serialization throughout
- `crossbeam-channel` v0.5.12 — message passing between threads
- `parking_lot` v0.12.3 — synchronization primitives

**Tracing / Observability:**
- `tracing`, `tracing-log`, `tracing-subscriber`, `tracing-appender` — all pinned to git rev `908cc43` from `https://github.com/tokio-rs/tracing`

**Search:**
- `ignore` v0.4 — gitignore-aware file walking
- `grep-searcher`, `grep-matcher`, `grep-regex` — ripgrep search engine crates
- `nucleo` v0.5.0 — fuzzy matching for palette

**Config / TOML:**
- `config` v0.13.4 (pinned) — layered config loading
- `toml` + `toml_edit` v0.20.2 — TOML parsing/editing

**UI extras:**
- `pulldown-cmark` v0.11.0 — Markdown rendering for hover docs
- `Inflector` v0.11.4 — string inflection utilities
- `open` v5.1.4 — open URLs/files in OS default app
- `unicode-width` v0.1.13 — terminal-accurate string widths

**Crypto/Encoding:**
- `sha2` v0.10.8 — content hashing
- `base64` v0.21.7 — encoding
- `zip` v0.6.6 — zip archive support

**Platform:**
- `windows-sys` v0 — Win32 API bindings (Windows only)
- `dmg` v0.1.1 + `fs_extra` v1.2.0 — macOS DMG mount/copy (macOS only)
- `locale_config` — git dep, macOS locale detection (`https://github.com/lapce/locale_config.git`)
- `libc` v0.2 — C bindings
- `interprocess` v1.2.1 — IPC (named pipes / Unix domain sockets)

## Configuration

**Environment:**
- No `.env` files — purely compile-time and runtime file-based config
- Runtime config stored in OS config directory via `directories` crate
- Settings files: `defaults/settings.toml`, `defaults/keymaps-*.toml`, `defaults/dark-theme.toml`, `defaults/light-theme.toml`

**Build:**
- `Cargo.toml` (workspace root) — all version pins and workspace dependencies
- `.cargo/config.toml` — MSVC static CRT linkage; `ci` profile definition
- `Makefile` — macOS-specific targets: `binary`, `app`, `dmg` (uses `lipo`, `codesign`, `hdiutil`)

## Build Profiles

| Profile | Purpose |
|---------|---------|
| `dev` | Default development build |
| `release` | Optimized release |
| `release-lto` | Release + LTO + single codegen unit (production distribution) |
| `fastdev` | Dev for Lapce code, release-optimized for all deps |
| `ci` | CI builds: no debug info, no optimization |

## Platform Requirements

**Development (Ubuntu):**
```bash
sudo apt-get install -y clang libxkbcommon-x11-dev pkg-config libvulkan-dev \
  libgtk-3-dev libwayland-dev xorg-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

**Production targets:**
- Windows (MSVC, portable zip or MSI installer)
- macOS (universal binary via `lipo`, DMG via `hdiutil`)
- Linux x86_64 / aarch64 (tar.gz)
- FreeBSD / OpenBSD (partial support in update code)

---

*Stack analysis: 2026-06-07*
