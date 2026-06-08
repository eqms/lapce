---
status: partial
phase: 03-download-pipeline-crash-fixes
source: [03-VERIFICATION.md]
started: 2026-06-08T14:56:47Z
updated: 2026-06-08T14:56:47Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. SSH Remote Proxy Bootstrap End-to-End (fail-closed on download failure)
expected: Connect to a remote host via SSH (RemoteSSH workspace) where the proxy
binary download URL returns a non-2xx response (e.g. block/redirect the GitHub
release URL, or point at an unavailable proxy version). The bootstrap must abort
with a clear user-visible error and MUST NOT proceed to upload/launch a missing
or empty proxy binary. On a healthy network, the SSH remote proxy bootstrap still
completes normally and the editor connects. (Validates fail-closed fix at
lapce-app/src/proxy/remote.rs:361-367; success criterion #2.)
why_human: download_remote is SSH- and HTTP-bound with no mock infrastructure in
the project; the fail-closed control flow is correct by source inspection and
`?`-propagation, but the end-to-end behavioral assertion (clear UI error instead
of a confusing SSH install failure) requires a live SSH remote with controllable
network conditions.
result: [pending]

## Summary

total: 1
passed: 0
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
