# Tasks

## 1. Playback completion policy

- [x] 1.1 Remove threshold-only cancellation in shared announcement policy and add regression tests for speed, load and combined phrases finishing, no below-threshold repeats, cooldown and renewed eligibility; verify focused announcement tests.
- [x] 1.2 Verify cancellation remains bounded for stale context, disconnect, rule edits and timeout with focused regression tests.

## 2. Integration checks

- [x] 2.1 Run backend-service and loopback-server tests, relevant Rust formatting and warnings-denied Clippy, strict OpenSpec validation and diff whitespace checks; record outcomes.

## Verification outcomes

- Eight focused announcement tests passed; the new tests reproduced threshold cancellation before the fix.
- Backend-service: 105 unit tests and two BLE transport tests passed.
- Loopback-server: 14 unit/integration tests passed with local socket access outside the sandbox; sandbox runs could not bind sockets.
- Direct conformance passed outside the sandbox.
- Warnings-denied Clippy for both crates and all targets, Rust formatting, planning-document formatting, strict OpenSpec validation and `git diff --check` passed.
