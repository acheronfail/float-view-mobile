# Tasks

## 1. Shared schema metadata

- [x] 1.1 Carry editor scale and percentage mode through normalization, transient records and public schema; verify frozen identities, record round trips and invalid metadata with codec tests.
- [x] 1.2 Update contract consumers and canonical conformance assertions; verify direct/mock schema responses and shared native boundary checks.

## 2. Shared numeric editor

- [x] 2.1 Implement displayed-unit input, precision, steps and confirmation formatting while retaining canonical storage; verify focused component/controller tests including existing draft restoration and invalid input.
- [x] 2.2 Add browser coverage for percentage editing, blur normalization and exact outgoing values; verify frontend checks, build and focused browser test.

## 3. Integration

- [x] 3.1 Run affected Rust consumers, warnings-denied Clippy, formatting and stable Refloat direct conformance; record actual outcomes.

## Workflow follow-up

- Sync the delta, validate the canonical specification and archive after verification.

## Verification evidence

- Codec: 19 tests passed, including every frozen schema/identity, editor metadata, transient record defaults and wire round trips.
- Frontend: 76 focused controller, diff and mock tests passed; Svelte check reported no errors or warnings; build, targeted ESLint and formatting passed.
- Browser: all eight affected scenarios passed across Chromium and WebKit (editor/restoration/write feedback, motion, overlays). Fixed refreshed-value presentation while focused; updated overlay locators and simulated readiness for its open/cancel-only layout scenario.
- Rust consumers: backend service (153 tests and BLE transport), configuration service, loopback server, Apple C ABI and Android JNI checks passed. Loopback listeners were rerun outside the sandbox after sandbox-only bind failures. Two focused existing Tune storage tests passed.
- Shared contract: stable Refloat 1.3.1 direct conformance and full mock conformance passed; conformance TypeScript check passed. Hosts continue to relay the shared schema through existing bindings.
- Warnings-denied all-target Clippy passed for codec, backend service, loopback server and product store; Rust formatting and diff checks passed.
- No wire, storage, overflow or mutation-policy change. No new physical interoperability behavior is introduced.
- OpenSpec change validation passed with no issues. Canonical validation passed with four unchanged pre-existing long-requirement warnings (requirements 0, 1, 2 and 4); the added requirement has no warnings.
