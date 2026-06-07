# Codebase Structure

**Analysis Date:** 2026-06-07

## Directory Layout

```
lapce/                          # Workspace root (Cargo workspace)
├── Cargo.toml                  # Workspace manifest; defines members + shared deps
├── Cargo.lock                  # Lockfile (committed)
├── lapce-app/                  # UI crate — the editor frontend (Floem-based)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── bin/lapce.rs        # Binary entry point → app::launch()
│   │   ├── app.rs              # AppData, WindowData construction, app-level views
│   │   ├── window.rs           # WindowData, WindowCommonData structs
│   │   ├── window_tab.rs       # WindowTabData, CommonData — workspace-tab state hub
│   │   ├── main_split.rs       # SplitData, Editors registry, MainSplitData
│   │   ├── editor.rs           # EditorData — per-editor state and commands
│   │   ├── editor_tab.rs       # EditorTabData, EditorTabChild enum
│   │   ├── doc.rs              # Doc — document/buffer model
│   │   ├── command.rs          # All command enums (LapceCommand, InternalCommand…)
│   │   ├── config.rs           # LapceConfig + sub-configs
│   │   ├── config/             # Config sub-modules
│   │   │   ├── core.rs         # CoreConfig
│   │   │   ├── editor.rs       # EditorConfig
│   │   │   ├── ui.rs           # UIConfig
│   │   │   ├── terminal.rs     # TerminalConfig
│   │   │   ├── color_theme.rs  # ColorThemeConfig
│   │   │   ├── icon_theme.rs   # IconThemeConfig
│   │   │   ├── color.rs        # ThemeColor
│   │   │   ├── icon.rs         # LapceIcons constants
│   │   │   ├── svg.rs          # SVG store
│   │   │   └── watcher.rs      # ConfigWatcher (file-watch + reload)
│   │   ├── proxy.rs            # ProxyData
│   │   ├── proxy/
│   │   │   ├── remote.rs       # Remote trait + binary bootstrap
│   │   │   ├── ssh.rs          # SSH remote implementation
│   │   │   └── wsl.rs          # WSL remote implementation (Windows)
│   │   ├── panel/              # Panel views and data
│   │   │   ├── mod.rs
│   │   │   ├── data.rs         # PanelData, PanelSection, PanelSize
│   │   │   ├── kind.rs         # PanelKind enum
│   │   │   ├── position.rs     # PanelPosition
│   │   │   ├── view.rs         # Panel container view
│   │   │   ├── terminal_view.rs
│   │   │   ├── plugin_view.rs
│   │   │   ├── source_control_view.rs
│   │   │   ├── global_search_view.rs
│   │   │   ├── problem_view.rs
│   │   │   ├── debug_view.rs
│   │   │   ├── document_symbol.rs
│   │   │   ├── references_view.rs
│   │   │   ├── call_hierarchy_view.rs
│   │   │   ├── implementation_view.rs
│   │   │   └── style.rs
│   │   ├── editor/             # Editor sub-features
│   │   │   ├── view.rs         # Editor view rendering
│   │   │   ├── gutter.rs       # Line number / gutter view
│   │   │   ├── diff.rs         # DiffEditorData
│   │   │   └── location.rs     # EditorLocation
│   │   ├── terminal/           # Terminal emulator integration
│   │   │   ├── mod.rs
│   │   │   ├── data.rs
│   │   │   ├── event.rs
│   │   │   ├── panel.rs
│   │   │   ├── raw.rs
│   │   │   ├── tab.rs
│   │   │   └── view.rs
│   │   ├── keypress/           # Key binding system
│   │   │   ├── condition.rs
│   │   │   ├── key.rs
│   │   │   ├── keymap.rs
│   │   │   ├── loader.rs
│   │   │   └── press.rs
│   │   ├── palette/            # Command palette
│   │   │   ├── item.rs
│   │   │   └── kind.rs
│   │   ├── file_explorer/      # File tree panel
│   │   │   ├── data.rs
│   │   │   ├── mod.rs
│   │   │   ├── node.rs
│   │   │   └── view.rs
│   │   ├── app/
│   │   │   ├── grammars.rs     # Tree-sitter grammar loading
│   │   │   └── logging.rs      # Tracing setup + panic hook
│   │   ├── db.rs               # LapceDb (sled-based persistence)
│   │   ├── workspace.rs        # LapceWorkspace, LapceWorkspaceType, WorkspaceInfo
│   │   ├── plugin.rs           # PluginData (UI side)
│   │   ├── completion.rs       # CompletionData
│   │   ├── inline_completion.rs
│   │   ├── hover.rs            # HoverData
│   │   ├── palette.rs          # PaletteData
│   │   ├── source_control.rs   # SourceControlData
│   │   ├── global_search.rs    # GlobalSearchData
│   │   ├── debug.rs            # DAP debug data
│   │   ├── find.rs             # Find-in-file
│   │   ├── rename.rs           # LSP rename
│   │   ├── code_action.rs      # CodeActionData
│   │   ├── code_lens.rs        # CodeLens
│   │   ├── lsp.rs              # LSP helper utilities
│   │   ├── history.rs          # File history (VCS diff view)
│   │   ├── snippet.rs          # Snippet expansion
│   │   ├── keymap.rs           # Keymap settings view
│   │   ├── settings.rs         # Settings UI
│   │   ├── about.rs            # About dialog
│   │   ├── alert.rs            # Alert/confirmation dialogs
│   │   ├── update.rs           # Auto-updater
│   │   ├── markdown.rs         # Markdown rendering (hover docs)
│   │   ├── status.rs           # Status bar
│   │   ├── title.rs            # Title bar
│   │   ├── id.rs               # Typed ID newtypes (EditorId, EditorTabId, …)
│   │   ├── listener.rs         # Listener<T> helper (event bus cell)
│   │   ├── tracing.rs          # Tracing macros re-export
│   │   ├── wave.rs             # Decorative wave animation
│   │   ├── web_link.rs         # Clickable URLs in UI
│   │   ├── text_input.rs       # Single-line text input widget
│   │   ├── text_area.rs        # Multi-line text area widget
│   │   └── focus_text.rs       # Focus-aware text widget
│   └── benches/                # Criterion benchmarks (visual_line)
├── lapce-proxy/                # Backend proxy crate (separate process)
│   ├── Cargo.toml
│   └── src/
│       ├── bin/lapce-proxy.rs  # Proxy binary entry → lapce_proxy::mainloop()
│       ├── lib.rs              # mainloop() — stdio transport + Dispatcher wiring
│       ├── dispatch.rs         # Dispatcher — routes all ProxyRequests
│       ├── buffer.rs           # Buffer — file loading and rope management
│       ├── terminal.rs         # Terminal (alacritty_terminal integration)
│       ├── watcher.rs          # FileWatcher (notify)
│       ├── cli.rs              # CLI arg parsing + IPC open-in-existing-process
│       └── plugin/
│           ├── mod.rs          # PluginCatalogRpcHandler, PluginCatalogNotification
│           ├── catalog.rs      # PluginCatalog — manages all running plugins
│           ├── lsp.rs          # LSP subprocess client
│           ├── dap.rs          # DAP subprocess client
│           ├── psp.rs          # Plugin Server Protocol (PSP) host/handler
│           └── wasi.rs         # WASI plugin runtime (wasmtime)
├── lapce-rpc/                  # Shared RPC message types crate
│   └── src/
│       ├── lib.rs              # RpcMessage<Req,Notif,Resp>, RpcError, stdio_transport
│       ├── proxy.rs            # ProxyRequest, ProxyNotification, ProxyResponse enums
│       ├── core.rs             # CoreRpc, CoreNotification, CoreRequest enums
│       ├── stdio.rs            # stdio_transport() function
│       ├── parse.rs            # JSON-RPC frame parser
│       ├── buffer.rs           # BufferId
│       ├── file.rs             # FileNodeItem, PathObject
│       ├── file_line.rs        # FileLine (path:line:col)
│       ├── plugin.rs           # PluginId, VoltID, VoltInfo, VoltMetadata
│       ├── dap_types.rs        # DAP protocol types
│       ├── source_control.rs   # FileDiff, DiffInfo
│       ├── style.rs            # LineStyle, SemanticStyles
│       └── terminal.rs         # TermId, TerminalProfile
├── lapce-core/                 # Shared primitives crate (no Floem UI dependency)
│   └── src/
│       ├── lib.rs              # Module declarations; re-exports floem_editor_core::*
│       ├── language.rs         # LapceLanguage enum (tree-sitter config per language)
│       ├── syntax/             # Tree-sitter highlight engine
│       │   ├── mod.rs          # Syntax struct, highlight iteration
│       │   ├── highlight.rs    # HighlightConfiguration, HighlightEvent
│       │   ├── edit.rs         # SyntaxEdit — incremental tree update
│       │   └── util.rs         # RopeProvider
│       ├── directory.rs        # Directory — platform config/data/log paths
│       ├── encoding.rs         # File encoding detection
│       ├── lens.rs             # Lens (folding region heights)
│       ├── meta.rs             # ReleaseType, VERSION (generated at build)
│       ├── rope_text_pos.rs    # RopeTextPosition trait
│       └── style.rs            # line_styles helper
├── defaults/                   # Built-in configuration files (compiled in)
│   ├── settings.toml           # Default settings schema
│   ├── dark-theme.toml         # Default dark color theme
│   ├── light-theme.toml        # Default light color theme
│   ├── icon-theme.toml         # Default icon theme
│   ├── keymaps-common.toml     # Cross-platform keybindings
│   ├── keymaps-macos.toml      # macOS-specific keybindings
│   ├── keymaps-nonmacos.toml   # Linux/Windows keybindings
│   └── run.toml                # Run/debug config schema
├── icons/
│   ├── lapce/                  # Lapce SVG icons
│   └── codicons/               # VS Code Codicons
├── extra/
│   ├── fonts/DejaVu/           # Vendored DejaVu fonts (embedded at compile time)
│   ├── proxy.sh                # Unix proxy bootstrap script (embedded via include_bytes!)
│   ├── proxy.ps1               # Windows proxy bootstrap script
│   ├── linux/docker/           # Docker build contexts per distro (CI)
│   ├── macos/                  # macOS app bundle template
│   ├── windows/wix/            # Windows installer (WiX)
│   └── schemas/                # JSON schema files
├── docs/                       # Developer documentation
├── .github/
│   ├── workflows/              # GitHub Actions CI/CD
│   └── ISSUE_TEMPLATE/
├── .cargo/                     # Cargo configuration (e.g., target aliases)
├── .devcontainer/              # Dev container config
├── deny.toml                   # cargo-deny (license/security checks)
├── docker-bake.hcl             # Docker multi-platform build config
├── lapce.spec                  # RPM spec file
└── Makefile                    # Build helpers
```

