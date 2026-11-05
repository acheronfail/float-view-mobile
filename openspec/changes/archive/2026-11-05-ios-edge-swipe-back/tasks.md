# Tasks

## 1. Native gesture and capability

- [x] 1.1 Add the closed frontend capability and shared Back eligibility check; verify host absence, availability and existing Back behavior with focused tests.
- [x] 1.2 Add native edge recognition, completion thresholds and lifecycle reset; verify iOS compilation and simulator UI gesture coverage for completion, cancellation, vertical/non-edge gestures and restored parent position.

## 2. Integration verification

- [x] 2.1 Run affected frontend checks, formatting/lint, frontend build, native checks and navigation browser regressions; record results and limits.
- [x] 2.2 Validate and sync the app-navigation requirement after verification.

## Workflow follow-up

- Archive once implementation and required verification are complete.

## Verification evidence

- Frontend Svelte/TypeScript check: zero errors/warnings. Focused host capability, navigation cache and activation Vitest coverage: 16 tests passed. Changed frontend ESLint/Prettier and conformance TypeScript/format checks passed.
- Frontend production build passed (existing single-bundle size warning).
- Chromium and WebKit: navigation restoration and guarded navigation browser files, four cases passed. Coverage includes host availability, primary-page no-op, edge return to deep lists and guarded board-editor dismissal.
- iOS simulator Debug build and `FloatViewLifecycleTests/testNativeEdgeBackGesture` passed on a disposable iPhone simulator. Covers short/non-edge/vertical drags, completed edge return, restored parent scroll, primary-page no-op and fresh WebView capability publication. Initial native test exposed WebKit recognition interference; simultaneous recognition resolved it. Temporary gesture diagnostics removed after verification.
- SwiftFormat passed for the changed Swift files. No backend, storage or other host contract changes.
- Device gesture feel and chart/map edge arbitration remain useful user-trial feedback; no physical-device result is claimed.
- OpenSpec change validation has no issues; canonical app-navigation validation retains only the same three pre-existing requirement-length warnings.
