# Coding Conventions

**Analysis Date:** 2026-06-07

## Naming Patterns

**Files:**
- `snake_case.rs` for all source files (e.g., `color_theme.rs`, `window_tab.rs`, `rope_text_pos.rs`)
- Modules with sub-files use a directory + `mod.rs` (e.g., `lapce-app/src/panel/mod.rs`, `lapce-proxy/src/plugin/mod.rs`)
- Benchmark files live under `benches/` (e.g., `lapce-app/benches/visual_line.rs`)

**Types (structs, enums, traits):**
- `PascalCase` for all type definitions
- Examples: `ProxyRpcHandler`, `LapceConfig`, `DiffSectionKind`, `ThemeColorPreference`
- Newtype wrappers use `PascalCase` with the wrapped type implicit: `BufferId(pub u64)`, `TermId(pub u64)`, `DapId(pub u64)`

**Functions and methods:**
- `snake_case` for all functions and methods
- Examples: `height_of_line`, `line_of_height`, `load_from_str`, `request_async`
- Boolean queries use `is_` or `has_` prefix: `is_empty`, `has_multiline_phantom`
- Constructor convention: `new()` for the primary constructor

**Variables and fields:**
- `snake_case` for all variable names and struct fields
- Examples: `variables_reference`, `color_preference`, `plugin_dev_path`

**Constants and statics:**
- `SCREAMING_SNAKE_CASE` for constants: `READ_BUFFER_SIZE`, `OPEN_FILE_EVENT_TOKEN`, `CANCELLATION_CHECK_INTERVAL`, `DEFAULT_CODE_GLANCE_LIST`
- `Lazy<T>` statics use `SCREAMING_SNAKE_CASE`: `DEFAULT_CONFIG`, `DEFAULT_LAPCE_CONFIG`, `DEFAULT_DARK_THEME_COLOR_CONFIG`

**Modules:**
- All lowercase `snake_case`: `color_theme`, `icon_theme`, `rope_text_pos`
- Test modules named `tests` (rarely `test` — see `lapce-app/src/keypress/condition.rs`)

## Code Style

**Formatting:**
- Tool: `rustfmt` (stable channel)
- `max_width = 85` (configured in `.rustfmt.toml`)
- Nightly-only options (`imports_granularity`, `group_imports`) are commented out; do not use them
- Run check: `cargo fmt --all --check`

**Linting:**
- Tool: `clippy` (run on all three platforms: Ubuntu, macOS, Windows)
- Run: `cargo clippy --profile ci`
- Suppressed lints must be annotated with a reason comment where non-obvious (see examples below)
- Common suppressions:
  - `#![allow(clippy::manual_clamp)]` — crate-level in `lapce-rpc/src/lib.rs`, `lapce-proxy/src/lib.rs`, `lapce-core/src/lib.rs`
  - `#[allow(clippy::large_enum_variant)]` — on large RPC enums in `lapce-rpc/src/proxy.rs`, `lapce-proxy/src/plugin/mod.rs`
  - `#[allow(clippy::too_many_arguments)]` — on plugin/dispatch init functions in `lapce-proxy/src/plugin/catalog.rs`, `lapce-proxy/src/plugin/lsp.rs`
  - `#[allow(clippy::type_complexity)]` — on complex channel types in `lapce-rpc/src/core.rs`
  - `#[allow(dead_code)]` — on intentionally unused plugin scaffolding in `lapce-proxy/src/plugin/mod.rs`

**Spell checking:**
- Tool: `typos` (crate-ci/typos), configured in `_typos.toml`
- Inline suppression: `# spellchecker:disable-line` or `// spellchecker:disable-line`
- Block suppression: `# spellchecker:off` … `# spellchecker:on`

## Import Organization

**Order (by convention, not yet enforced by rustfmt nightly options):**
1. `std::` imports
2. External crate imports (alphabetical within group)
3. Workspace-internal crate imports (`lapce_core`, `lapce_rpc`, `lapce_proxy`)
4. `self::` re-imports from sub-modules of the current module
5. `crate::` imports from the current crate
6. `super::` imports (used inside sub-modules)

**Grouping style:** Multi-item groups are collapsed using braced imports. Example from `lapce-app/src/editor.rs`:
```rust
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use floem::{
    ViewId,
    action::{TimerToken, exec_after, show_context_menu},
    // ...
};
use lapce_core::{
    buffer::rope_text::{RopeText, RopeTextVal},
    // ...
};
use crate::{
    command::{CommandKind, InternalCommand},
    // ...
};
```

**Path aliases:** None detected. All imports use full crate paths.

**`self::` re-exports:** Used to surface sub-module items into the current namespace without polluting the module's public API. Example from `lapce-app/src/config.rs`:
```rust
use self::{
    color::LapceColor,
    color_theme::{ColorThemeConfig, ThemeColor, ThemeColorPreference},
    // ...
};
```

## Error Handling

