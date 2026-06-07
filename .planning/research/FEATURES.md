# Feature Research

**Domain:** Developer tool hardening — supply-chain integrity, error surfacing, async download UX
**Researched:** 2026-06-07
**Confidence:** HIGH (derived from CONCERNS.md audit + established patterns for Rust developer tooling)

## Feature Landscape

### Table Stakes (Users Expect These)

Features a hardened editor must have. Missing any of these leaves the stated "Core Value" unmet.

| Feature | Why Expected | Complexity | Maps to CONCERNS.md |
|---------|--------------|------------|---------------------|
| SHA256 checksum verification before unpacking any downloaded archive | HTTPS alone does not protect against a compromised CDN/mirror or a MITM that swaps the asset after TLS termination; every serious package manager (cargo, npm, pip) verifies content hashes | MEDIUM | `plugin/mod.rs:1555–1600`, `update.rs:55–85`, `proxy/remote.rs:341–360` |
| Fail-closed on checksum mismatch (refuse to unpack, alert user) | A verification that can be silently bypassed provides false assurance; fail-open defeats the entire mechanism | LOW (policy, not implementation) | Security: all three download paths |
| Path-traversal validation on archive extraction | Tar/zip path traversal (`../../`) is a well-known vector; `archive.unpack()` without validation writes outside the plugin dir | MEDIUM | `plugin/mod.rs:1592,1596` |
| User-visible error when git operations fail | Swallowing errors into `eprintln!` means the user has no feedback and may believe the operation succeeded; all mature editors (VS Code, Helix, Zed) show git errors in a notification or status bar | LOW | `dispatch.rs:358,369,377,385` |
| Graceful `Result` propagation instead of `unwrap()`/`unimplemented!()` panics | A panic on a normal user action (open workspace-less git op, launch broken DAP server, corrupt plugin) crashes the editor entirely; the crash is unrecoverable and destroys editor state | MEDIUM | `dispatch.rs:1343`, `dap.rs:104,105`, `plugin/mod.rs:1590`, `condition.rs:95,104,108` |
| Progress indication for downloads (plugin install, self-update, remote proxy) | Downloads are 5–20 MB over potentially slow connections; without progress the editor appears frozen; this is table-stakes for any UI that performs network I/O | MEDIUM | Performance: blocking HTTP in background threads |
| `https_proxy` env var scheme validation before use | Passing an unvalidated env var to `reqwest::Proxy::all()` is an injection risk; scheme check (`http://` or `https://` only) is trivial and standard | LOW | `lapce-proxy/src/lib.rs:193` |
| Regression test per crash/security fix | Without a reproducing test, fixes silently regress; the codebase has near-zero coverage so this is not optional hygiene — it is the only regression safety net | MEDIUM | Test Coverage Gaps (all sections) |

### Differentiators (Competitive Advantage)

Features beyond the minimum that meaningfully raise the security or UX bar.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Cryptographic signature verification (ed25519/minisign) on plugin archives | Moves beyond "did the bytes arrive intact" to "did the expected key sign this release"; protects against a compromised registry serving a correctly-hashed but malicious payload | HIGH | Requires registry-side key publication; no current registry support — conditional on upstream coordination |
| TUF-style trusted manifest (separate signed metadata document listing all release hashes) | Separates the transport of metadata from the transport of the artifact; standard in Rust toolchain (rustup uses TUF); prevents CDN compromise from being sufficient to deliver a malicious payload | HIGH | Requires server-side infrastructure; over-engineering for a fork unless registry adopts it |
| Cancellable downloads with partial-progress resume | Users who click "install" and change their mind can cancel; resume avoids re-downloading large binaries after a connection drop | HIGH | Valuable but not a hardening concern; defer to a UX milestone |
| Concurrent plugin installs (bounded semaphore) | Installing multiple plugins on first launch runs in parallel rather than serially; improves perceived performance | MEDIUM | CONCERNS.md "Single Plugin Download Thread"; opportunistic, not a milestone goal per PROJECT.md |
| Download speed/retry telemetry surfaced in UI | Shows retry count and reason for slow installs; helps users diagnose corporate proxy issues | HIGH | Disproportionate implementation cost for a fork; not a hardening behavior |
| Proxy binary version check before re-download | Avoids re-downloading ~10 MB on every SSH session by checking `--version` first | LOW | CONCERNS.md "Proxy Version Detection Missing"; low complexity win, though more perf than security |

### Anti-Features (Deliberately NOT Building)

