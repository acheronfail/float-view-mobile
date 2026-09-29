## Context

See proposal.md for motivation and scope. The existing `app-appearance` specification covers early frontend theme resolution, but not the native surfaces visible before it. Current tests in `frontend/src/lib/app/theme.spec.ts` exercise an abstract environment, not the inline bootstrap in `frontend/src/app.html`. Canonical conformance checks persisted theme settings and frontend lifecycle continuity, not rendered startup frames.

The uncommitted candidate changes add an iOS adaptive launch asset, a matching SwiftUI background and explicit startup-failure state, WebView canvas colors, Android window/splash resources, macOS appearance propagation, and early HTML CSS. They remain subject to review and the acceptance below. In particular, native appearance ownership belongs under `theme`, and the initial callback order and appearance changes after window recreation need verification.

The previous verification attempt passed native packaging/unit checks and direct/macOS/iOS conformance, but the full gate failed in Android conformance. The emulator disappeared during one run; a restarted run timed out waiting for authoritative state. Separate linting reported existing errors in untouched frontend mock/BMS files. These observations are diagnostic context, not acceptance evidence for the final implementation. Subsequent checks and recording results are tracked in acceptance.md; earlier failures do not describe the final gate result.

## Goals / Non-Goals

**Goals:** A continuous background through OS launch, host creation and first web paint; correct theme reconciliation; preserved failure visibility; repeatable behavioral and visual validation.

**Non-Goals:** New public readiness messages, a separate native theme database, new renderer-recovery policy, a custom splash controller, or hiding the UI until a device/backend is connected. No protocol, storage, or frontend/native ownership changes.

## Decisions

### 1. Match surfaces and use normal platform presentation

Use the application canvas (`#f8fafc` / `#020617`) through the startup chain:

`OS launch surface → native window/container → empty WebView → early HTML → application shell`

- iOS: adaptive named launch color in the asset catalog and `UILaunchScreen`; matching SwiftUI canvas and public WebKit background APIs. Remove the normal loading message. Keep actual startup failure distinguishable from background teardown and foreground activation.
- Android: light/night window resources, API 31+ system splash background, and a WebView background that exposes the themed native container before document paint. Keep the standard OS splash icon/exit behavior. Limit required device observations to the connected Android phone and retain build coverage of versioned resources; do not introduce a separate splash activity or a new library solely for a background color.
- macOS: show the normal window promptly with an adaptive canvas and WebKit under-page background. Apply native appearance through a small theme-owned adapter using the existing service callback. Retain automated appearance tests for existing and replacement windows; additional manual desktop testing is outside the acceptance scope.
- Shared frontend: paint the HTML canvas before application CSS/modules arrive; execute the existing validated local theme hint before content presentation. Keep platform branches out of the frontend.

A timed overlay or readiness bridge would add delay, hidden-error states and lifecycle resources. Do not add either merely to mask color mismatches. If visual evidence reveals an actual paint-order defect that surface matching cannot solve, investigate it and update this design before adopting additional readiness machinery.

