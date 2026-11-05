# Tasks

## 1. Shared export implementation

- [x] 1.1 Add bounded board bundle preparation/encoding with tests for board isolation, active exclusion, collisions, both formats, cancellation and resource cleanup; run product-store archive tests.
- [x] 1.2 Extend exclusive export selectors through contract, mock and shared service/HTTP routing; verify focused frontend, service and transport tests.

## 2. Board menu

- [x] 2.1 Add both Export all actions, busy/empty guards and visible result/explanation; verify frontend checks and browser interaction coverage.

## 3. Integration

- [x] 3.1 Verify mock/direct canonical board export behavior and affected Apple/Android native presentation consumers using existing integration routes.
- [x] 3.2 Run affected formatting/lint and spec validation; review diff and record results.

## Workflow follow-up

- Sync canonical specs and archive once required verification passes.

## Verification evidence

- Frontend Svelte checks: zero errors/warnings; production build passed (existing bundle-size advisory).
- Focused export client/mock tests: 40 passed; mock HTTP authentication: 4 passed outside the sandbox.
- Board export browser scenarios: Chromium and WebKit passed touch, keyboard, non-primary rejection, busy/empty guards, download and failure handling.
- Affected product-store, backend-service and loopback-server crate suites passed. Three focused bundle tests additionally verify 52 sessions, colliding timestamps, source preservation, ordinary-reader admission, count/size bounds, corruption, cancellation and cleanup.
- Focused canonical `board-export` scenario passed mock, direct, macOS and Android. Android built all four native ABIs and verified/dismissed both real scoped save pickers; these are software host checks.
- Rust formatting and warnings-denied Clippy, affected frontend ESLint/Prettier and conformance TypeScript checks passed.
- Earlier broad direct/macOS attempts stopped at readiness/location-permission timeouts before feature coverage. Focused scenarios subsequently passed. Sandbox-local mock HTTP tests initially could not bind ports and passed outside the sandbox.
- Canonical spec validation passed with 36 unchanged requirement-length warnings in backend-contract and one in saved-board-session-history; original content is preserved verbatim and new requirements have no warnings. Target adapter tests: 8 passed.
