<!-- GSD:project-start source:PROJECT.md -->
## Project

**Lapce Hardening Fork**

A hardening-focused fork of [Lapce](https://github.com/lapce/lapce), the Rust-native code editor (Floem GUI, `lapce-proxy` backend, LSP/DAP support, plugins, terminal, remote SSH). This project systematically resolves the engineering-quality concerns surfaced in the codebase audit (`.planning/codebase/CONCERNS.md`): runtime panics, missing download integrity verification, performance bottlenecks, and outdated/unsafe dependency pins. The audience is the maintainer of this fork — improvements may later be offered upstream, but mergeability is a secondary goal.

**Core Value:** The editor must never panic on normal user actions, and every binary it downloads (plugin, self-update, remote proxy) must be integrity-verified before execution. Stability and supply-chain safety come first; everything else is secondary.

### Constraints

- **Tech stack**: Rust, Floem GUI, Cargo workspace — no language/framework change; fixes stay idiomatic to existing patterns (see `.planning/codebase/CONVENTIONS.md`).
- **Compatibility**: Must not break existing editor/LSP/DAP/plugin/terminal/remote behavior — these are Validated capabilities.
- **Security**: Integrity verification must fail-closed (reject + alert on mismatch), never fail-open.
- **Testing**: Every crash/security fix requires a reproducing regression test (Key Decision).
- **Dependencies**: `interprocess` 2.x and `reqwest` 0.12 upgrades carry API-migration risk; verify single-instance IPC and proxy handling still work after each bump.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (Edition 2024) - All application code across all workspace crates
- TOML - Configuration files, themes, keymaps (`defaults/`, `Cargo.toml`)
## Runtime
- Native binary — no interpreter or managed runtime
- Minimum Rust version: 1.87.0 (enforced via `rust-version` in `Cargo.toml`)
- Cargo (workspace layout)
- Lockfile: `Cargo.lock` present and committed
## Workspace Crates
| Crate | Path | Purpose |
|-------|------|---------|
| `lapce` | root `Cargo.toml` | Binary entry point — bootstraps `lapce-app` + `lapce-proxy` |
| `lapce-app` | `lapce-app/` | Full GUI application, UI rendering, plugin management, config |
| `lapce-proxy` | `lapce-proxy/` | Headless backend: LSP, DAP, file I/O, WASM plugin host |
| `lapce-rpc` | `lapce-rpc/` | Shared RPC message types between app and proxy |
| `lapce-core` | `lapce-core/` | Syntax highlighting, language definitions, rope text model |
- `lapce-app/src/bin/lapce.rs` — main GUI binary
- `lapce-proxy/src/bin/lapce-proxy.rs` — headless proxy binary
## Frameworks
- `floem` v0.2.0 — Lapce's own reactive UI framework (pinned git rev `31fa8f4`)
- `floem-editor-core` — editor primitives from the same floem repo
- `alacritty_terminal` v0.24.1-dev — pinned git rev from `https://github.com/alacritty/alacritty`
- `tree-sitter` v0.22.6 — incremental parsing library
- Grammars downloaded at runtime from `https://github.com/lapce/tree-sitter-grammars`
- `wasmtime` v14.0.2 — WebAssembly runtime for plugins
- `wasmtime-wasi` v14.0.2
- `wasi-common` v14.0.2
- `wasi-experimental-http-wasmtime` — git dep from `https://github.com/lapce/wasi-experimental-http`
- `lapce-xi-rope` v0.3.2 — rope data structure for text editing
- `criterion` v0.5 — benchmarking (used in `lapce-app`)
- `rustfmt` — formatting enforced in CI; config: `.rustfmt.toml` (`max_width = 85`)
- `clippy` — linting enforced in CI
- `typos` (crate-ci/typos) — spell checking in CI
## Key Dependencies
- `lsp-types` v0.95.1 — LSP protocol types (patched via git: `https://github.com/lapce/lsp-types`)
- `psp-types` — Plugin Server Protocol types (git: `https://github.com/lapce/psp-types`)
- `git2` v0.20.0 — libgit2 bindings for source control panel (features: `vendored-openssl`)
- `reqwest` v0.11 — HTTP client for plugin downloads, auto-update, GitHub API calls (features: `blocking`, `json`, `socks`)
- `serde` / `serde_json` v1.0 — serialization throughout
- `crossbeam-channel` v0.5.12 — message passing between threads
- `parking_lot` v0.12.3 — synchronization primitives
- `tracing`, `tracing-log`, `tracing-subscriber`, `tracing-appender` — all pinned to git rev `908cc43` from `https://github.com/tokio-rs/tracing`
- `ignore` v0.4 — gitignore-aware file walking
- `grep-searcher`, `grep-matcher`, `grep-regex` — ripgrep search engine crates
- `nucleo` v0.5.0 — fuzzy matching for palette
- `config` v0.13.4 (pinned) — layered config loading
- `toml` + `toml_edit` v0.20.2 — TOML parsing/editing
- `pulldown-cmark` v0.11.0 — Markdown rendering for hover docs
- `Inflector` v0.11.4 — string inflection utilities
- `open` v5.1.4 — open URLs/files in OS default app
- `unicode-width` v0.1.13 — terminal-accurate string widths
- `sha2` v0.10.8 — content hashing
- `base64` v0.21.7 — encoding
- `zip` v0.6.6 — zip archive support
- `windows-sys` v0 — Win32 API bindings (Windows only)
- `dmg` v0.1.1 + `fs_extra` v1.2.0 — macOS DMG mount/copy (macOS only)
- `locale_config` — git dep, macOS locale detection (`https://github.com/lapce/locale_config.git`)
- `libc` v0.2 — C bindings
- `interprocess` v1.2.1 — IPC (named pipes / Unix domain sockets)
## Configuration
- No `.env` files — purely compile-time and runtime file-based config
- Runtime config stored in OS config directory via `directories` crate
- Settings files: `defaults/settings.toml`, `defaults/keymaps-*.toml`, `defaults/dark-theme.toml`, `defaults/light-theme.toml`
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
- Windows (MSVC, portable zip or MSI installer)
- macOS (universal binary via `lipo`, DMG via `hdiutil`)
- Linux x86_64 / aarch64 (tar.gz)
- FreeBSD / OpenBSD (partial support in update code)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- `snake_case.rs` for all source files (e.g., `color_theme.rs`, `window_tab.rs`, `rope_text_pos.rs`)
- Modules with sub-files use a directory + `mod.rs` (e.g., `lapce-app/src/panel/mod.rs`, `lapce-proxy/src/plugin/mod.rs`)
- Benchmark files live under `benches/` (e.g., `lapce-app/benches/visual_line.rs`)
- `PascalCase` for all type definitions
- Examples: `ProxyRpcHandler`, `LapceConfig`, `DiffSectionKind`, `ThemeColorPreference`
- Newtype wrappers use `PascalCase` with the wrapped type implicit: `BufferId(pub u64)`, `TermId(pub u64)`, `DapId(pub u64)`
- `snake_case` for all functions and methods
- Examples: `height_of_line`, `line_of_height`, `load_from_str`, `request_async`
- Boolean queries use `is_` or `has_` prefix: `is_empty`, `has_multiline_phantom`
- Constructor convention: `new()` for the primary constructor
- `snake_case` for all variable names and struct fields
- Examples: `variables_reference`, `color_preference`, `plugin_dev_path`
- `SCREAMING_SNAKE_CASE` for constants: `READ_BUFFER_SIZE`, `OPEN_FILE_EVENT_TOKEN`, `CANCELLATION_CHECK_INTERVAL`, `DEFAULT_CODE_GLANCE_LIST`
- `Lazy<T>` statics use `SCREAMING_SNAKE_CASE`: `DEFAULT_CONFIG`, `DEFAULT_LAPCE_CONFIG`, `DEFAULT_DARK_THEME_COLOR_CONFIG`
- All lowercase `snake_case`: `color_theme`, `icon_theme`, `rope_text_pos`
- Test modules named `tests` (rarely `test` — see `lapce-app/src/keypress/condition.rs`)
## Code Style
- Tool: `rustfmt` (stable channel)
- `max_width = 85` (configured in `.rustfmt.toml`)
- Nightly-only options (`imports_granularity`, `group_imports`) are commented out; do not use them
- Run check: `cargo fmt --all --check`
- Tool: `clippy` (run on all three platforms: Ubuntu, macOS, Windows)
- Run: `cargo clippy --profile ci`
- Suppressed lints must be annotated with a reason comment where non-obvious (see examples below)
- Common suppressions:
- Tool: `typos` (crate-ci/typos), configured in `_typos.toml`
- Inline suppression: `# spellchecker:disable-line` or `// spellchecker:disable-line`
- Block suppression: `# spellchecker:off` … `# spellchecker:on`
## Import Organization
## Error Handling
- Use `anyhow!("message")` for ad-hoc errors: `Err(anyhow!("can't save to read only file"))`
- Use `.ok_or_else(|| anyhow!("..."))` for `Option`-to-`Result` conversions
- Use `.context("...")` for adding context to propagated errors (imported from `anyhow::Context`)
- `thiserror` is declared as a workspace dependency but used sparingly
- `RpcError` in `lapce-rpc/src/lib.rs` is a plain `#[derive(Debug, Clone, Serialize, Deserialize)]` struct, not a `thiserror` type
- `.unwrap()` and `.expect("message")` are used in tests and in places where failure represents a programming error (not a recoverable runtime error)
- `.expect()` is preferred over `.unwrap()` when a message clarifies the intent
- In production code, prefer `?` propagation over `.unwrap()`; silent `.unwrap()` calls appear in some older dispatch code and are a known concern
## Logging
- `tracing::error!("{:?}", err)` — most common, for RPC and dispatch failures
- `tracing::event!(tracing::Level::ERROR, ...)` — used in `lapce-proxy/src/dispatch.rs` for structured events
- `tracing::debug!(...)` — used in keypress loader (`lapce-app/src/keypress/loader.rs`)
- `tracing::error!(...)` — imported as `use tracing::error;` in some files for brevity
## Comments
- Public API items on library crates (`lapce-rpc`, `lapce-proxy`) use `/// doc comments`
- Internal implementation details use `// inline comments`
- Module-level documentation uses `//!` inner doc comments (rare; seen in `lapce-rpc`)
- Complex algorithms and non-obvious decisions are explained inline
- Full sentences with punctuation
- Link to related types/functions using `[name]` syntax
- `TODO:` for general improvements
- `TODO(minor):` or `TODO(scope):` for scoped items
## Function Design
- Prefer `Option<T>` for values that may not exist
- Prefer `Result<T>` for fallible operations
- Use `-> ()` (implicit) for notification/fire-and-forget methods
## Derive Macros
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
- `Debug` is derived on virtually all public types
- `Clone` is derived on types that cross thread/channel boundaries
- `Serialize, Deserialize` on all RPC-transported types
- `PartialEq, Eq` on types used in comparisons or as map keys
- `Hash` added when used in `HashMap`/`HashSet`
- `Default` added explicitly when a meaningful default exists
- `Copy` only for small value types (ids, flags)
## Module Design
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| `AppData` | Root app state; windows, config, release info | `lapce-app/src/app.rs` |
| `WindowData` | Single OS window; collection of window-tabs | `lapce-app/src/window.rs` |
| `WindowTabData` | Workspace tab; owns all panel/editor state | `lapce-app/src/window_tab.rs` |
| `CommonData` | Shared workspace context (proxy, focus, commands) | `lapce-app/src/window_tab.rs` |
| `MainSplitData` | Editor split tree + `Editors` registry | `lapce-app/src/main_split.rs` |
| `EditorTabData` | Tab group holding editor tab children | `lapce-app/src/editor_tab.rs` |
| `EditorData` | Single editor instance (cursor, viewport, doc ref) | `lapce-app/src/editor.rs` |
| `Doc` | Document model (rope text, syntax, LSP data) | `lapce-app/src/doc.rs` |
| `LapceConfig` | All settings (core, UI, editor, terminal, themes) | `lapce-app/src/config.rs` |
| `Dispatcher` | Proxy-side request router | `lapce-proxy/src/dispatch.rs` |
| `PluginCatalog` | Manages running LSP/DAP/WASI plugin processes | `lapce-proxy/src/plugin/catalog.rs` |
| `PluginCatalogRpcHandler` | RPC bridge for plugin operations | `lapce-proxy/src/plugin/mod.rs` |
| `lapce-rpc` | Shared message types + stdio transport | `lapce-rpc/src/` |
| `lapce-core` | Language detection, tree-sitter syntax, rope utils | `lapce-core/src/` |
## Pattern Overview
- The UI (lapce-app) and backend (lapce-proxy) run as **separate OS processes** communicating over a JSON-RPC stdio channel defined in `lapce-rpc`.
- The UI layer uses **Floem reactive signals** (`RwSignal`, `ReadSignal`, `Memo`, `create_effect`) as the core state mechanism — no explicit message passing within the UI tree.
- A **command bus** pattern dispatches user actions: `LapceCommand` (editor ops), `LapceWorkbenchCommand` (app-level), `InternalCommand` (cross-component), `WindowCommand` (window lifecycle).
- Each `WindowTabData` holds an `Rc<CommonData>` that is passed down the entire component tree, providing access to `proxy`, `config`, `focus`, and command listeners without prop drilling.
- The proxy can be **local or remote** (`LapceWorkspaceType::Local`, `RemoteSSH`, `RemoteWSL`). Remote modes bootstrap the proxy binary over SSH/WSL and pipe stdio.
## Layers
- Purpose: Render the editor UI, handle input, own all reactive state
- Location: `lapce-app/src/`
- Contains: View functions (return `impl View`), Data structs (hold `RwSignal`s), Command handlers
- Depends on: `lapce-rpc`, `lapce-core`, `floem`, `floem-editor-core`
- Used by: `lapce` binary (`lapce-app/src/bin/lapce.rs`)
- Purpose: Defines all request/response/notification message types for the app↔proxy boundary
- Location: `lapce-rpc/src/`
- Contains: `ProxyRequest`, `ProxyNotification`, `ProxyResponse`, `CoreNotification`, `CoreRequest`, `stdio_transport`
- Depends on: serde, lsp-types, lapce-xi-rope
- Used by: both `lapce-app` and `lapce-proxy`
- Purpose: File I/O, LSP/DAP clients, WASI plugins, terminal, git — runs in a separate process
- Location: `lapce-proxy/src/`
- Contains: `Dispatcher`, `PluginCatalog`, `Buffer`, `Terminal`, `FileWatcher`
- Depends on: `lapce-rpc`, `lapce-core`, `lsp-types`, `git2`, `wasmtime`, `alacritty_terminal`
- Used by: `lapce-proxy` binary (`lapce-proxy/src/bin/lapce-proxy.rs`)
- Purpose: Shared, framework-free primitives reused by both app and proxy
- Location: `lapce-core/src/`
- Contains: `LapceLanguage` (tree-sitter config), `Syntax` (highlight engine), `Directory` (platform paths), rope utilities
- Depends on: `floem-editor-core` (re-exported for cursor/buffer/selection types), tree-sitter
- Used by: `lapce-app` and `lapce-proxy`
## Data Flow
### Primary Edit Request Path
### LSP Response Path (Completions, Diagnostics etc.)
### Remote Workspace Path
- All mutable UI state lives in `RwSignal<T>` values owned by `AppData`, `WindowData`, `WindowTabData`, or deeper data structs.
- Floem's reactive runtime propagates changes via `create_effect` and `create_memo` without manual diffing.
- `im::HashMap` (persistent/immutable hash map) is used extensively for cheap clone-on-write sharing of collections.
- `Rc<CommonData>` is cloned into each child component rather than passed through function arguments.
## Key Abstractions
- Purpose: Single source of truth for a buffer's text, syntax highlight, diagnostics, LSP data
- Examples: `lapce-app/src/doc.rs`
- Pattern: Holds a `floem_editor_core::Buffer` (rope), `RwSignal<Syntax>`, LSP result caches. Multiple `EditorData` instances can share the same `Doc` via `Rc<Doc>`.
- Purpose: Editor instance state — cursor position, viewport scroll, which `Doc` is open
- Examples: `lapce-app/src/editor.rs`
- Pattern: Holds `Rc<Doc>` and per-editor signals. Registered in `Editors(RwSignal<HashMap<EditorId, EditorData>>)` in `lapce-app/src/main_split.rs:233`.
- Purpose: Recursive split tree for the editor area; `EditorTabData` holds a tab strip of `EditorTabChild` items
- Examples: `lapce-app/src/main_split.rs`, `lapce-app/src/editor_tab.rs`
- Pattern: `SplitContent` enum holds either a nested `SplitData` or an `EditorTabData`
- Purpose: Typed command dispatch decoupled from UI events
- Examples: `lapce-app/src/command.rs`
- Pattern: `CommandKind` enum unifies `LapceWorkbenchCommand`, `EditCommand`, `FocusCommand` etc. `InternalCommand` is for cross-component side effects that don't fit the modal command flow.
- Purpose: Thread-safe handles to the bidirectional JSON-RPC channel
- Examples: `lapce-rpc/src/proxy.rs`, `lapce-rpc/src/core.rs`
- Pattern: Each handler wraps a `crossbeam_channel` sender; responses are routed by `RequestId`.
## Entry Points
- Location: `lapce-app/src/bin/lapce.rs`
- Triggers: Compiled as `lapce` executable; calls `lapce_app::app::launch()`
- Responsibilities: Parse CLI args, set up logging, optionally spawn a `--wait` child process, initialize Floem app loop
- Location: `lapce-proxy/src/bin/lapce-proxy.rs`
- Triggers: Spawned by `lapce-app` (locally) or bootstrapped on remote host (SSH/WSL)
- Responsibilities: Parse `--proxy` flag, create `Dispatcher`, wire stdio JSON-RPC transport, run plugin loop
- Location: `lapce-app/src/app.rs:3715`
- Responsibilities: Load/embed fonts, read config, construct `AppData`, open initial `WindowData` via `floem::new_window`
## Architectural Constraints
- **Threading:** Floem UI runs on the main thread (single-threaded reactive loop). Heavy work (LSP, search, git) is offloaded to the proxy process or background threads communicating via channels. `crossbeam_channel` is the preferred inter-thread mechanism.
- **Global state:** `LapceDb` (sled-based persistence) is injected via Floem's `provide_context` / `use_context` — accessible as a global from any view scope. `Directory` provides platform-specific path constants.
- **Process boundary:** The proxy process has no access to UI state. All proxy↔UI data flows through the typed RPC messages in `lapce-rpc`. This is a hard boundary.
- **Circular imports:** `lapce-core` re-exports `floem_editor_core::*` (`lapce-core/src/lib.rs:12`). Consumers that import both should prefer the `lapce-core` re-export to avoid duplicate type paths.
- **Remote proxy bootstrapping:** `lapce-app/src/proxy/remote.rs` embeds platform-specific proxy scripts (`extra/proxy.sh`, `extra/proxy.ps1`) as compile-time `include_bytes!`. These scripts download and launch the proxy binary on the remote host.
## Anti-Patterns
### Bypassing the Command Bus with Direct Signal Mutation
### Performing File I/O in the UI Process
## Error Handling
- RPC errors are returned as `RpcError { code, message }` (defined in `lapce-rpc/src/lib.rs`).
- Callback-based responses use `impl FnOnce(PluginId, Result<Resp, RpcError>)` signatures in `lapce-proxy/src/plugin/mod.rs`.
- UI side uses `create_ext_action` (Floem) to bridge `Result` from background threads back to the reactive loop.
- Panics are caught by the hook in `lapce-app/src/app/logging.rs` and written to the logs directory.
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
