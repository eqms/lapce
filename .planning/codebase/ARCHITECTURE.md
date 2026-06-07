<!-- refreshed: 2026-06-07 -->
# Architecture

**Analysis Date:** 2026-06-07

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│                        lapce-app  (UI Process)                       │
│                                                                      │
│  AppData → WindowData → WindowTabData → MainSplitData                │
│                                   ├── EditorTabData                  │
│                                   ├── EditorData  ←──── Doc          │
│                                   ├── PanelData (Terminal, Explorer, │
│                                   │   SCM, Plugin, Search, Debug…)   │
│                                   └── PaletteData                    │
│                                                                      │
│  Reactive layer: floem RwSignal / Listener / create_effect           │
│  Command bus: InternalCommand / LapceWorkbenchCommand / WindowCommand │
└────────────────────────────┬─────────────────────────────────────────┘
                              │  stdin/stdout JSON-RPC
                              │  (ProxyRpcHandler ↔ CoreRpcHandler)
                              │  lapce-rpc crate
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      lapce-proxy  (Proxy Process)                    │
│                                                                      │
│  Dispatcher                                                          │
│  ├── Buffer management (file I/O, rope text)                         │
│  ├── FileWatcher  (notify)                                           │
│  ├── Terminal  (alacritty_terminal)                                  │
│  ├── Git  (git2)                                                     │
│  └── PluginCatalog                                                   │
│       ├── LSP client (JSON-RPC subprocess per language server)       │
│       ├── DAP client (Debug Adapter Protocol)                        │
│       └── WASI plugins (wasmtime sandbox)                            │
└──────────────────────────────────────────────────────────────────────┘
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

**Overall:** Reactive Model-View with Process Separation

**Key Characteristics:**
- The UI (lapce-app) and backend (lapce-proxy) run as **separate OS processes** communicating over a JSON-RPC stdio channel defined in `lapce-rpc`.
- The UI layer uses **Floem reactive signals** (`RwSignal`, `ReadSignal`, `Memo`, `create_effect`) as the core state mechanism — no explicit message passing within the UI tree.
- A **command bus** pattern dispatches user actions: `LapceCommand` (editor ops), `LapceWorkbenchCommand` (app-level), `InternalCommand` (cross-component), `WindowCommand` (window lifecycle).
- Each `WindowTabData` holds an `Rc<CommonData>` that is passed down the entire component tree, providing access to `proxy`, `config`, `focus`, and command listeners without prop drilling.
- The proxy can be **local or remote** (`LapceWorkspaceType::Local`, `RemoteSSH`, `RemoteWSL`). Remote modes bootstrap the proxy binary over SSH/WSL and pipe stdio.

## Layers

**UI Application Layer (`lapce-app`):**
- Purpose: Render the editor UI, handle input, own all reactive state
- Location: `lapce-app/src/`
- Contains: View functions (return `impl View`), Data structs (hold `RwSignal`s), Command handlers
- Depends on: `lapce-rpc`, `lapce-core`, `floem`, `floem-editor-core`
- Used by: `lapce` binary (`lapce-app/src/bin/lapce.rs`)

**Process Communication Layer (`lapce-rpc`):**
- Purpose: Defines all request/response/notification message types for the app↔proxy boundary
- Location: `lapce-rpc/src/`
- Contains: `ProxyRequest`, `ProxyNotification`, `ProxyResponse`, `CoreNotification`, `CoreRequest`, `stdio_transport`
- Depends on: serde, lsp-types, lapce-xi-rope
- Used by: both `lapce-app` and `lapce-proxy`

**Backend Proxy Layer (`lapce-proxy`):**
- Purpose: File I/O, LSP/DAP clients, WASI plugins, terminal, git — runs in a separate process
- Location: `lapce-proxy/src/`
- Contains: `Dispatcher`, `PluginCatalog`, `Buffer`, `Terminal`, `FileWatcher`
- Depends on: `lapce-rpc`, `lapce-core`, `lsp-types`, `git2`, `wasmtime`, `alacritty_terminal`
- Used by: `lapce-proxy` binary (`lapce-proxy/src/bin/lapce-proxy.rs`)

