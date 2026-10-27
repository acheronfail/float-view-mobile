# Tasks

## 1. Shared backend

- [x] 1.1 Add atomic checkpoint extension with restart, stale/expired/active, shorter-window and storage-failure tests; run product-store tests.
- [x] 1.2 Add shared command, loopback dispatch and diagnostics vocabulary; test timer replacement, stale callbacks, expiry and absence of Bluetooth work in service tests.

## 2. Frontend and mock

- [x] 2.1 Add typed client/mock command and timer replacement; run mock ride-continuity unit tests for repeated extension and boundary rejection.
- [x] 2.2 Add shared Give me more time control with pending/error handling; verify primary, non-primary and keyboard activation in browser coverage on both paused-card routes.

## 3. Integration verification

- [x] 3.1 Extend canonical ride-continuity scenarios; run focused mock and direct shared-server conformance, frontend checks/build, conformance typecheck, Rust affected-consumer checks and lint/format validation.
- [x] 3.2 Validate OpenSpec strictly and report actual results and unavailable host integration coverage.

## Verification results

- Frontend: 51 focused client/mock/ride-continuity tests passed; Svelte check had no errors or warnings; production build passed; changed-file ESLint and Prettier passed.
- Rust: affected product-store, backend-service, loopback-server, shared host, Apple FFI and Android JNI tests passed with the pinned Rust 1.93 toolchain. Warnings-denied Clippy passed for those crates and all targets using Rust 1.95; formatting passed.
- Canonical ride-continuity conformance passed against the mock and direct shared Rust server, including extension, frontend replacement, resume and stale identity rejection. Conformance TypeScript and mandatory method inventory checks passed.
- Chromium browser coverage passed for touch, non-primary pointers, keyboard, duplicate/pending activation and visible failure/retry on Home and the session viewer. WebKit could not launch because this Linux host lacks libicu74, libxml2 and libflite1. Packaged Apple/Android simulator runs were not performed; their unchanged bindings passed shared Rust checks.
- Strict OpenSpec validation and `git diff --check` passed.

### UX wording follow-up

- Replaced Extend pause with Give me more time, explained what happens after expiry, and added a five-second confirmation using the authoritative remaining reconnect time. Renamed the settings label to Keep ride open for (minutes) and clarified when setting changes apply.
- Verification on macOS: focused ride-continuity unit tests (4), Svelte check, production build, changed-component ESLint, Prettier, conformance TypeScript, strict OpenSpec validation and Chromium/WebKit interaction tests passed. Browser coverage checks confirmation after success, no confirmation after failure, and automatic dismissal. Screenshots were captured from the built frontend with the mock backend.
