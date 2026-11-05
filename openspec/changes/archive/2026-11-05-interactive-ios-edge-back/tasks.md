# Tasks

## 1. Interactive Back

- [x] 1.1 Report accepted navigation and completed restoration to the native gesture; verify asynchronous, blocked, failed and ineligible completion in unit/browser tests.
- [x] 1.2 Add viewport snapshot tracking, completion/cancellation and lifecycle cleanup; verify native compilation and simulator gesture coverage, including visual feedback.

## 2. Integration

- [x] 2.1 Run frontend checks/build, formatting, relevant browser regressions and native UI coverage; record results.
- [x] 2.2 Validate and sync the modified navigation requirement.

## Workflow follow-up

- Archive after verification and spec sync.

## Verification evidence

- Frontend Svelte/TypeScript check: zero errors/warnings; host Back, navigation cache and activation Vitest suites: 20 tests passed. Changed frontend ESLint/Prettier and conformance TypeScript/format checks passed.
- Frontend production build passed with the existing single-bundle size warning.
- Chromium/WebKit navigation restoration and guarded navigation: four cases passed. Interactive acknowledgement is checked after restored list position; blocked surface/editor requests report no parent navigation. Unit coverage includes deferred success, rejected/failed navigation and ineligible gestures.
- iOS Debug build and native edge gesture UI test passed, including short/non-edge/vertical gestures, completed return, parent scroll and fresh frontend. A recorded simulator replay was visually inspected during the drag: full viewport and header translate together with edge shadow and neutral Back affordance, then the restored parent is revealed. No live parent preview during the drag is claimed.
- Native startup/foreground presentation test passed after adding WebView dismantling cleanup. Changed Swift files pass SwiftFormat. The moving layer is removed on cancellation, completion, backgrounding, document failure/replacement and WebView teardown; stale snapshot and acknowledgement callbacks are generation-checked.
- OpenSpec delta validation has no issues. Canonical validation retains only its three existing requirement-length warnings.
- No physical-device motion or feel result is claimed; the user will trial the interaction on-device.