**Core Utilities Layer (`lapce-core`):**
- Purpose: Shared, framework-free primitives reused by both app and proxy
- Location: `lapce-core/src/`
- Contains: `LapceLanguage` (tree-sitter config), `Syntax` (highlight engine), `Directory` (platform paths), rope utilities
- Depends on: `floem-editor-core` (re-exported for cursor/buffer/selection types), tree-sitter
- Used by: `lapce-app` and `lapce-proxy`

## Data Flow

### Primary Edit Request Path

1. User keypress → floem event → `WindowTabData::key_down` (`lapce-app/src/window_tab.rs:2325`)
2. `KeyPressData` maps key sequence → `LapceCommand` (`lapce-app/src/keypress/`)
3. `WindowTabData::run_lapce_command` dispatches to `EditorData` (`lapce-app/src/window_tab.rs:692`)
4. `EditorData` mutates `Doc` via `do_edit` / `do_raw_edit` (`lapce-app/src/editor.rs:2074`)
5. `Doc::apply_deltas` updates the rope, triggers Floem reactive recalculation (`lapce-app/src/doc.rs:598`)
6. `CommonData::proxy` sends `ProxyNotification::DidChangeTextDocument` over RPC (`lapce-rpc/src/proxy.rs`)
7. `Dispatcher` on proxy side forwards to `PluginCatalog::handle_did_change_text_document` (`lapce-proxy/src/dispatch.rs`)
8. LSP client receives updated document content (`lapce-proxy/src/plugin/lsp.rs`)

### LSP Response Path (Completions, Diagnostics etc.)

1. Proxy's `PluginCatalog` receives LSP response from language server subprocess
2. `PluginCatalogRpcHandler` sends `CoreNotification` back over stdio (`lapce-proxy/src/plugin/mod.rs`)
3. `Dispatcher` forwards via `core_rpc` channel (`lapce-proxy/src/dispatch.rs`)
4. UI thread reads from `RpcMessage` channel → `WindowTabData::run_internal_command` (`lapce-app/src/window_tab.rs:1580`)
5. Reactive signals update (e.g., `CompletionData`, `DiagnosticData`) → Floem re-renders affected views

### Remote Workspace Path

1. `LapceWorkspaceType::RemoteSSH` detected during workspace init
2. `lapce-app/src/proxy/remote.rs` bootstraps proxy binary on remote host via SSH
3. stdio of the SSH subprocess is piped through `lapce-rpc::stdio_transport`
4. All subsequent proxy messages travel over the SSH pipe transparently

**State Management:**
- All mutable UI state lives in `RwSignal<T>` values owned by `AppData`, `WindowData`, `WindowTabData`, or deeper data structs.
- Floem's reactive runtime propagates changes via `create_effect` and `create_memo` without manual diffing.
- `im::HashMap` (persistent/immutable hash map) is used extensively for cheap clone-on-write sharing of collections.
- `Rc<CommonData>` is cloned into each child component rather than passed through function arguments.

## Key Abstractions

**`Doc` (Document):**
- Purpose: Single source of truth for a buffer's text, syntax highlight, diagnostics, LSP data
- Examples: `lapce-app/src/doc.rs`
- Pattern: Holds a `floem_editor_core::Buffer` (rope), `RwSignal<Syntax>`, LSP result caches. Multiple `EditorData` instances can share the same `Doc` via `Rc<Doc>`.

**`EditorData`:**
- Purpose: Editor instance state — cursor position, viewport scroll, which `Doc` is open
- Examples: `lapce-app/src/editor.rs`
- Pattern: Holds `Rc<Doc>` and per-editor signals. Registered in `Editors(RwSignal<HashMap<EditorId, EditorData>>)` in `lapce-app/src/main_split.rs:233`.