**Primary strategy:** `anyhow::Result<T>` for fallible functions throughout the codebase. The `?` operator is used pervasively for propagation.

**Error construction:**
- Use `anyhow!("message")` for ad-hoc errors: `Err(anyhow!("can't save to read only file"))`
- Use `.ok_or_else(|| anyhow!("..."))` for `Option`-to-`Result` conversions
- Use `.context("...")` for adding context to propagated errors (imported from `anyhow::Context`)

**Custom error types:**
- `thiserror` is declared as a workspace dependency but used sparingly
- `RpcError` in `lapce-rpc/src/lib.rs` is a plain `#[derive(Debug, Clone, Serialize, Deserialize)]` struct, not a `thiserror` type

**Boundary between subsystems:** RPC layer uses `Result<T, RpcError>` (not `anyhow::Result`) for request/response types, e.g., `Result<ProxyResponse, RpcError>` in `lapce-rpc/src/proxy.rs`.

**Unwrap policy:**
- `.unwrap()` and `.expect("message")` are used in tests and in places where failure represents a programming error (not a recoverable runtime error)
- `.expect()` is preferred over `.unwrap()` when a message clarifies the intent
- In production code, prefer `?` propagation over `.unwrap()`; silent `.unwrap()` calls appear in some older dispatch code and are a known concern

**`eprintln!` fallback:** Some error paths in `lapce-proxy/src/dispatch.rs` fall back to `eprintln!("{e:?}")` rather than using `tracing::error!`. This is inconsistent; prefer `tracing::error!` for new code.

## Logging

**Framework:** `tracing` crate (pinned to a specific git revision in workspace `Cargo.toml`)

**Macros used:**
- `tracing::error!("{:?}", err)` — most common, for RPC and dispatch failures
- `tracing::event!(tracing::Level::ERROR, ...)` — used in `lapce-proxy/src/dispatch.rs` for structured events
- `tracing::debug!(...)` — used in keypress loader (`lapce-app/src/keypress/loader.rs`)
- `tracing::error!(...)` — imported as `use tracing::error;` in some files for brevity

**Format convention:** Errors are logged with `{:?}` (Debug format). Structured fields use named arguments when additional context is provided.

**Avoid `println!`/`eprintln!`** in production code. Use `tracing` macros instead.

## Comments

**When to Comment:**
- Public API items on library crates (`lapce-rpc`, `lapce-proxy`) use `/// doc comments`
- Internal implementation details use `// inline comments`
- Module-level documentation uses `//!` inner doc comments (rare; seen in `lapce-rpc`)
- Complex algorithms and non-obvious decisions are explained inline

**Doc comment style:**
```rust
/// Wrapper around a `notify::Watcher`. It runs the inner watcher
/// in a separate thread, and communicates with it via a [crossbeam channel].
/// [crossbeam channel]: https://docs.rs/crossbeam-channel
```
- Full sentences with punctuation
- Link to related types/functions using `[name]` syntax

**`# Examples` sections:** Used for public functions where correct usage is non-obvious (e.g., `load_volt` in `lapce-proxy/src/plugin/wasi.rs`)

**TODO/FIXME format:**
```rust
// TODO: more tests with unicode characters
// TODO(minor): should this be i >= min or i + 1 >= min?
```
- `TODO:` for general improvements
- `TODO(minor):` or `TODO(scope):` for scoped items

## Function Design

**Size:** No enforced line limit, but functions exceeding ~50 lines typically warrant splitting. Large match arms (in dispatch/RPC) are acceptable given the domain.

**Parameters:** Functions with many parameters (especially plugin initialization) are annotated with `#[allow(clippy::too_many_arguments)]` rather than refactored into a builder. This is a known trade-off in the plugin subsystem.

**Return Values:**
- Prefer `Option<T>` for values that may not exist
- Prefer `Result<T>` for fallible operations
- Use `-> ()` (implicit) for notification/fire-and-forget methods

## Derive Macros

Standard pattern for data types:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyStruct { ... }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyEnum { ... }
```

- `Debug` is derived on virtually all public types
- `Clone` is derived on types that cross thread/channel boundaries
- `Serialize, Deserialize` on all RPC-transported types
- `PartialEq, Eq` on types used in comparisons or as map keys
- `Hash` added when used in `HashMap`/`HashSet`
- `Default` added explicitly when a meaningful default exists
- `Copy` only for small value types (ids, flags)

## Module Design

**Exports:** Public items are declared with `pub` at the item level. Modules are declared `pub mod` in `lib.rs` or `mod.rs`; private submodules use `mod`.

**Re-exports:** `lapce-core/src/lib.rs` re-exports `floem_editor_core::*` for backward compatibility:
```rust
pub use floem_editor_core::*;
```

**`once_cell::sync::Lazy`** is used for expensive lazy-initialized statics (e.g., `DEFAULT_CONFIG`, `DEFAULT_LAPCE_CONFIG` in `lapce-app/src/config.rs`).

---

*Convention analysis: 2026-06-07*
