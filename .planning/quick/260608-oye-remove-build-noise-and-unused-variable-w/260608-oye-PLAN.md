---
phase: 260608-oye
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - lapce-core/build.rs
  - lapce-app/src/app/logging.rs
autonomous: true
requirements:
  - remove-build-noise
must_haves:
  truths:
    - "cargo build emits no cargo::warning= lines from build.rs under normal (success) conditions"
    - "cargo build emits no 'unused variable: res' warning from logging.rs"
    - "Genuine failure warnings (Failed to obtain git repo / Failed to obtain head) are preserved"
    - "The notify-send .spawn() call and its arguments are unchanged"
  artifacts:
    - path: "lapce-core/build.rs"
      provides: "Build script without informational println! noise"
    - path: "lapce-app/src/app/logging.rs"
      provides: "Unix notification helper with suppressed unused-result binding"
  key_links:
    - from: "lapce-core/build.rs line 100"
      to: "commit variable"
      via: "commit.map(...)"
      pattern: "commit\\.map"
---

<objective>
Remove three compiler/build warnings that fire on every clean build without affecting any runtime behaviour or legitimate error diagnostics.

Purpose: Eliminate build noise so genuine warnings remain visible and actionable.
Output: Two modified files; no behaviour change.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/quick/260608-oye-remove-build-noise-and-unused-variable-w/
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove informational cargo::warning= prints from build.rs</name>
  <files>lapce-core/build.rs</files>
  <action>
    Make exactly two edits to lapce-core/build.rs:

    Edit A — Delete lines 20-21 (the comment and the Compiling-meta print):
      Remove: `    // Print info to terminal during compilation`
      Remove: `    println!("cargo::warning=Compiling meta: {release_info:?}");`
    The blank line at 22 (before `let meta_file =`) may optionally be collapsed; leave
    surrounding code untouched.

    Edit B — Delete line 99 only (the Commit-found print):
      Remove: `    println!("cargo::warning=Commit found: {commit:?}");`
    Line 100 (`    commit.map(|s| s.to_string().split_at(7).0.to_owned())`) must remain
    exactly as-is — the `commit` binding is still used there.

    DO NOT touch lines 87 or 94 (cargo::warning=Failed to obtain … — these are
    legitimate failure diagnostics, not noise).
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/lapce && touch lapce-core/build.rs && cargo build -p lapce-core 2>&1 | grep "cargo::warning=Compiling meta\|cargo::warning=Commit found" | wc -l | tr -d ' ' | grep -q '^0' && echo "PASS: no noise warnings" || echo "FAIL"</automated>
  </verify>
  <done>
    `cargo build -p lapce-core` (after touching build.rs to force re-run) produces zero
    lines matching "cargo::warning=Compiling meta" or "cargo::warning=Commit found".
    The build succeeds (exit 0).
  </done>
</task>

<task type="auto">
  <name>Task 2: Suppress unused-result binding in logging.rs</name>
  <files>lapce-app/src/app/logging.rs</files>
  <action>
    In lapce-app/src/app/logging.rs, inside the `#[cfg(unix)]` fn error_notification,
    change line 140 from:

        let res = std::process::Command::new("notify-send")

    to:

        let _ = std::process::Command::new("notify-send")

    Only the binding name changes (`res` → `_`). The `.args([...]).spawn()` chain and
    every other byte of the function remain exactly as-is.
  </action>
  <verify>
    <automated>cd /Users/picard/gitbase/lapce && cargo build -p lapce-app 2>&1 | grep "unused variable: res" | wc -l | tr -d ' ' | grep -q '^0' && echo "PASS: no unused-variable warning" || echo "FAIL"</automated>
  </verify>
  <done>
    `cargo build -p lapce-app` produces zero lines matching "unused variable: res".
    The build succeeds (exit 0).
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| build.rs → cargo output | No trust boundary crossed; deleting println! output only |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-oye-01 | Tampering | build.rs | accept | Mechanical line deletion; no logic change; diff is trivially reviewable |
</threat_model>

<verification>
Full workspace smoke check after both edits:

```
touch /Users/picard/gitbase/lapce/lapce-core/build.rs
cargo build --workspace 2>&1 | grep -E "cargo::warning=Compiling meta|cargo::warning=Commit found|unused variable: res"
```

Expected: zero matching lines. Build exits 0.
The `block v0.1.6` future-incompat warning is out of scope (transitive floem dependency) and may still appear — that is expected and acceptable.
</verification>

<success_criteria>
- lapce-core/build.rs: lines containing "Compiling meta" and "Commit found" prints are removed; failure-path warnings on lines 87 and 94 are intact.
- lapce-app/src/app/logging.rs: `let res =` replaced with `let _ =` on the notify-send line; no other changes.
- `cargo build --workspace` succeeds with none of the three target warnings.
</success_criteria>

<output>
Create `.planning/quick/260608-oye-remove-build-noise-and-unused-variable-w/260608-oye-01-SUMMARY.md` when done.
</output>
