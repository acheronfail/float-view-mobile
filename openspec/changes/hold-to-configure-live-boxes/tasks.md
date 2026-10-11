# Tasks

## 1. Shared settings and presentation

- [x] 1.1 Extend the frontend/Rust slot catalogue and defaults; verify persisted validation and canonical settings round trips.
- [x] 1.2 Implement cancellable hold configuration and authoritative lighting boxes/picker previews; verify focused gesture/control tests and document keyboard access.
- [x] 1.3 Move Live charts to the menu and remove menu lighting; update and verify browser menu, widget and current-ride scenarios.

## 2. Integration verification

- [x] 2.1 Run frontend checks/build, relevant browser suites, affected Rust/consumer checks and canonical mock/direct coverage; record results.

## Workflow follow-up

- Commit implementation, sync specs and archive after verification.

## Verification

- Frontend check and production build passed; focused lint passed.
- Gesture, live-control, value catalogue and mock-handler unit coverage passed (51 tests).
- Chromium and WebKit: lighting/menu, electrical/picker and current-ride scenarios passed (6 browser cases).
- Product-store integration suite passed (23 tests); shared service and Apple C/Android JNI consumer checks passed.
- Canonical direct and mock conformance passed, including the new persisted slot catalogue.
- Conformance TypeScript and diff whitespace checks passed.