## Directory Purposes

**`lapce-app/src/`:**
- Purpose: All UI logic — reactive state, view functions, command handling
- Contains: Data structs with `RwSignal` fields, view builder functions returning `impl View`, command dispatch
- Key files: `app.rs`, `window_tab.rs`, `editor.rs`, `doc.rs`, `command.rs`

**`lapce-app/src/panel/`:**
- Purpose: Side panel implementations (Terminal, File Explorer, SCM, Plugin, Search, Debug, etc.)
- Contains: One `*_view.rs` file per panel for rendering, `data.rs` for shared panel state, `kind.rs` for the `PanelKind` enum

**`lapce-app/src/editor/`:**
- Purpose: Editor-specific rendering sub-components
- Contains: `view.rs` (main editor canvas), `gutter.rs`, `diff.rs`, `location.rs`

**`lapce-app/src/proxy/`:**
- Purpose: Client-side proxy connection management
- Contains: `remote.rs` (bootstrap trait), `ssh.rs`, `wsl.rs`

**`lapce-proxy/src/plugin/`:**
- Purpose: All plugin protocol implementations
- Contains: `catalog.rs` (registry), `lsp.rs` (Language Server Protocol), `dap.rs` (Debug Adapter Protocol), `wasi.rs` (WebAssembly plugins), `psp.rs` (Plugin Server Protocol)

