# Testing Patterns

**Analysis Date:** 2026-06-07

## Test Framework

**Runner:**
- Rust's built-in `cargo test`
- No external test runner (no nextest detected)
- CI profile: `ci` (inherits `dev`, `opt-level = 0`, `debug = false`) — defined in `.cargo/config.toml`

**Assertion Library:**
- Standard library macros: `assert_eq!`, `assert_ne!`, `assert!`
- `panic!` for explicit failure with a descriptive message (preferred over bare `assert!(false, ...)`)

**Run Commands:**
```bash
cargo test                                    # Run all tests (dev profile)
cargo test --profile ci --doc --workspace    # Run doc tests (as CI does)
cargo test -p lapce-core                      # Run tests in a specific crate
cargo test --profile ci                       # Run with CI profile
```

**Benchmarks:**
- `criterion` crate — defined in `lapce-app/benches/visual_line.rs`
- Run: `cargo bench`

## Test File Organization

**Location:**
- Inline in the source file using `#[cfg(test)] mod tests { ... }` — the dominant pattern
- One exception: `lapce-proxy/src/plugin/wasi/tests.rs` is a separate file declared as `#[cfg(test)] mod tests;` at the top of `lapce-proxy/src/plugin/wasi.rs`
- No top-level `tests/` directory for integration tests

**Naming:**
- Module always named `tests` (occasionally `test` — `lapce-app/src/keypress/condition.rs`)
- Test functions prefixed `test_`: `test_snippet`, `test_lens_metric`, `test_keymap`, `test_volt_metadata_id`
- Exception: `lapce-app/src/config/icon_theme.rs` uses descriptive names without `test_` prefix: `try_all_equal_value_empty_none`, `resolve_path_to_icon_no_paths_none`

**Structure example (standard inline pattern):**
```rust
// At bottom of source file
#[cfg(test)]
mod tests {
    use super::*;          // or specific items: use super::SpecificType;
    use crate::something;  // additional imports as needed

    #[test]
    fn test_something() {
        // arrange
        // act
        // assert
    }
}
```

## Test Structure

**Suite Organization:**

Most tests follow the inline module pattern. The condition test in `lapce-app/src/keypress/condition.rs` shows the table-driven approach:

```rust
#[cfg(test)]
mod test {
    use super::Condition;
    use crate::keypress::{KeyPressData, KeyPressFocus, condition::CheckCondition};

    #[test]
    fn test_check_condition() {
        let test_cases = [
            ("editor_focus", true),
            ("list_focus", true),
            ("!editor_focus", false),
            // ...
        ];

        for (condition, should_accept) in test_cases.into_iter() {
            assert_eq!(
                should_accept,
                KeyPressData::check_condition(condition, &focus),
                "Condition check failed. Condition: {condition}. Expected result: {should_accept}",
            );
        }
    }
}
```

**Setup/teardown:**
- No `before_each` / `after_each` equivalent used
- Shared setup is extracted into private helper functions within the test module:
  ```rust
  fn get_icon_theme_config() -> IconThemeConfig {
      IconThemeConfig {
          path: "icons".to_owned().into(),
          // ...
      }
  }
  ```
- Filesystem state (for plugin tests) relies on pre-existing fixtures in `lapce-proxy/src/plugin/wasi/plugins/`

**Assertion style:**
- `assert_eq!(expected, actual)` — note: expected value comes first, actual second
- `assert_eq!(expected, actual, "failure message {details}")` — message included for table-driven tests
- `assert!` for boolean conditions
- `panic!("message")` for explicit failure on unexpected `Ok`/`Err` variants

## Mocking

**Framework:** No external mock framework (no `mockall`, no `mock_derive`)

**Manual mock pattern:** Hand-rolled mock structs that implement the relevant trait, scoped inside the test module:

```rust
// From lapce-app/src/keypress/condition.rs
#[derive(Clone, Copy, Debug)]
struct MockFocus {
    accepted_conditions: &'static [Condition],
}

impl KeyPressFocus for MockFocus {
    fn check_condition(&self, condition: Condition) -> bool {
        self.accepted_conditions.contains(&condition)
    }

    fn get_mode(&self) -> Mode {
        unimplemented!()  // Not needed for this test
    }

    fn run_command(&self, ...) -> CommandExecuted {
        unimplemented!()
    }

    fn receive_char(&self, _c: &str) {
        unimplemented!()
    }
}
```

**`unimplemented!()` in mocks:** Trait methods not exercised by the test are filled with `unimplemented!()` rather than panicking with a custom message. This is acceptable since mock panics indicate a test design error.

