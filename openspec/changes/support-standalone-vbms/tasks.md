## 1. Contract and classification

- [x] 1.1 Add required generation-scoped `deviceKind` state to the frontend contract, mock, Apple payload, and Android payload
- [x] 1.2 Extract and test one exact supported BMS identity policy shared by primary and auxiliary discovery on Apple and Android

## 2. Native BMS-only lifecycle

- [x] 2.1 Implement immediate exact direct-VBMS classification, readiness, and direct values polling on Apple without controller or session work
- [x] 2.2 Mirror the standalone BMS lifecycle and generation cancellation on Android
- [x] 2.3 Add Apple and Android tests for ready state, negative controller/session work, polling failure/recovery, disconnect, and reconnect

## 3. Shared frontend experience

- [x] 3.1 Add the full-page BMS route using the existing BMS content component with status and disconnect behavior
- [x] 3.2 Route Home/bootstrap/live/BMS states by authoritative device kind and add frontend interaction and recreation tests

## 4. Canonical coverage and cleanup

- [x] 4.1 Replace direct-VBMS gateway fixtures and documentation with the standalone direct-VBMS scenario
- [x] 4.2 Add canonical contract, request-transcript, parity, privacy, cancellation, and no-session assertions

## 5. Verification

- [x] 5.1 Run focused frontend, Apple, Android, and conformance checks
- [x] 5.2 Run `just verify` and resolve all software-gate failures