**`lapce-rpc/src/`:**
- Purpose: Shared message type definitions and transport
- Contains: Pure data types + the `stdio_transport` function; no business logic

**`lapce-core/src/`:**
- Purpose: Portable primitives with no direct UI dependency
- Contains: Tree-sitter syntax engine, language definitions, filesystem path helpers

**`defaults/`:**
- Purpose: Default configuration baked into the binary via `include_dir!`
- Generated: No (hand-edited TOML)
- Committed: Yes

**`extra/`:**
- Purpose: Platform packaging assets and embedded scripts
- Generated: No
- Committed: Yes

## Naming Conventions

**Files:**
- `snake_case.rs` for all Rust source files
- Files ending in `_view.rs` contain Floem view-building functions (return `impl View`)
- Files ending in `_data.rs` or named `data.rs` in modules contain reactive data structs

**Directories:**
- One directory per major feature/panel (e.g., `terminal/`, `panel/`, `editor/`, `keypress/`)
- Crate names prefixed with `lapce-` throughout the workspace

**Types:**
- Data structs: `PascalCase` with `Data` suffix for reactive state holders (e.g., `EditorData`, `WindowTabData`)
- Command enums: `PascalCase` with `Command` suffix (e.g., `LapceWorkbenchCommand`, `InternalCommand`)
- ID newtypes: defined in `lapce-app/src/id.rs` (e.g., `EditorId`, `EditorTabId`, `SplitId`)
- RPC message variants: `PascalCase` with descriptive names matching the operation (e.g., `ProxyRequest::NewBuffer`, `CoreNotification::CompletionResponse`)

