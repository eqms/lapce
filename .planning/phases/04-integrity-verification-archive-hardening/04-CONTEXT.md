# Phase 4: Integrity Verification + Archive Hardening - Context

**Gathered:** 2026-06-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden the three supply-chain surfaces audited in `CONCERNS.md`: integrity verification of downloaded binaries, path-traversal-safe archive extraction, and proxy-env validation. Requirements in nominal scope: SEC-01..05 + TEST-01 (regression test per security fix).

**CRITICAL REALITY (resolved ROADMAP open question — confirmed against live sources):**
Neither upstream source publishes any expected hash today:
- Plugin registry `plugins.lapce.dev/api/v1/plugins` returns **no** `sha256`/`checksum` field (fields: name/version/author/…/wasm). Download flow: API → S3 URL → S3 bytes, no hash anywhere.
- `github.com/lapce/lapce` releases publish **no** checksum asset (no `SHA256SUMS`, no `.sha256`) — assets are the raw binaries only. This covers both self-update (SEC-02) and the remote proxy binary (SEC-03), which pull from the same releases.

Therefore "verify against a published SHA256" is literally impossible for any path today without a trusted-hash source. The **HOW** (in-memory verify before any disk write, fail-closed) is locked from prior phases; the **WHERE** (source of the expected hash) was the decision made in this discussion.

**Decided scope (pragmatic per-path, no fork release infrastructure this phase):**
- **In scope (live):** SEC-03 proxy verification (stable only, via build-time pinned hash), SEC-04 tar traversal/symlink guard, SEC-05 `https_proxy` scheme validation, TEST-01 for each. Plus a reusable verify-before-write mechanism built once.
- **Built but dormant:** the generic integrity mechanism is wired so SEC-01/SEC-02 can activate later, but neither is live (no trusted hash source).
- **Out of scope → v2:** SEC-01 plugin integrity, SEC-02 self-update integrity. Documented as known gaps, not silently skipped.

**⚠️ ROADMAP reconciliation required before/at planning:** Phase 4 Success Criteria **#1 (plugin tamper rejected)** and **#2 (self-update wrong-hash fails closed)** cannot be met this phase — SEC-01/SEC-02 are deferred to v2. Criteria **#3 (proxy, stable), #4 (traversal), #5 (proxy scheme), #6 (per-fix tests)** remain in scope. Only one of the three integrity paths (proxy) goes live.

</domain>

<decisions>
## Implementation Decisions

### Integrity Mechanism (shared)
- **D-01:** Build a single, reusable **verify-before-write** primitive: download fully into memory (`resp.bytes()` — all three sites already buffer the whole body before writing), compute SHA256 with `sha2` (workspace dep since DEPS-07), compare against the expected hash, and only on match proceed to write/extract. Fail-closed is enforced by the `Result` type — a mismatch returns `Err` before any filesystem mutation. This is the locked anti-pattern fix ("verify-after-extract is fail-open").
- **D-02:** The verify primitive is reused by every wired path; SEC-01/SEC-02 call sites are structured to accept an expected hash so they can be switched live in v2 without re-architecting. **Do not** add a silent "skip verification when no hash" branch — that is fail-open and forbidden. A path is either live (hash present) or explicitly not calling the verify gate (documented deferral).