Use only public platform APIs. Guidance: [Apple launching](https://developer.apple.com/design/human-interface-guidelines/launching), [WebKit under-page background](https://developer.apple.com/documentation/webkit/wkwebview/underpagebackgroundcolor), and [Android splash screens](https://developer.android.com/develop/ui/views/launch/splash-screen).

### 2. Acknowledge when theme information becomes available

The OS launch surface uses system appearance before application settings can be read. It cannot reliably match a saved opposite app preference on every platform. Permit that deliberate system-to-app transition; do not promise identical colors before the preference is available or add duplicated authoritative persistence.

Native UI applies the existing service's theme effect; HTML uses its valid local hint until authoritative settings arrive. Missing, invalid or inaccessible local storage falls back to system appearance. A stale valid hint reconciles to authoritative state. Explicit preferences ignore OS appearance changes; returning to `system` must observe the actual current OS preference, including after an AppKit/UIKit override. Ordering must not cause a settled preference to oscillate through default colors.

### 3. Preserve lifecycle and failures

Keep the process-owned runtime, authenticated loopback boundary and existing teardown/recovery policy. Normal absence of a WebView during backgrounding or reconstruction is not an error. An actual loopback startup failure on iOS shows an error only in the foreground and clears after successful retry through the existing activation path.

Delaying or failing document load, losing backend connectivity, or terminating a renderer must not leave content hidden behind a new startup mask. Retain the existing platform/frontend error and recovery paths; fixing previously missing renderer recovery is outside this change. Any new callback/subscription must be replaced or released with its owner and must not create a second runtime or retry loop.

### 4. Test observable behavior, then inspect the transition

Extend existing harnesses rather than create wrapper scripts or tests that merely search resource files for color literals. Add Playwright as a development-only conformance dependency: its Chromium and WebKit engines replace a fake DOM/CSS interpreter and let the actual HTML, CSS and theme controller execute. Install its browser runtimes with `cd conformance && npx playwright install chromium webkit`; no browser library ships in the application. Browser/bootstrap checks must execute the actual app HTML with application modules delayed and inspect computed canvas/theme before hydration. Add only missing cases; retain existing theme resolution/persistence tests.

Canonical startup/lifecycle scenarios and evidence collection belong in `conformance/`; target adapters perform launch, appearance, capture and lifecycle operations. Focused native assertions can live in the existing host test targets. If an essential host UI test is not run by the current gate, wire it into the root verification workflow. Native resource packaging alone is not visual acceptance.

Required device acceptance uses only the currently connected physical iOS and Android devices. Record their model, installed OS version, and WebView version where available when testing; do not assume the iOS simulator version is the phone version. The Android phone previously tested is the API 29 Samsung device; confirm the connected target at execution time.

| Coverage                 | Required checks                                                                                                    | Evidence                                                                                                  |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| Early HTML               | Existing actual-bootstrap checks for hints, inaccessible storage, delayed modules and authoritative reconciliation | Automated browser checks retained in the software gate                                                    |
| Connected iOS device     | Cold launch, background/foreground, system/app theme changes, visible errors and recovery reachable on the device  | Device recordings and existing automated lifecycle checks                                                 |
| Connected Android device | Cold launch, short resume, resume after the existing grace interval, theme changes and Activity recreation         | Device recordings and existing instrumentation; trim-memory checks where the device tooling supports them |
| Failure/delay            | Existing executable delay/failure checks plus failure paths safely reproducible on the connected devices           | Visible errors, no startup mask, unchanged service ownership                                              |
| Other backends/resources | Existing build, unit and conformance checks                                                                        | `just verify`; no additional manual runtime matrix                                                        |

On each connected device, record system light/dark × app system/light/dark. Repeat representative system and opposite saved-theme cold launches at least three times. Review normal lifecycle transitions in light and dark and compare baseline/candidate launches on the same device. Required physical testing is startup/UI testing, not a BLE fixture checkpoint.

Additional devices and OS versions, simulator/emulator visual matrices, macOS manual launch/reopen/renderer recordings, and failure injection that requires unavailable runtime tooling are out of scope for this change. Do not create new simulator/emulator or desktop acceptance work to replace infeasible device checks. Record such exclusions explicitly without marking them passed or leaving them as blocking pending tasks. Existing automated checks, including any simulator/emulator checks already in `just verify`, remain unchanged. Earlier recordings can be retained as supplemental evidence without extending the acceptance matrix. Supported backends, supported OS versions and product behavior requirements remain unchanged.

Capture from before launch through first usable UI, not just a final screenshot. Inspect frames in unobscured application background regions, including safe-area/system-bar boundaries where app-controlled. Pass means no intermediate default white/black surface, transient loading/error message, or extra splash after app handoff, except a documented intentional theme reconciliation. Native OS icon animation, launch snapshots, window chrome and permission prompts are identified separately. Compare before/after timings on the same environment (at least three representative runs); report time to first usable UI and verify there is no added fixed dwell. Do not impose a hardware-independent millisecond target.

Store a small acceptance report with scenario results, environment, commit/diff identity, capture links and timing observations under this change. Keep large recordings outside Git in retained artifacts, not only transient `/tmp` logs. Redact tokens, addresses and telemetry. Visual acceptance is required only on the two connected physical devices as listed above. The phone startup checks do not add a physical BLE checkpoint or require the ESP32 fixture.

## Risks / Trade-offs

- OS launch caches can show an old launch asset → validate at least one clean-install launch and one installed-app relaunch; distinguish stale OS snapshots from current resource output.
- Native appearance overrides can feed back into web system-theme detection → cover explicit-to-system and OS changes after background/window recreation.
- An early CSS rule can mask later theme changes → assert computed backgrounds before and after reconciliation, not just dataset values.
- Transparent WebViews can expose a wrong container color → inspect native and web surfaces together, including safe areas and system bars.
- Existing conformance failures can hide regressions → diagnose and resolve or separately repair the gate failure before marking this change accepted; do not weaken the suite.
- A final screenshot can miss a single-frame flash → require transition recordings and repeated representative launches.

## Migration Plan

Commit the planning artifacts first, preserving the existing candidate source edits as uncommitted work. Review and complete implementation in coherent commits with task updates. No database migration, reset or compatibility path is needed. Rollback consists of reverting presentation changes; user settings and session data remain untouched. After focused/visual acceptance and a passing `just verify`, sync the delta specs and archive in the final commit.
