## Why

Launching Float View on iOS visibly changes surfaces before the WebView appears. Startup presentation is not specified or visually verified across iOS, macOS, and Android, so passing backend tests cannot establish a smooth launch or return to the app.

## What Changes

- Align the OS launch surface, native canvas, WebView background, and earliest HTML paint with the application palette and the best available theme at each stage.
- Use each platform's normal launch/window behavior without artificial splash durations, a second branded splash, or waiting for Bluetooth/backend connectivity before showing frontend content.
- Remove the temporary iOS loading message; distinguish normal frontend absence from an actual startup failure.
- Preserve appearance when disposable WebViews return after backgrounding, recreation, or existing recovery paths; retain native service ownership and error visibility.
- Add targeted behavioral coverage and a recorded visual acceptance matrix for cold launch, lifecycle transitions, theme changes, and failure paths. Require a passing software gate before completion.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-appearance`: Extend initial-theme requirements to the entire visible startup handoff, including unavailable or stale local hints and system/app theme differences.
- `platform-hosts`: Specify native startup presentation, lifecycle continuity, bounded resources, and visible failure behavior.
- `backend-conformance`: Require focused startup regressions and visual evidence alongside the existing software gate.

## Impact

Affects Apple launch resources and host UI/theme/lifecycle wiring, Android theme resources and window/WebView integration, shared frontend HTML bootstrap, and focused tests/evidence owned by `conformance/`. No public wire-contract, Rust policy, persistence format, or firmware compatibility changes are intended. No new production dependency is planned. Superseded loading presentation is removed rather than retained as a fallback; no migration or compatibility machinery is introduced.

Non-goals: a new splash brand/animation, a general startup-performance overhaul, new renderer-recovery policy, Bluetooth changes, and a new physical BLE checkpoint. Required device acceptance is limited to the currently connected physical iOS and Android devices, on their installed OS/WebView versions. Additional devices, OS versions, simulator/emulator acceptance matrices, and macOS manual visual/lifecycle testing are out of scope because they are not feasible for this change. Existing automated checks and the `just verify` software gate remain in scope. Earlier recordings remain supplemental evidence, not required follow-up work. Implementation still covers iOS, macOS and Android; supported OS versions are unchanged.

Existing uncommitted edits are a candidate implementation to review against these artifacts. They do not establish task completion or visual acceptance.
