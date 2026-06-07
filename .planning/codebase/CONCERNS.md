# Codebase Concerns

**Analysis Date:** 2026-06-07

## Tech Debt

**Unimplemented KeyPress Conditions:**
- Issue: Three `unimplemented!()` macro calls in `Or`, `And`, and `Not` operators of `CheckCondition`
- Files: `lapce-app/src/keypress/condition.rs` lines 95, 104, 108
- Impact: Any keybinding that uses combined conditions (AND/OR/NOT) will panic at runtime
- Fix approach: Implement the recursive condition evaluation logic for compound expressions

**Terminal Default Profile Config Hack:**
- Issue: Explicit comment marks this as a hack — terminal default profile requires special-cased `parent`/`key` rewriting before saving to TOML
- Files: `lapce-app/src/config.rs` line 987–990
- Impact: Makes config serialization brittle; breaks consistency with all other config fields
- Fix approach: Store `terminal.default-profile` as a flat OS-suffixed key at the data model level, or redesign the profile schema

**Palette Next/Previous Page Not Implemented:**
- Issue: `next_page()` and `previous_page()` methods are stubs with `// TODO: implement`
- Files: `lapce-app/src/palette.rs` lines 1514, 1518
- Impact: Palette cannot page through long result lists; UX degradation on large workspaces
- Fix approach: Implement pagination using the existing `index` / `items` signals with page-size calculation

**Logging Panel Command Stub:**
- Issue: `InternalCommand::UpdateLogLevel` is received but only logs a debug trace — no actual log level change or panel
- Files: `lapce-app/src/window_tab.rs` line 1587
- Impact: Runtime log level change UI is non-functional even if surfaced
- Fix approach: Wire `tracing::subscriber` dynamic filter or implement a `LogLevel` control on the tracing subscriber

**Dead Code in Plugin RPC Handler:**
- Issue: `id`, `pending`, and `handle_response` on `PluginCatalogRpcHandler` are all marked `#[allow(dead_code)]` — infrastructure for request/response RPC is built but unused
- Files: `lapce-proxy/src/plugin/mod.rs` lines 224, 226, 243
- Impact: Code complexity, maintenance burden, and potential confusion about how plugin RPC requests are supposed to work
- Fix approach: Either complete the request-response plugin RPC path, or remove the unused infrastructure

**Proxy Version Detection Missing:**
- Issue: Remote proxy binary is unconditionally deleted and re-downloaded on every connection — there is no version check
- Files: `lapce-app/src/proxy/remote.rs` line 341
- Impact: Every SSH/remote session re-downloads a ~10 MB binary, causing slow remote workspace startup
- Fix approach: Compare the proxy binary version (e.g., via a `--version` flag) before re-downloading

**Glob Recompiled on Every Directory Read:**
- Issue: `Glob::new(...)` and `glob.compile_matcher()` are called inside the directory listing loop
- Files: `lapce-app/src/file_explorer/data.rs` line 207
- Impact: File explorer performance degrades when browsing large directories; glob compilation is not cheap
- Fix approach: Cache the compiled `GlobMatcher` in config state, invalidated only when `files_exclude` changes

**Granular Invalidation Missing in Doc:**
- Issue: Three separate `// TODO: more granular invalidation` comments — `clear_text_cache()` is called for completion lens, inline completion, and diagnostics changes that may only affect a single line
- Files: `lapce-app/src/doc.rs` lines 1139, 1146, 1430
- Impact: Full text layout cache busted on every single completion or diagnostic update, causing unnecessary re-renders across the entire document
- Fix approach: Invalidate only affected line ranges instead of the entire cache

**Font Family Parsed on Every Line Render:**
- Issue: `FamilyOwned::parse_list(...)` is called uncached for every line in `font_families()`
- Files: `lapce-app/src/doc.rs` line 1951 (comment: `// TODO: cache this`)
- Impact: String parsing on every render call; significant CPU waste in large files
- Fix approach: Cache parsed font families in a `Signal` that invalidates only when `config.editor.font_family` changes

**Diff View Wrapping Disabled and vline Count Approximated:**
- Issue: Multiple TODOs in `screen_lines` for diff view: wrapped lines are disabled, vline counts are guessed rather than computed
- Files: `lapce-app/src/editor.rs` lines 3622, 3668, 3688, 3707, 3745
- Impact: Diff view cannot enable word wrap; scroll position may drift in large diffs
- Fix approach: Implement proper dual-editor vline synchronization with actual vline counting