## Key File Locations

**Entry Points:**
- `lapce-app/src/bin/lapce.rs`: UI binary entry point
- `lapce-proxy/src/bin/lapce-proxy.rs`: Proxy binary entry point
- `lapce-app/src/app.rs:3715` (`launch()`): App initialization

**Configuration:**
- `lapce-app/src/config.rs`: `LapceConfig` struct (aggregates all sub-configs)
- `lapce-app/src/config/watcher.rs`: Hot-reload logic
- `defaults/settings.toml`: Default settings shipped with the binary

**Core Logic:**
- `lapce-app/src/window_tab.rs`: `WindowTabData` — the central workspace state hub
- `lapce-app/src/main_split.rs`: Split tree + `Editors` registry
- `lapce-app/src/doc.rs`: `Doc` — document model (rope + syntax + LSP data)
- `lapce-proxy/src/dispatch.rs`: `Dispatcher` — proxy-side request handler
- `lapce-proxy/src/plugin/catalog.rs`: `PluginCatalog` — plugin lifecycle

**RPC Protocol:**
- `lapce-rpc/src/proxy.rs`: All proxy-bound message types
- `lapce-rpc/src/core.rs`: All core (UI)-bound message types
- `lapce-rpc/src/stdio.rs`: Transport implementation

**Testing:**
- `lapce-proxy/src/plugin/wasi/tests.rs`: WASI plugin unit tests
- `lapce-app/benches/visual_line.rs`: Criterion benchmark for visual line calculation

## Where to Add New Code

**New panel:**
- Panel data struct: `lapce-app/src/panel/data.rs` (add to `PanelData`) or a new `lapce-app/src/panel/my_panel.rs`
- Panel kind: add variant to `PanelKind` in `lapce-app/src/panel/kind.rs`
- Panel view: `lapce-app/src/panel/my_panel_view.rs` — function returning `impl View`
- Wire into workbench: `lapce-app/src/app.rs` (`workbench` function)

**New editor feature (UI-side):**
- Data: extend `EditorData` in `lapce-app/src/editor.rs` or `Doc` in `lapce-app/src/doc.rs`
- Command: add variant to `InternalCommand` or `LapceWorkbenchCommand` in `lapce-app/src/command.rs`
- Handler: `WindowTabData::run_internal_command` in `lapce-app/src/window_tab.rs:1580`

**New proxy capability (file I/O, LSP, etc.):**
- Message types: add to `ProxyRequest` / `ProxyResponse` in `lapce-rpc/src/proxy.rs` and matching `CoreNotification` in `lapce-rpc/src/core.rs`
- Proxy handler: `Dispatcher::handle_request` in `lapce-proxy/src/dispatch.rs`
- UI caller: send via `CommonData::proxy` (`ProxyRpcHandler`) in the relevant data struct
- UI receiver: handle in `WindowTabData::run_internal_command`

**New config option:**
- Add field to appropriate sub-config in `lapce-app/src/config/` (e.g., `editor.rs`, `ui.rs`)
- Add default value to `defaults/settings.toml`

**New language support:**
- Add `LapceLanguage` variant in `lapce-core/src/language.rs`
- Add tree-sitter grammar query files in the grammars location loaded by `lapce-app/src/app/grammars.rs`

**New command:**
- If workbench-level: add variant to `LapceWorkbenchCommand` in `lapce-app/src/command.rs`
- If editor-level: add variant to `EditCommand` or `FocusCommand` in `lapce-core` (re-exported via `floem_editor_core`)
- Add default keybinding to `defaults/keymaps-common.toml` (or platform-specific file)

## Special Directories

**`.cargo/`:**
- Purpose: Cargo config (build target aliases, registry settings)
- Generated: No
- Committed: Yes

**`extra/linux/docker/`:**
- Purpose: Per-distro Dockerfiles for CI cross-compilation
- Generated: No
- Committed: Yes

**`extra/fonts/`:**
- Purpose: Vendored DejaVu fonts embedded at compile time via `include_bytes!`
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-06-07*
