# Tasks

## 1. Durable ride ownership

- [x] 1.1 Implement bounded durable pause/resume and settings with baseline-preserving recovery/upgrade tests, including restart, expiry, repeated breaks, wrong-controller and retention cases.
- [x] 1.2 Preserve recording order/timeline and explicit gaps; verify riding time, distance, history/pruning and supported archive imports with focused Rust tests.

## 2. Shared contract and interface

- [x] 2.1 Add paused-ride projection and identity-scoped End ride to contract/client/mock/shared dispatch; verify canonical conformance through both shared host bindings.
- [x] 2.2 Add resume-window setting, Home/session paused actions and Live End ride with accurate confirmation; verify primary/non-primary/keyboard activation and frontend checks/browser coverage.

## 3. Verification

- [x] 3.1 Run affected Rust tests and warnings-denied lint, frontend checks/build, mock/direct conformance and relevant browser tests; document results and any unavailable coverage, then validate the change strictly.
