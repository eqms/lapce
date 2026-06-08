# Phase 4: Integrity Verification + Archive Hardening - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-08
**Phase:** 4-integrity-verification-archive-hardening
**Areas discussed:** Hash source (SEC-01/02/03), Plugin integrity (SEC-01), Archive hardening depth (SEC-04), Invalid proxy (SEC-05)

---

## Pre-discussion finding (resolved ROADMAP open question)

Live-source check on 2026-06-08:
- `plugins.lapce.dev/api/v1/plugins` listing returns no `sha256`/`checksum`/hash field.
- `github.com/lapce/lapce` latest (v0.4.6) and nightly releases publish no checksum asset (no `SHA256SUMS`/`.sha256`) — raw binaries only.

Conclusion: no trusted hash source exists upstream for any of the three download paths. This reframed the entire phase and drove every decision below.

---

## Hash source — trust strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Pragmatic per-path | Build generic verify-before-write once; wire each path to the best source available now (proxy bake-in); rest dormant | ✓ |
| Own fork release infra | Fork builds + publishes SHA256SUMS, repoint update/proxy URLs to fork | |
| Companion .sha256 convention | Expect `{url}.sha256` per download — inert/breaks since none exist upstream | |

**User's choice:** Pragmatic per-path.
**Notes:** Does not block on release infrastructure; honest about what can be verified today.

## Hash source — which fork-binary paths go live

| Option | Description | Selected |
|--------|-------------|----------|
| Only proxy live, self-update deferred | SEC-03 via build-time pinned hash; SEC-02 dormant (latest has no source; fail-closed would block all updates) | ✓ |
| Proxy + self-update via pinned manifest | Both against baked hashes; 'latest' without entry → fail-closed (brittle on each upstream release) | |
| Mechanism only, both dormant | Build reusable verify fn + tests; wire nothing live; SEC-01/02/03 all v2 | |

**User's choice:** Only proxy live, self-update deferred.
**Notes:** SEC-03 keyed off version-matched `meta::VERSION`; maintainer updates on bump. SEC-02 → v2.

## Hash source — nightly proxy handling

| Option | Description | Selected |
|--------|-------------|----------|
| Nightly = documented exception | Pinned hash only for stable; nightly (moving target) → unverified + documented gap | ✓ |
| Nightly also fail-closed | No entry → remote proxy refuses; blocks all nightly remote users | |
| Hash table per version, lookup by version+platform | Manifest mapping (version, platform-arch) → hash; missing entry behaves like option 1 | |

**User's choice:** Nightly = documented exception.
**Notes:** Must not fail-closed (would block nightly remote), must not fake verification.

## Plugin integrity (SEC-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to v2 | Mechanism dormant; install unchanged but SEC-04 guard still applies; no fail-open since verification not claimed | ✓ |
| Fork-hosted hash manifest | Fork maintains blessed-plugin hash list; ongoing maintenance + restricts ecosystem | |
| TOFU (trust-on-first-use) | Record hash on first install, warn/block on change; misses first-install compromise | |

**User's choice:** Defer to v2.
**Notes:** Registry is third-party and exposes no hash; SEC-04 still protects the plugin archive.

## Archive hardening depth (SEC-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-scan, fail-closed, incl. symlink-escape | Reject `..`, absolute paths, symlink/hardlink escape targets; whole-archive reject, no partial writes; both tar paths | ✓ |
| Forbid all symlinks in volt | Any symlink/hardlink → reject + `..`/absolute check; slightly overbroad but no symlink vector | |
| Only `..` traversal, symlinks allowed | tar-crate default + explicit `..` check; leaves symlink-escape vector open | |

**User's choice:** Pre-scan, fail-closed, incl. symlink-escape.
**Notes:** Regression test with `..` entry + symlink-escape entry; zip path already covered since Phase 1.

## Invalid proxy (SEC-05) — allowed schemes

| Option | Description | Selected |
|--------|-------------|----------|
| http/https/socks | socks feature is enabled (`Cargo.toml:58`); restricting to http/https would break socks users | ✓ |
| Strict http/https | Per requirement wording; rejects socks5 despite active socks feature | |

**User's choice:** http/https/socks5/socks5h.

## Invalid proxy (SEC-05) — behavior on invalid

| Option | Description | Selected |
|--------|-------------|----------|
| Hard fail + log | `get_url_async` returns `Err` + `tracing::error!`; no silent proxy bypass | ✓ |
| Fall back to direct + log | Build client without proxy; risk of silently bypassing user's proxy (leak) | |

**User's choice:** Hard fail + log.
**Notes:** Stricter, fork-appropriate; satisfies "invalid proxy not passed to reqwest" without silent bypass.

## Claude's Discretion

- Location of the verify-before-write primitive + `sha2` (`lapce-core` framework-free vs `lapce-proxy`).
- Storage form of the SEC-03 pinned-hash table (const table vs manifest via `include_str!`/`build.rs`).
- Test-seam mechanism for SEC-04/SEC-05 assertions.
- Whether SEC-03/SEC-05 failures additionally surface via `CoreNotification::ShowMessage` vs log-only.

## Deferred Ideas

- SEC-01 plugin verification → v2 (needs fork-controlled hash source).
- SEC-02 self-update verification → v2 (needs release-published hash / fork infra).
- Fork-owned release infrastructure (CI + SHA256SUMS, repointed URLs) — likely its own phase.
- Nightly proxy verification — revisit with fork infra or per-version manifest.
- `.expect()` cleanup in `remote.rs:357,360` — opportunistic where SEC-03 edits the block.
