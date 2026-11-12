# Tasks

## 1. Shared listener and startup appearance

- [x] 1.1 Remove fixed host port policy and verify independent listener/authentication tests and host envelope rejection cases.
- [x] 1.2 Embed current validated saved appearance into HTML and preserve it through frontend initialization; verify Rust response tests, focused theme tests and pre-module browser startup coverage.

## 2. Hosts and tooling

- [x] 2.1 Update Android and Apple hosts to request assignment and publish private debug port readiness; update host tests and run affected native build/unit checks.
- [x] 2.2 Discover actual endpoints in conformance and development, preserve device origins and scope forwarding cleanup; update runner tests/docs and run TypeScript checks/tests.

## 3. Integration

- [x] 3.1 Run affected frontend build/checks, shared Rust tests/lints and direct/native conformance/startup software checks; record results and specific unavailable checkpoints.
- [x] 3.2 Verify repeated physical Android/iOS cold launches and resume in light/dark with saved preferences opposite to system appearance; inspect Android recordings and retain user-reported iOS visual acceptance. Distinguish the system launch window from the native/WebView handoff. No BLE fixture is needed.

## Workflow follow-up

- Sync canonical specs, validate the change and touched capabilities, and archive only after required checks complete.

## Verification evidence (2026-11-12)

- Shared loopback crate: 16 tests passed; warnings-denied Clippy and Rust formatting passed. Concurrent listeners, host start envelopes, token isolation and fresh HTML appearance are covered.
- Frontend: production build, Svelte check, affected formatting/lint and 26 focused unit/proxy tests passed. Chromium/WebKit startup file passed all 30 tests, including embedded preferences with absent, stale and inaccessible storage.
- Android: 56 linked JVM tests passed, including independently assigned ports; four-ABI Debug runtime and startup instrumentation compiled. The three startup instrumentation tests and both Debug-app and canonical-target startup conformance passed, including frontend recreation.
- Apple: five affected host tests passed. Native macOS and iOS simulator startup conformance passed; the iOS occupied-former-port UI regression passed.
- User-reported physical iOS acceptance: installation and launch passed, followed by the requested four system/app light-dark combinations, distinctive accent, repeated cold launches and foreground checks. The user reported all worked correctly and authorized Android validation followed by closeout/commit. No raw iOS recording was supplied.
- Tooling: TypeScript check and 14 focused runner tests passed, including scoped Android forwarding replacement and simulated physical-iOS proxy discovery/termination. Focused direct diagnostics conformance passed.
- OpenSpec change and platform-hosts validation have no issues. Backend-conformance has 16 unchanged baseline long-requirement warnings; changed requirements have no warnings. `just spec-check os-assigned-loopback-ports` passed. Canonical specs are synced.
- Broader Swift test run exposed an unchanged loading-artwork mismatch: `MacThemeTests.swift:67` requires a light-mode tint while `MacLoadingView.swift` sets that tint to nil. Affected listener/configuration tests pass; this unrelated assertion remains unfixed.
- Physical Android acceptance: eight recorded cold launches across verified system Light/Dark with app System/opposite explicit theme, plus four resumes, passed with the former fixed port held by a disposable listener. Authenticated state, fresh embedded appearance, usable navigation and retained Rose accent passed. An isolated Debug application ID preserved the installed stable app and its data. Recordings show the existing system-colored Android launch window before saved native appearance; no default-theme reversal occurs during the native/WebView handoff. Detailed device/build inventory, recordings and restoration evidence remain ignored under `.local/physical-testing/os-assigned-loopback-ports/android/`.
- Physical iOS visual acceptance is user-reported; no raw iOS recording or physical USB-proxy check was supplied. Earlier browser/simulator/emulator results remain software evidence. Required mobile startup acceptance is complete; no BLE interoperability claim is made.