### SEC-03 — Remote Proxy Binary (LIVE)
- **D-03:** Verify the downloaded proxy `.gz` **bytes as transferred** (`remote.rs:354` `resp.bytes()`), then decompress — buffer → verify → `GzDecoder`, replacing the current stream-to-file. Verification happens locally before the binary is written, uploaded to the remote, or executed (satisfies Criterion #3 "fails before execution").
- **D-04:** Expected hash source = **build-time pinned hash table**, keyed by `(platform, architecture)` for the stable `meta::VERSION`. The maintainer records the upstream `lapce-proxy-{platform}-{arch}.gz` hashes and updates them on each version bump. Mismatch → `Err` + `tracing::error!`, no write.
- **D-05:** **Nightly = documented exception.** `proxy_version` resolves to `"nightly"` for non-stable (`remote.rs:346-349`) — a moving target with no pinnable hash. When the resolved version has no pinned entry, the proxy bootstrap proceeds **unverified** and this is recorded as a known, documented gap (consistent with SEC-02 deferral). It must NOT fail-closed (that would block all nightly remote users) and must NOT pretend to verify.
- **Storage of the pinned table = planner discretion:** a Rust `const`/`static` table vs. a checked-in manifest loaded via `include_str!`/`build.rs`. Either is acceptable; keep it framework-free and easy for the maintainer to update.

### SEC-02 — Self-Update (DEFERRED → v2)
- **D-06:** Self-update fetches a *future* "latest"/nightly release (`update.rs:23-58`); its hash is unknowable at build time and upstream publishes none. Fail-closed would block **all** updates; fail-open is forbidden. Resolution: build the mechanism, leave the self-update call site **not gated** this phase, document SEC-02 as v2. `download_release` (`update.rs:60-95`) keeps current behavior. Revisit when fork release infra (with `SHA256SUMS`) exists.

### SEC-01 — Plugin Integrity (DEFERRED → v2)
- **D-07:** The registry is third-party and not fork-controlled; it exposes no hash field. Plugin install (`download_volt`, `plugin/mod.rs:1556`) keeps current download behavior, but **SEC-04's traversal/symlink guard still applies to the plugin archive** — plugins are not left fully unprotected. SEC-01 SHA256 verification → v2 (fork-hosted manifest / TOFU were considered and rejected for this phase as too heavy/restrictive).

### SEC-04 — Archive Traversal/Symlink Guard (LIVE)
- **D-08:** Harden the **tar** extraction paths (`plugin/mod.rs:1596` zstd, `:1600` gz). The zip path (`update.rs` Windows extract) already carries the Phase-1 `zip_slip_traversal_rejected` test and the zip 2.x CVE fix — SEC-04 is the tar side.
- **D-09:** **Pre-scan, fail-closed:** before unpacking, validate every archive entry — reject `..` path components, absolute paths, and symlink/hardlink entries whose target resolves outside `plugin_dir`. If any entry is suspicious, reject the **whole archive** with no partial writes (consistent with verify-before-write). No bulk `archive.unpack` without the guard.
- **D-10:** Regression tests (TEST-01) construct a tar with (a) a `../escape` entry and (b) a symlink-escape entry, asserting extraction returns `Err` and nothing is written outside the target dir — the test asserts rejection, not merely absence of panic.

### SEC-05 — `https_proxy` Scheme Validation (LIVE)
- **D-11:** Validate at the single choke point `get_url_async` (`lapce-proxy/src/lib.rs:197-208`) before `reqwest::Proxy::all(...)`. The sync `get_url` shim and all app-side calls funnel through it (Phase-3 D-02), so one validation covers every path.
- **D-12:** Allowed schemes = **{http, https, socks5, socks5h}**. The `socks` reqwest feature is enabled (`Cargo.toml:58`), so restricting to http/https only would break legitimate socks users despite the feature being on. Anything else (ftp, file, empty scheme, …) is invalid.
- **D-13:** Invalid scheme → **hard fail**: `get_url_async` returns `Err` + `tracing::error!`. No silent fallback to a direct connection — silently bypassing a user-set proxy could leak traffic around a corporate proxy. Satisfies Criterion #5 ("invalid proxy not passed to reqwest") via the stricter, fork-appropriate interpretation.

### Claude's Discretion
- Where the verify-before-write primitive + `sha2` usage lives — `lapce-core` (framework-free, both crates depend on it) vs. alongside `get_url_async` in `lapce-proxy`. Both acceptable; keep `lapce-core` UI-framework-free if placed there.
- Exact storage form of the SEC-03 pinned-hash table (D-04 const-table vs. manifest).
- Test-seam mechanism for asserting SEC-04/SEC-05 rejection (reuse the Phase-3 `CoreRpcHandler` + `recv_timeout` seam where a UI notification is involved; pure-`Result` assertions where no UI surface applies).
- Whether SEC-03/SEC-05 failures additionally surface via `CoreNotification::ShowMessage` (proxy bootstrap is user-initiated → a notification is reasonable) vs. log-only — planner's call within the established surfacing pattern.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` §"Security Hardening" — SEC-01..05 authoritative text + audited file:line locations.
- `.planning/ROADMAP.md` §"Phase 4" — the six success criteria. **Note the reconciliation:** #1 and #2 are deferred to v2 this phase (SEC-01/SEC-02); verify against #3/#4/#5/#6 only. The ROADMAP open question about a registry hash field is **resolved here: no hash field exists.**

### Codebase Maps
- `.planning/codebase/CONCERNS.md` — original audit motivating all five SEC items (the three unverified download paths + archive extraction + proxy env).
- `.planning/codebase/CONVENTIONS.md` — error handling (`anyhow` + `?`, `.expect()` only for programmer errors), logging (`tracing::error!`). The verify/guard fixes follow these.
- `.planning/codebase/ARCHITECTURE.md` — process model (app vs proxy as separate processes; the shared HTTP core lives in `lapce-proxy`), `CoreNotification` UI-surfacing path, Floem single-thread / `create_ext_action` bridge constraint.

### Prior Phases
- `.planning/phases/03-download-pipeline-crash-fixes/03-CONTEXT.md` — D-02 (async core in `lapce-proxy`, `DownloadPipeline` thin wrapper), D-04 (`CoreNotification::ShowMessage` uniform UI channel). Phase 4 extends the exact call sites Phase 3 migrated to async; the Phase-3 fail-closed gap fix (`remote.rs:361-367` non-2xx → `Err`) is the precedent for SEC-03's fail-before-write.
- `.planning/phases/02-async-runtime-introduction/02-CONTEXT.md` — ambient tokio runtime via `rt.enter()`; `Handle::current().block_on` available at the sync call sites the verify code runs in.

No external ADRs/specs — supply-chain decisions fully captured in the decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sha2` (workspace dep, DEPS-07) — the hashing primitive; usable from `lapce-proxy` and `lapce-core`.
- `get_url_async` / `get_url` (`lapce-proxy/src/lib.rs:197,224`) — single HTTP choke point; SEC-05 validates here (the `https_proxy` read is at `:201`). 10s timeout + ≤3 retry loop preserved.
- All three download sites already buffer the full body in memory before writing (`resp.bytes()`): `download_volt` (`plugin/mod.rs:1590`), `download_release` (`update.rs:88`), proxy download (`remote.rs:358`) — verify-in-memory-before-write (D-01) is mechanically clean at each.
- `update.rs` `zip_slip_traversal_rejected` test (`update.rs:255-300`) — template/structure for the SEC-04 tar regression tests.
- Phase-3 `CoreRpcHandler::new()` + `rx().recv_timeout()` zero-mock test seam — reuse for any UI-surfaced SEC failures.

### Established Patterns
- `tar::Archive::new(decoder).unpack(&plugin_dir)` is the current extraction call for both zstd and gz (`plugin/mod.rs:1592-1601`) — SEC-04 replaces the bare `unpack` with a pre-scan-then-extract (or guarded per-entry) flow.
- Error-handling convention: `anyhow` + `?`; `.expect()` only for programmer errors. The proxy download still has `.expect("failed to create file")` / `.expect("failed to copy content")` (`remote.rs:357,360`) — out-of-scope tech debt noted in Phase 3, but SEC-03 touches this exact block, so the planner may convert them to `?` opportunistically.
- `meta::RELEASE` / `meta::VERSION` (`lapce-core::meta`) drive both the self-update target (`update.rs:24`) and the proxy version (`remote.rs:346`) — the SEC-03 pinned table keys off `meta::VERSION` for stable.

### Integration Points
- `lapce-proxy/src/lib.rs:197-212` — SEC-05 scheme validation (keystone for proxy env).
- `lapce-proxy/src/plugin/mod.rs:1590-1601` — SEC-04 tar guard.
- `lapce-app/src/proxy/remote.rs:350-367` — SEC-03 proxy verify (buffer → verify against pinned table → decompress).
- (dormant) `lapce-app/src/update.rs:60-95` (SEC-02) and `lapce-proxy/src/plugin/mod.rs:1556-1604` (SEC-01) — structured to accept an expected hash; not gated live this phase.
- Verify primitive location — new helper (`lapce-core` or `lapce-proxy`, D-01/discretion).

</code_context>

<specifics>
## Specific Ideas

- Live-source check performed during discussion (2026-06-08): plugin registry listing returns no hash field; `lapce/lapce` latest (v0.4.6) + nightly release assets contain no checksum file. This is the empirical basis for deferring SEC-01/SEC-02 — preserve it; do not re-investigate.
- "Verify the bytes as transferred" for SEC-03 means hashing the `.gz` payload, not the decompressed binary — matches what a maintainer can record from the published asset.
- SEC-04 must reject the **whole** archive on any bad entry (no partial extraction), mirroring the verify-before-write fail-closed stance.
- SEC-05 hard-fail (not silent direct fallback) is the deliberate hardening choice; the ROADMAP wording ("not passed to reqwest") is satisfied either way, but silent proxy-bypass was rejected as a leak risk.

</specifics>

<deferred>
## Deferred Ideas

- **SEC-01 plugin SHA256 verification** → **v2.** Requires a fork-controlled hash source (fork-hosted manifest or TOFU) — both rejected this phase as too heavy/restrictive. The generic verify mechanism is built so this can activate later.
- **SEC-02 self-update SHA256 verification** → **v2.** Blocked on a trusted hash published with each release (i.e., fork release infrastructure with `SHA256SUMS`). Mechanism dormant.
- **Fork-owned release infrastructure** (CI builds + `SHA256SUMS` assets, repointing self-update/proxy URLs from `lapce/lapce` to the fork) — the path that would make SEC-02 (and proper SEC-03 across all channels incl. nightly) live. Likely its own phase/milestone; out of scope here.
- **Nightly proxy verification** — no pinnable hash; revisits only with fork release infra or a per-version manifest.
- **`.expect()` cleanup in `remote.rs:357,360`** — Phase-3-noted tech debt; opportunistic only where SEC-03 already edits the block.

</deferred>

---

*Phase: 4-integrity-verification-archive-hardening*
*Context gathered: 2026-06-08*
