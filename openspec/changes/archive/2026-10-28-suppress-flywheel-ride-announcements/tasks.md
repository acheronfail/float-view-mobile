# Tasks

## 1. Automatic speech exclusion

- [x] 1.1 Implement connection-scoped flywheel suppression, cancellation and qualification reset; verify service tests cover every automatic rule, entry/exit, missing mode, cooldowns and late callbacks.
- [x] 1.2 Verify manual previews, visual electrical warnings and connection invalidation remain independent with focused service tests; document the policy and deferred statistics/GPS review in this change.

## 2. Integration checks

- [x] 2.1 Run backend-service and loopback-server consumer tests, Rust formatting/lint checks and strict OpenSpec validation; record actual results.

## 3. Session footpad overlays

- [x] 3.1 Exclude recorded flywheel intervals from both footpad fault overlays; verify entry/exit, same-time ordering, sparse retained samples and pagination with focused detector/loader tests, and document the behavior.
- [x] 3.2 Run frontend type/lint/format checks, build and focused Chromium/WebKit footpad overlay coverage, plus strict OpenSpec validation; record actual results.