Scope that seems related but would over-engineer this milestone or introduce its own risks.

| Feature | Why Requested | Why Problematic | Better Approach |
|---------|---------------|-----------------|-----------------|
| Custom PKI / in-house certificate authority for plugin signing | "Full end-to-end control" appeal | Maintaining a CA is a security responsibility in itself; key management, revocation, HSM — a fork does not have the operational capacity; a compromised CA key is worse than no signing | Use an existing identity (minisign key published on the official repo, or defer until TUF is adopted upstream) |
| Strict certificate pinning for `plugins.lapce.dev` and GitHub | Appears to add supply-chain protection beyond TLS | Certificate pinning breaks on legitimate cert rotation; causes silent failures on corporate MITM proxies; provides minimal benefit over standard TLS + hash verification | SHA256 hash verification of archive content is the right layer; trust the TLS CA store |
| Interactive download consent dialog per plugin | Seems security-conscious | Consent fatigue causes users to click through every prompt; dialog adds no information beyond what the registry already shows; VS Code and all mature editors install without per-download consent | Show the hash in the install UI as an informational detail; only block on mismatch |
| Plugin sandboxing / capability model | Legitimate long-term goal | Wasm/WASI sandboxing for plugins is a multi-month architectural change; completely out of scope for a hardening milestone that fixes existing panics and adds hash checks | Document as a future milestone; do not let it block the current scope |
| Full async rewrite of all network paths in one pass | Seems like the "right" approach for the async runtime adoption | Doing every network consumer (plugin, update, proxy) simultaneously across multiple crates in one PR increases revert risk to zero-recovery; a single broken merge leaves all downloads broken | Migrate one download path at a time; tokio adoption is one consumer per PR |
| Retry-with-backoff for all download failures | "Resilient" | Automatic retries mask registry outages and can cause repeated writes to corrupted state; a developer tool should surface the error clearly and let the user retry explicitly | Single attempt, clear error message, manual retry via UI |

## Feature Dependencies

```
[SHA256 verification — plugin]
    └──requires──> [Manifest or hash published by registry API]
                       └──requires──> [Registry-side support (plugins.lapce.dev)]

[SHA256 verification — self-update]
    └──requires──> [Hash published in GitHub release metadata or companion .sha256 file]

[SHA256 verification — remote proxy]
    └──requires──> [Hash embedded in app at build time alongside the version constant]

[Path-traversal validation]
    └──requires──> [Iterating archive entries before unpack (replaces direct archive.unpack())]

[Graceful panic → Result conversion]
    └──enables──> [User-visible error surfacing]
                      └──enhances──> [Git error display in UI]

[Async runtime (tokio)]
    └──enables──> [Progress indication with live byte counts]
    └──enables──> [Cancellable downloads]
    └──enables──> [Concurrent plugin installs] (deferred)

[Regression test per fix]
    └──requires──> [Each crash/security fix be written in test-first or test-alongside style]
```

### Dependency Notes

- **SHA256 verification requires hash source:** For plugins, the `plugins.lapce.dev` registry API must return a checksum field (or a companion `.sha256` URL must exist). For the proxy binary, the hash can be embedded at compile time next to the version constant — no external dependency. For self-updates, GitHub releases conventionally include a `SHA256SUMS` file.
- **Async runtime enables progress UX:** Without a real async runtime, streaming download progress (bytes received / total) cannot be reported back to the UI without blocking or polling hacks. The tokio migration is therefore a prerequisite for proper progress indication.
- **Panic → Result enables user-visible errors:** `eprintln!` suppression can only be fixed once the call sites return `Result` up the stack to a handler that can send an RPC notification to the UI. Converting panics to `Result` is the prerequisite step.

## MVP Definition — Hardening Milestone

### Ship in This Milestone (Hardening v1)

Minimum set to satisfy the Core Value ("every binary it downloads must be integrity-verified before execution; editor must never panic on normal user actions").

- [ ] SHA256 verification on all three download paths (plugin, self-update, remote proxy) with fail-closed rejection — addresses `plugin/mod.rs:1555–1600`, `update.rs:55–85`, `proxy/remote.rs:341–360`
- [ ] Path-traversal check on plugin archive extraction — addresses `plugin/mod.rs:1592,1596`
- [ ] `https_proxy` scheme validation — addresses `lapce-proxy/src/lib.rs:193`
- [ ] `unwrap()` → `Result` on all three known panic sites (`dispatch.rs:1343`, `dap.rs:104,105`, `plugin/mod.rs:1590`)
- [ ] `unimplemented!()` → working condition evaluation in `condition.rs:95,104,108`
- [ ] Git operation errors surfaced to user (not swallowed by `eprintln!`) — `dispatch.rs:358,369,377,385`
- [ ] Basic download progress indicator (indeterminate or byte-count) using async runtime — prerequisite also for reqwest 0.12 upgrade
- [ ] Regression test per each crash and security fix above