**`SplitData` / `EditorTabData`:**
- Purpose: Recursive split tree for the editor area; `EditorTabData` holds a tab strip of `EditorTabChild` items
- Examples: `lapce-app/src/main_split.rs`, `lapce-app/src/editor_tab.rs`
- Pattern: `SplitContent` enum holds either a nested `SplitData` or an `EditorTabData`

**`LapceCommand` / `InternalCommand`:**
- Purpose: Typed command dispatch decoupled from UI events
- Examples: `lapce-app/src/command.rs`
- Pattern: `CommandKind` enum unifies `LapceWorkbenchCommand`, `EditCommand`, `FocusCommand` etc. `InternalCommand` is for cross-component side effects that don't fit the modal command flow.

**`ProxyRpcHandler` / `CoreRpcHandler`:**
- Purpose: Thread-safe handles to the bidirectional JSON-RPC channel
- Examples: `lapce-rpc/src/proxy.rs`, `lapce-rpc/src/core.rs`
- Pattern: Each handler wraps a `crossbeam_channel` sender; responses are routed by `RequestId`.

## Entry Points

**UI Binary:**
- Location: `lapce-app/src/bin/lapce.rs`
- Triggers: Compiled as `lapce` executable; calls `lapce_app::app::launch()`
- Responsibilities: Parse CLI args, set up logging, optionally spawn a `--wait` child process, initialize Floem app loop

**Proxy Binary:**
- Location: `lapce-proxy/src/bin/lapce-proxy.rs`
- Triggers: Spawned by `lapce-app` (locally) or bootstrapped on remote host (SSH/WSL)
- Responsibilities: Parse `--proxy` flag, create `Dispatcher`, wire stdio JSON-RPC transport, run plugin loop

**`app::launch()` → `floem::Application`:**
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

**What happens:** Calling `some_rw_signal.set(...)` directly from a view callback instead of sending an `InternalCommand` or `LapceWorkbenchCommand`.
**Why it's wrong:** It couples view code to specific state locations, breaks the separation between data mutation and UI rendering, and makes it impossible to intercept the action for logging, undo, or testing.
**Do this instead:** Define the action as an `InternalCommand` variant in `lapce-app/src/command.rs` and handle it in `WindowTabData::run_internal_command` (`lapce-app/src/window_tab.rs:1580`).

### Performing File I/O in the UI Process

**What happens:** Using `std::fs` directly in `lapce-app` code rather than routing through the proxy.
**Why it's wrong:** The proxy is the correct location for all file operations, especially because it may be running on a remote host. Direct file access in the UI process breaks remote workspace support.
**Do this instead:** Send a `ProxyRequest` via `CommonData::proxy` (a `ProxyRpcHandler`) and handle the `CoreNotification` response in the `Dispatcher` path.

## Error Handling

**Strategy:** `anyhow::Result` for fallible operations throughout; `thiserror` for typed error kinds in `lapce-proxy`.

**Patterns:**
- RPC errors are returned as `RpcError { code, message }` (defined in `lapce-rpc/src/lib.rs`).
- Callback-based responses use `impl FnOnce(PluginId, Result<Resp, RpcError>)` signatures in `lapce-proxy/src/plugin/mod.rs`.
- UI side uses `create_ext_action` (Floem) to bridge `Result` from background threads back to the reactive loop.
- Panics are caught by the hook in `lapce-app/src/app/logging.rs` and written to the logs directory.

## Cross-Cutting Concerns

**Logging:** `tracing` crate with `tracing-subscriber` and file appender via `lapce-app/src/app/logging.rs`. Log files written to `Directory::logs_directory()`.
**Validation:** LSP types from `lsp-types` crate are used directly; no separate validation layer.
**Authentication:** No in-app auth. SSH credentials use the system SSH agent (invoked via subprocess in `lapce-app/src/proxy/ssh.rs`).

---

*Architecture analysis: 2026-06-07*
