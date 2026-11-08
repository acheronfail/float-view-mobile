# Tasks

## 1. Authoritative numeric limits

- [x] 1.1 Publish numeric encoding bounds from the codec through shared schema and mock; verify audited schemas, unchanged identities and exact integer/decimal limits with codec tests and contract conformance.
- [x] 1.2 Show inline overflow guidance and usable editor limits, including restored values; verify focused controller/component tests and no invalid write submission.

## 2. Error presentation

- [x] 2.1 Translate generic backend envelopes in the shared API client with operation-specific recovery while preserving raw envelopes and useful messages; verify client/state tests and uncertain write behavior.
- [x] 2.2 Verify browser overflow, recovery and generic write rejection in Chromium and WebKit, plus frontend build/check/lint and mock/direct conformance.

## 3. Integration

- [x] 3.1 Verify affected Rust consumers and native C/JNI boundaries, warnings-denied Clippy, formatting and conformance TypeScript checks; record results.

## Workflow follow-up

- Sync and validate the specification, then archive completed work.

## Verification evidence

- Codec: all 20 tests passed, including every audited schema identity, published encoding extrema and rejection one wire step beyond each numeric width.
- Frontend: 127 focused client, error-presentation, state, configuration, diff and mock tests passed. Coverage includes exact generic-envelope translation, preserved specific messages/raw envelopes, restored overflow, inward decimal corrections, combined XML/encoding limits and unchanged valid values.
- Browser: all four Refloat editor scenarios passed in Chromium and WebKit on the final build, including overflow feedback, disabled write/no request, canonical values, generic-state recovery, retained failure copy and timeout/unavailable uncertainty.
- Shared consumers: backend service (153 tests plus BLE transport), loopback server (14 tests), Apple C ABI and Android JNI tests passed. Stable Refloat 1.3.1 direct conformance and full mock conformance passed with canonical encoding-bound assertions. Hosts use the same shared schema projection.
- Frontend Svelte check, build, targeted ESLint/Prettier, conformance TypeScript check, warnings-denied all-target Clippy, Rust formatting and diff checks passed.
- No persistent-data, wire encoding, mutation-admission or retry policy change. Existing editor changes were committed separately as fae13e3a before this work.
- OpenSpec change validation passed with no issues. Canonical validation passed with only the four unchanged long-requirement warnings at requirements 0, 1, 2 and 4; new requirements have no warnings.
