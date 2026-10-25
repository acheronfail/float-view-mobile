# Tasks

## 1. Shared ride distance

- [x] 1.1 Accumulate and restore live ride distance in the telemetry owner; test reconnects, restored totals, missing speed, reverse movement, raw counter resets and new rides.
- [x] 1.2 Project the total to live state and native Live Activity, document contract semantics, and verify shared projection tests for both host consumers.

## 2. Client and conformance

- [x] 2.1 Align mock distance and live labels/documentation; verify frontend ride-continuity and widget tests.
- [x] 2.2 Add canonical live-distance assertions and run affected direct/mock conformance, focused Rust consumer tests, frontend checks, formatting, Clippy and strict OpenSpec validation.

## Verification results

- Shared backend and telemetry unit tests pass, including persisted nonzero distance across service replacement and native live projection equality.
- Affected application runtime, coordinator, mutation, loopback and session SQLite tests pass. Socket tests required execution outside the sandbox; two pre-existing scale tests remain ignored.
- Focused frontend ride-continuity, widget and mock tests pass; Svelte check reports no errors or warnings. Conformance TypeScript check passes.
- Direct and mock ride-continuity conformance pass. The direct fixture is stationary; nonzero/reverse travel and board-counter resets are covered by deterministic Rust tests and nonzero mock continuity coverage.
- Warnings-denied Clippy, Rust/Prettier formatting, focused ESLint, diff checks and strict OpenSpec validation pass.
