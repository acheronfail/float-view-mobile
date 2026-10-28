# Tasks

## 1. Contract and rendering

- [x] 1.1 Extend frontend/mock and shared native profile validation; verify accepted/rejected action variants and profile round trips.
- [x] 1.2 Add picker, configuration and marker button with session-scoped feedback; verify frontend types and activation behavior.

## 2. Verification

- [x] 2.1 Add browser coverage for capture, failure, lifecycle and edit/input suppression; capture phone and desktop screenshots.
- [x] 2.2 Run affected frontend, shared product-store, canonical conformance and OpenSpec checks; record results.

## Verification results

- Frontend type check, changed-file lint and production build passed.
- Focused frontend tests: 53 passed. Shared product-store dashboard tests: 3 passed.
- Canonical conformance passed against the mock and shared Rust service used by Apple and Android.
- Chromium marker-widget browser coverage passed for touch/keyboard capture, duplicate suppression, non-primary input, movement/cancellation, hold/edit suppression, rejected save, pending request and session change. Phone and desktop screenshots were captured under ignored `conformance/generated/marker-widget/`.
- The existing Chromium ride-markers browser test passed gauge capture assertions but timed out in marker review waiting for `No finite samples in this range.`; that review path is unchanged by this implementation.
- WebKit execution is unavailable on this Linux host because required ICU/XML/Flite libraries are missing. No native packaging or physical-device validation was needed for this shared UI/profile addition.
- OpenSpec strict validation, Rust formatting and diff whitespace checks passed.