**Completion `is_incomplete` Field Ignored:**
- Issue: `CompletionResponse::List.is_incomplete` is silently discarded — the LSP protocol uses this to tell the client to re-request completions as more text is typed
- Files: `lapce-app/src/completion.rs` line 106
- Impact: For LSPs that return incomplete lists (e.g., rust-analyzer on large crates), completion results may be stale and miss valid candidates
- Fix approach: Track `is_incomplete`, and trigger a new completion request on next keystroke when set

**Inline Completion and Completion Lens Coexist Without Coordination:**
- Issue: Both inline completion and completion lens can be displayed simultaneously with no deduplication logic
- Files: `lapce-app/src/doc.rs` line 1811
- Impact: Visual clutter; two overlapping ghost-text suggestions for the same cursor position
- Fix approach: Add a priority rule (e.g., inline completion wins) and suppress completion lens when inline completion is active

## Known Bugs

**`unwrap()` on Optional Workspace in Dispatch:**
- Symptoms: Panics when certain git commands (checkout, discard) are triggered with no workspace open
- Files: `lapce-proxy/src/dispatch.rs` line 1343 (`.unwrap()` on `self.workspace`)
- Trigger: Running git operations when no folder is open
- Workaround: None; workspace existence must be verified before calling dispatch

**`unwrap()` on Plugin Process stdin/stdout:**
- Symptoms: Panics if the DAP server process fails to capture stdio
- Files: `lapce-proxy/src/plugin/dap.rs` lines 104, 105
- Trigger: DAP server launch failure where stdin/stdout are not captured
- Workaround: None; process spawn errors propagate as panics

**`unwrap()` on `zstd::Decoder::new` in Plugin Download:**
- Symptoms: Panics if the zstd stream is malformed during plugin installation
- Files: `lapce-proxy/src/plugin/mod.rs` line 1590
- Trigger: Corrupt or truncated zstd plugin archive
- Workaround: None; propagate as `Result` instead

**`eprintln!` Used for Error Reporting in Dispatch:**
- Symptoms: Git operation errors (checkout, discard, init) are silently swallowed — only printed to stderr, never surfaced to the user
- Files: `lapce-proxy/src/dispatch.rs` lines 358, 369, 377, 385
- Trigger: Any git operation that fails (e.g., dirty tree on checkout)
- Workaround: Check the terminal output; no UI feedback is given

## Security Considerations

**No Integrity Verification for Plugin Downloads:**
- Risk: Plugin archives are downloaded from S3 via a redirect from `plugins.lapce.dev` with no checksum or signature verification
- Files: `lapce-proxy/src/plugin/mod.rs` lines 1555–1600 (`download_volt`)
- Current mitigation: HTTPS transport only
- Recommendations: Add SHA256 checksums returned by the plugin registry API and verify before unpacking; consider code-signing

**No Integrity Verification for App Self-Updates:**
- Risk: Update binaries are downloaded from GitHub releases without hash or signature verification
- Files: `lapce-app/src/update.rs` lines 55–85 (`download_release`)
- Current mitigation: HTTPS transport only
- Recommendations: Verify SHA256 of downloaded archive against a published hash; reject and alert on mismatch

**No Integrity Verification for Remote Proxy Binary:**
- Risk: The proxy binary downloaded from GitHub during remote SSH sessions is unpacked directly without checksum verification
- Files: `lapce-app/src/proxy/remote.rs` lines 341–360
- Current mitigation: HTTPS transport to GitHub; binary is re-downloaded on every session
- Recommendations: Pin expected hash alongside the proxy version; fail closed if hash does not match

**Plugin Archive Path Traversal Risk:**
- Risk: Plugin archives are unpacked with `archive.unpack(&plugin_dir)` — if a malicious archive contains paths like `../../`, it could write outside the plugin directory
- Files: `lapce-proxy/src/plugin/mod.rs` lines 1592, 1596
- Current mitigation: Plugins are sourced only from `plugins.lapce.dev` registry (no local install)
- Recommendations: Validate each archive entry path before extraction; use a safe-extraction crate

**Proxy `https_proxy` Environment Variable Injection:**
- Risk: `std::env::var("https_proxy")` is read without sanitization and passed directly to `reqwest::Proxy::all()`
- Files: `lapce-proxy/src/lib.rs` line 193
- Current mitigation: Proxy URL parsing will fail on malformed values
- Recommendations: Validate the proxy URL scheme is `http` or `https` before using

## Performance Bottlenecks