**What to mock:**
- UI focus/input interfaces (e.g., `KeyPressFocus`)
- Traits with heavy dependencies that are not the subject under test

**What NOT to mock:**
- File I/O in plugin tests — real fixture files are used from `lapce-proxy/src/plugin/wasi/plugins/`
- RPC/channel types — tests construct real objects

## Fixtures and Factories

**Inline construction:** Most tests construct expected values inline using struct literal syntax:
```rust
// From lapce-rpc/src/plugin.rs
let volt_metadata = VoltMetadata {
    name: "plugin".to_string(),
    version: "0.1".to_string(),
    display_name: "Plugin".to_string(),
    author: "Author".to_string(),
    description: "Useful plugin".to_string(),
    icon: None,
    repository: None,
    wasm: None,
    color_themes: None,
    icon_themes: None,
    dir: std::env::current_dir().unwrap().canonicalize().ok(),
    activation: None,
    config: None,
};
```

**Helper factory functions:** Used in `lapce-app/src/config/icon_theme.rs` to avoid repetition:
```rust
fn get_icon_theme_config() -> IconThemeConfig {
    IconThemeConfig {
        path: "icons".to_owned().into(),
        filename: [("Makefile", "makefile.svg"), ("special.rs", "special.svg")]
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .into(),
        // ...
    }
}
```

**File-based fixtures:**
- Located at `lapce-proxy/src/plugin/wasi/plugins/` — real plugin directories used by wasi tests
- Includes `smiley.png` (intentionally invalid UTF-8 for error path testing)
- Tests navigate to these using `std::env::current_dir()` relative paths

## Coverage

**Requirements:** None enforced. No coverage tooling configured in CI.

**Doc tests:** Explicitly run in CI:
```bash
cargo test --profile ci --doc --workspace
```
Doc tests are present on public functions in `lapce-proxy/src/plugin/wasi.rs` (the `load_volt` function has a `# Examples` block).

## Test Types

**Unit Tests:**
- All tests are co-located unit tests
- Scope: individual functions/methods within the module
- Examples: encoding offset conversions (`lapce-core/src/encoding.rs`), lens metrics (`lapce-core/src/lens.rs`), snippet parsing (`lapce-app/src/snippet.rs`)

**Integration Tests:**
- Effectively achieved by the wasi plugin test in `lapce-proxy/src/plugin/wasi/tests.rs` — it loads real plugin fixture directories from disk and validates full `load_volt` parsing logic
- No formal `tests/` integration test directory

**Doc Tests:**
- Present on `load_volt` in `lapce-proxy/src/plugin/wasi.rs`
- Run via `cargo test --doc --workspace`

**E2E Tests:**
- Not used

**Benchmarks:**
- `lapce-app/benches/visual_line.rs` uses `criterion` for visual line layout performance

## Platform-Conditional Tests

Tests that depend on path separators or OS behavior use `#[cfg(windows)]` / `#[cfg(unix)]` on individual test functions:

```rust
// From lapce-proxy/src/cli.rs
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
```

Both functions can have the same name because they are gated to different platforms.

## Common Patterns

**Testing `Result`-returning functions:**
```rust
// Unwrap in tests is acceptable
let (keymaps, _) = loader.finalize();

// For expected-error paths, match explicitly with panic:
match path.canonicalize() {
    Ok(path) => panic!("{path:?} file must not exist, but it is"),
    Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::NotFound),
};
```

**Testing `Option`-returning functions:**
```rust
assert_eq!(None, icon_theme_config.resolve_path_to_icon(&[]));
assert_eq!(
    Some("icons/makefile.svg".to_owned().into()),
    icon_theme_config.resolve_path_to_icon(&["Makefile"].map(AsRef::as_ref))
);
```

**Testing `Display`/`FromStr` round-trips:**
```rust
// From lapce-app/src/snippet.rs
let s = "start $1${2:second ${3:third}} $0";
let parsed = Snippet::from_str(s).unwrap();
assert_eq!(s, parsed.to_string());
```

**Testing with `Default`:**
```rust
// From lapce-app/src/debug.rs
Scope {
    variables_reference: 0,
    ..Default::default()
}
```

**Current-directory-relative fixtures:**
```rust
// From lapce-proxy/src/plugin/wasi/tests.rs
let lapce_proxy_dir = std::env::current_dir()
    .expect("Can't get \"lapce-proxy\" directory")
    .join("src")
    .join("plugin")
    .join("wasi")
    .join("plugins");
```

Note: this assumes `cargo test` is run from the `lapce-proxy/` crate directory. Tests that use `current_dir()` must be run from the correct working directory.

---

*Testing analysis: 2026-06-07*