### Add After Validation (v1.x)

- [ ] Proxy binary version check before re-download — low complexity perf win, not a hardening blocker
- [ ] Concurrent plugin installs with bounded semaphore — improves first-launch experience

### Future Consideration (v2+)

- [ ] Cryptographic signature verification (minisign/ed25519) — requires registry-side key publication
- [ ] TUF-style trusted manifest — requires server infrastructure beyond this fork's control
- [ ] Plugin sandboxing (Wasm/WASI) — full architectural milestone

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| SHA256 verification — all three download paths | HIGH (Core Value) | MEDIUM | P1 |
| Fail-closed on mismatch + user alert | HIGH | LOW | P1 |
| Path-traversal validation on extraction | HIGH | LOW | P1 |
| Panic → Result: dispatch, dap, plugin | HIGH | MEDIUM | P1 |
| `condition.rs` unimplemented! removal | HIGH (crashes keybindings) | MEDIUM | P1 |
| Git errors surfaced to user UI | HIGH | MEDIUM | P1 |
| `https_proxy` scheme validation | MEDIUM | LOW | P1 |
| Download progress indication | MEDIUM | MEDIUM (needs async runtime) | P1 |
| Regression tests per fix | HIGH (prevents silent regression) | MEDIUM | P1 |
| Proxy version check before re-download | MEDIUM (perf, not security) | LOW | P2 |
| Concurrent plugin installs | LOW (first-launch only) | MEDIUM | P2 |
| Cryptographic signature verification | HIGH (future) | HIGH | P3 |
| TUF-style trusted manifest | HIGH (future) | HIGH | P3 |
| Plugin sandboxing | HIGH (future) | VERY HIGH | P3 |

**Priority key:**
- P1: Must have for this hardening milestone
- P2: Add after core hardening validated
- P3: Future milestone

## Competitor / Reference Behavior

How mature developer tools handle the same concerns (informing what "good" looks like):

| Behavior | VS Code | Helix / Zed | rustup | Our Target |
|----------|---------|-------------|--------|------------|
| Extension/plugin download integrity | SHA256 from marketplace API; VSIX is a signed ZIP | No plugin system / signed releases via GitHub | TUF + SHA256; fails closed hard | SHA256 from registry API; fail closed |
| Self-update integrity | Signed installer (OS-native) | GitHub release SHA256SUMS | SHA256 + ed25519 signature | SHA256 from GitHub release `SHA256SUMS` file |
| Remote binary integrity | N/A | N/A | TUF metadata | Hash embedded at compile time next to version constant |
| Git error surfacing | Notification toast + Source Control panel with error | Status bar message + notification | N/A | RPC notification to UI from proxy |
| Panic on user action | Never (JS exceptions caught globally) | Never (errors return as `Option`/`Result`) | Never | Never — every `unwrap()` at a user-triggered path replaced with `Result` |
| Download progress | Percentage + cancel button | Minimal (Zed: spinner + cancel) | Percentage bar in terminal | Byte count or indeterminate spinner; cancel not required for v1 |
| Archive path traversal | Handled by VS Code extension host sandbox | Not applicable | Handled by tar extraction library | Explicit entry path validation before `unpack()` |

## Sources

- `.planning/codebase/CONCERNS.md` — primary audit: all security, crash, and performance findings
- `.planning/PROJECT.md` — Core Value, Active requirements, Key Decisions (fail-closed mandate)
- rustup TUF implementation: https://github.com/rust-lang/rustup/blob/master/src/dist/manifest.rs (hash verification pattern)
- The Update Framework (TUF) specification: https://theupdateframework.io/overview/ (trusted manifest pattern reference)
- VS Code extension signing: https://code.visualstudio.com/api/working-with-extensions/publishing-extension#verify-a-published-extension (reference for "what good looks like")
- OWASP: Zip Slip vulnerability (path traversal in archives): https://owasp.org/www-community/vulnerabilities/Zip_Slip (rationale for entry validation)

---
*Feature research for: Lapce hardening milestone — supply-chain integrity, error surfacing, async download UX*
*Researched: 2026-06-07*