**Excessive `.clone()` Calls in Hot Paths:**
- Problem: 1,112 `.clone()` calls across `lapce-app/src/` alone; `doc.rs`, `editor.rs`, and `window_tab.rs` account for 180+ clones between them
- Files: `lapce-app/src/doc.rs`, `lapce-app/src/editor.rs`, `lapce-app/src/window_tab.rs`
- Cause: Reactive signal architecture requires owned data in closures; many cases could use `Arc` or structural sharing
- Improvement path: Profile with `cargo-flamegraph` to identify which clones are in render hot paths; replace with `Arc` sharing or `im::Vector` structural sharing where possible

**Blocking HTTP Calls in Background Threads (reqwest::blocking):**
- Problem: All network I/O (plugin download, update download, proxy download) uses `reqwest::blocking` synchronously inside `std::thread::spawn`
- Files: `lapce-proxy/src/lib.rs` (`get_url`), `lapce-app/src/plugin.rs`, `lapce-app/src/update.rs`, `lapce-app/src/proxy/remote.rs`
- Cause: No async runtime; blocking calls monopolize OS threads for the entire download duration
- Improvement path: Adopt `tokio` or `smol` for network I/O, or at minimum use a dedicated bounded thread pool for downloads

**Large Enum Variants in Plugin and DAP Message Types:**
- Problem: Multiple `#[allow(clippy::large_enum_variant)]` suppressions indicate oversized enum variants being copied on every match arm
- Files: `lapce-proxy/src/plugin/psp.rs` (lines 108, 117), `lapce-proxy/src/plugin/mod.rs` (lines 90, 155), `lapce-proxy/src/plugin/dap.rs` (line 416), `lapce-app/src/debug.rs` (line 131)
- Cause: Message enums include large variant payloads inline instead of boxed
- Improvement path: Box large variants (`Box<LargeVariant>`) or restructure message types

## Fragile Areas

**`app.rs` — 4,321 Lines:**
- Files: `lapce-app/src/app.rs`
- Why fragile: Monolithic file containing application setup, window management, IPC socket listener, update logic, and UI construction; changes to any area risk unintended side-effects
- Safe modification: Add comprehensive tests before refactoring; extract IPC, update, and window-management into separate modules
- Test coverage: No unit tests; only integration-level behavior observable at runtime

**`editor.rs` — 3,926 Lines:**
- Files: `lapce-app/src/editor.rs`
- Why fragile: Central editor data model mixes command dispatch (800+ lines), screen-line computation for both normal and diff view, snippet management, and LSP integration
- Safe modification: Isolate screen-line computation (`screen_lines`) into its own module before touching rendering logic
- Test coverage: No unit tests

**`window_tab.rs` — 2,989 Lines:**
- Files: `lapce-app/src/window_tab.rs`
- Why fragile: Aggregates all cross-cutting concerns: config reload, proxy connection, terminal, panel state, hover, completion, inlay hints, find, source control
- Safe modification: Each sub-system should be extracted to its own struct with clear ownership boundaries before modification
- Test coverage: No unit tests

**Watcher Path Re-registration Heuristic:**
- Files: `lapce-proxy/src/watcher.rs` lines 174–185
- Why fragile: Uses a coarse heuristic to decide whether child paths need re-watching after a parent directory change; the underlying `notify` crate's behavior is platform-dependent
- Safe modification: Only modify after the pending `notify` crate rewrite (noted in the TODO) lands with proper prefix-tree tracking
- Test coverage: No unit tests for watcher re-registration logic

**Config TOML Round-Trip:**
- Files: `lapce-app/src/config.rs`
- Why fragile: Config reading uses `config` crate + `toml_edit` for writing, with special-case branching for terminal profile. Multiple TODOs indicate the config system is under-specified
- Safe modification: Add serialization round-trip tests before adding new config fields
- Test coverage: Partial — `color_theme.rs` and `icon_theme.rs` have unit tests; core config serialization has none

## Scaling Limits

**Single Plugin Download Thread:**
- Current capacity: Plugins are installed one at a time in a single `std::thread::spawn`
- Limit: Installing many plugins on first launch causes sequential blocking HTTP downloads
- Scaling path: Use a semaphore-limited thread pool or async download pipeline for concurrent plugin installation

**Unbounded Crossbeam Channels:**
- Current capacity: `crossbeam_channel::unbounded()` used for plugin catalog RPC (`lapce-proxy/src/plugin/mod.rs`)
- Limit: High-frequency events (file watcher, LSP notifications) can grow the channel unboundedly
- Scaling path: Switch to bounded channels with backpressure for high-frequency paths

## Dependencies at Risk

