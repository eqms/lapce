# Phase 3: Download Pipeline + Crash Fixes - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-07
**Phase:** 3-download-pipeline-crash-fixes
**Areas discussed:** Download-Migrations-Scope, Fehler-UI-Kanal & Severity, Malformed-Config-Verhalten

---

## Download-Migrations-Scope

### Q1 — Migrate all `get_url` callers?
| Option | Description | Selected |
|--------|-------------|----------|
| Migrate all | Replace get_url entirely; grammars.rs + proxy download_volt included; satisfies Criterion #1 (zero reqwest::blocking) | ✓ |
| Only RT-03 trio + async shim | Three app sites on DownloadPipeline; get_url internally block_on-wrapped | |

### Q2 — Where does async HTTP logic live?
| Option | Description | Selected |
|--------|-------------|----------|
| Shared core in lapce-proxy | Async core stays in lapce-proxy; DownloadPipeline in lapce-app is a thin wrapper; no duplicate client | ✓ |
| Pipeline in lapce-app, proxy own copy | New pipeline in lapce-app/src/download.rs; proxy gets its own async path | |
| You decide | Planner chooses based on crate dependency graph | |

### Q3 — How to bridge sync→async?
| Option | Description | Selected |
|--------|-------------|----------|
| Handle::current().block_on() | Use ambient Phase-2 handle; minimal-invasive, no signature changes | ✓ |
| Make call sites async | Push .await up; cleaner but larger blast radius | |
| You decide | Per-site weighting | |

**User's choice:** All three recommended options.
**Notes:** RT-03's literal "wraps the async reqwest::Client" is reconciled as wrapping the shared proxy-side core (recorded as a deliberate interpretation, not a deviation). lapce-app already depends on lapce-proxy → dependency-clean.

---

## Fehler-UI-Kanal & Severity

### Q1 — Unified error channel?
| Option | Description | Selected |
|--------|-------------|----------|
| ShowMessage everywhere | CoreNotification::ShowMessage for all 5 fixes; one test seam | ✓ |
| Mixed by context | VoltInstalling{error} for plugins, ShowMessage for git/DAP | |

### Q2 — Severity?
| Option | Description | Selected |
|--------|-------------|----------|
| Error | MessageType::ERROR for failed user-triggered operations | ✓ |
| Warning | Less intrusive; editor keeps running | |
| You decide per case | Error for active actions, Warning for degradations | |

### Q3 — CRASH-02 git no-workspace: notify or no-op?
| Option | Description | Selected |
|--------|-------------|----------|
| Still no-op | No workspace = action not applicable; graceful return, no toast | ✓ |
| Info message | Show "No folder open" | |

### Q4 (follow-up) — Criterion #4 reconciliation
| Option | Description | Selected |
|--------|-------------|----------|
| Split: 1343 silent guard + user git commands notify | Background thread no-ops; user-triggered Checkout/Discard/Init show "No folder open" via ShowMessage. Satisfies #4 literally. | ✓ |
| 1343 itself notifies | Background thread also shows message; risks spurious toasts | |
| no-op everywhere + amend ROADMAP criterion | Both silent; reword #4 via /gsd-phase | |

**User's choice:** ShowMessage everywhere; Error severity; 1343 silent no-op — reconciled via the split (Q4).
**Notes:** Thinking-partner catch — dispatch.rs:1343 is `handle_workspace_fs_event` (a background fs-event handler), NOT a user git command. A toast there would be spam. Criterion #4's "user-visible notification" is correctly satisfied at the user-triggered git arms (dispatch.rs:358–385), which co-locate with CRASH-05.

---

## Malformed-Config-Verhalten (CRASH-01)

### Q1 — Behaviour when a keymap condition fails to parse?
| Option | Description | Selected |
|--------|-------------|----------|
| Log at load time | tracing::warn! during load_from_str with token; eval-time still silent skip | ✓ |
| Silent skip, no log | Exactly as today; minimal change | |
| User notification at load | Visible prompt on invalid keymap; potentially intrusive | |

### Q2 — Eval-time semantics as test contract?
| Option | Description | Selected |
|--------|-------------|----------|
| Keep: unknown=false, !unknown=true | Lock current behaviour as contract; regression test asserts it | ✓ |
| Unify to always false | !unknown=false too; consistent but changes behaviour, regression risk | |

**User's choice:** Log at load time; keep current eval semantics as contract.
**Notes:** Honest finding surfaced — the current `check_condition` evaluator (keypress.rs:524) is already panic-free; the REQUIREMENTS line numbers (condition.rs:95,104,108) reference an older audit snapshot already refactored upstream. CRASH-01 work = regression test + load-time warning, not removing a live panic.

---

## Claude's Discretion

- Preserve get_url's existing 10s timeout + 3-retry semantics in the async successor unless planner finds reason to change.
- Module/type names for the shared async core and DownloadPipeline wrapper.
- CRASH-04 plugin error surface: ShowMessage vs. existing VoltInstalling{error} path (both reach UI).
- Test-seam mechanism for asserting "error reached UI as notification" (Regression-Test-Seam area not selected — planner chooses; assertion must verify emission, not just no-panic).
- Whether to factor async core + retry/timeout into a shared helper vs. inline.

## Deferred Ideas

- SHA256 integrity verification (SEC-01..03) — Phase 4 (extends the same call sites).
- Path-traversal / symlink-escape guards in archive extraction (SEC-04) — Phase 4.
- https_proxy scheme validation (SEC-05) — Phase 4 (extends get_url successor).
- Visible user prompt for malformed keymap conditions — considered, rejected as too intrusive; log chosen instead.
- Pushing async up call chains / concurrent download pool / bounded channels — rejected for this phase (block_on kept); REF-04 / v2.