**`reqwest` Pinned to 0.11 (EOL):**
- Risk: `reqwest` 0.12 is the current stable release; 0.11 uses `hyper` 0.14 which is no longer maintained
- Impact: No security patches; breaks compilation with newer `tokio` ecosystem crates
- Migration plan: Upgrade to `reqwest = "0.12"` (API-compatible for most usages); requires updating `hyper`-dependent code

**`interprocess` Pinned to 1.2.1 (Outdated):**
- Risk: Current release is 2.x with significant API improvements and bug fixes for local socket handling; 1.2.1 has known issues on some Linux kernel versions
- Impact: Single-instance detection and IPC may behave incorrectly on modern kernels
- Migration plan: Upgrade to `interprocess = "2"` and update socket API call sites in `lapce-app/src/app.rs`

**`floem`, `tracing`, `alacritty_terminal`, `psp-types` Pinned to Git Commits:**
- Risk: All four critical dependencies are pinned to specific git commit SHAs rather than versioned releases; any breaking change in those repos is invisible until the SHA is bumped
- Impact: Reproducible builds are fragile; security patches in upstream repos require manual SHA updates
- Files: `Cargo.toml` lines 81–88 (floem), 105–125 (tracing), 122–123 (alacritty_terminal), 71 (psp-types)
- Migration plan: Work with upstream maintainers to cut versioned releases; pin to tags rather than arbitrary SHAs

**`lsp-types` Patched via `patch.crates-io`:**
- Risk: `lsp-types` is patched with a fork to add message-type debug; this fork may lag behind upstream LSP spec updates
- Files: `Cargo.toml` line 93 (patch section), dependency at line 69
- Impact: Missing LSP protocol features or incompatibility as LSP spec evolves
- Migration plan: Upstream the debug enhancement as a PR to `lsp-types`; remove the patch once merged

**`toml` Pinned to Wildcard Version `"*"`:**
- Risk: Any major version of `toml` is accepted; a breaking major version bump will silently select incompatible API
- Files: `Cargo.toml` line 65
- Impact: Unpredictable build behavior after `cargo update`
- Migration plan: Pin to a specific major version: `toml = "0.8"`

## Missing Critical Features

**No Logging Panel:**
- Problem: `InternalCommand::UpdateLogLevel` is received but there is no UI panel to view or filter log output
- Blocks: Developer debugging, support for users reporting issues without CLI access

**No WrapColumn Wrap Style:**
- Problem: `WrapStyle::WrapColumn` is commented out in config UI with `// TODO`
- Files: `lapce-app/src/config.rs` line 171
- Blocks: Users who want column-width-based wrapping instead of viewport wrapping

**No Markdown InlineHtml / InlineMath / DisplayMath Rendering:**
- Problem: Three Markdown event types are silently swallowed with `// TODO(panekj): Implement`
- Files: `lapce-app/src/markdown.rs` lines 187–189
- Blocks: Correct rendering of hover docs from LSPs that emit math or HTML snippets (e.g., Lean, Haskell)

## Test Coverage Gaps

**Editor Core Logic — Untested:**
- What's not tested: Command dispatch, screen-line computation, snippet application, diff-view rendering
- Files: `lapce-app/src/editor.rs` (3,926 lines), `lapce-app/src/editor/view.rs` (2,667 lines)
- Risk: Regressions in cursor movement, selection, or rendering go undetected
- Priority: High

**Document / Buffer Operations — Untested:**
- What's not tested: `apply_delta`, completion lens invalidation, diagnostic rendering, inline completion priority
- Files: `lapce-app/src/doc.rs` (2,239 lines)
- Risk: Silent breakage in text mutation or display
- Priority: High

**Window Tab / Application Orchestration — Untested:**
- What's not tested: Config reload, proxy connection lifecycle, panel state transitions
- Files: `lapce-app/src/window_tab.rs` (2,989 lines), `lapce-app/src/app.rs` (4,321 lines)
- Risk: Cross-cutting regressions when adding new panel or command types
- Priority: Medium

**Plugin Download and Installation — Untested:**
- What's not tested: `download_volt`, `install_volt`, archive extraction, error paths
- Files: `lapce-proxy/src/plugin/mod.rs` (1,797 lines)
- Risk: Broken plugin install silently fails with only an `eprintln!` as evidence
- Priority: Medium

**File Watcher Re-registration Heuristic — Untested:**
- What's not tested: Watcher behavior when parent directories are deleted/recreated
- Files: `lapce-proxy/src/watcher.rs`
- Risk: File changes not detected after directory rename on Linux/macOS
- Priority: Medium

---

*Concerns audit: 2026-06-07*
