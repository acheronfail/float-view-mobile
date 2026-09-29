# Startup presentation acceptance

Status: implementation and evidence collection in progress; not accepted.
Baseline: `fae1dff3` (before startup presentation edits). Planning commit: `fae14389`.

## Candidate review and requirement mapping

| Requirement                                         | Candidate / remaining work                                                                    | Verification                                                                                                        |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Stable initial appearance                           | Early HTML canvas and validated hint retained; real browser checks pass                       | Early-render tests: all hints, inaccessible storage, computed colors before hydration, authoritative reconciliation |
| Continuous startup appearance                       | Retain adaptive iOS asset and Android window/splash resources; verify native/WebView handoff  | Recorded light/dark cold launches, opposite saved themes, repeated launches and baseline timings                    |
| Native startup presentation                         | Retain public WebKit APIs and platform splash; macOS theme application moved out of lifecycle | Host builds, native appearance checks, no-device launch and macOS reopen                                            |
| Startup presentation through frontend lifecycle     | Existing service ownership stays intact; native lifecycle visual/behavior coverage missing    | Retained/disposed resume, activity recreation, existing renderer recovery and bounded repeated recreation           |
| Startup failures remain distinguishable and visible | Retain explicit iOS failure state and normal absence handling                                 | Bind failure/retry and background transitions; failed navigation/backend connection remain reachable                |
| Startup presentation acceptance evidence            | Visual evidence collected in part; final software gate passed                                 | Final strict OpenSpec validation, focused tests, recordings and passing `just verify`                               |

No wire-contract, persistence or protocol change is needed. No new production dependency is needed. Existing source edits remain candidate work until their mapped acceptance checks pass.

## Approved acceptance scope

Required device acceptance is limited to the currently connected physical iOS and Android devices on their installed software. Record the iOS phone model/OS at execution time; prior iOS 26.3.1 results below came from a simulator, not the connected phone. The Android phone previously exercised is the API 29 Samsung device; confirm it when testing.

All other devices and OS versions, simulator/emulator visual acceptance matrices, and manual macOS launch/lifecycle recordings are out of scope by user direction because further platform testing is not feasible. These are exclusions, not pending completion requirements. Failure injection requiring unavailable tooling is likewise excluded and must not be reported as passed. Implementation and supported OS versions remain unchanged across iOS, macOS and Android. Existing automated tests and `just verify` remain required.

Earlier simulator, emulator and desktop observations below are retained as supplemental evidence. Their unfinished manual follow-ups are superseded by this scope decision and no longer block acceptance. The connected iOS visual matrix and Android phone visual matrix remain required and incomplete; the phone instrumentation result does not replace those recordings. No ESP32 or new physical BLE checkpoint is required.

## Focused checks

- Real Chromium/WebKit startup checks: 26 tests passed. Actual HTML bootstrap, compiled CSS and transpiled production theme controller cover 12 system/hint combinations per engine, pre-hydration computed colors, stale hint reconciliation, blocked storage, OS changes, explicit-to-system and listener cleanup.
- macOS native appearance tests: 2 passed; existing/new windows, ten replacements, explicit/system override removal and adaptive color resolution. Manual macOS full-app lifecycle checks are now out of scope.
- Connected Android phone is API 29; it can provide minimum-version physical observations. Additional Android versions are out of scope.

- Android startup canvas and repeated Activity recreation instrumentation: passed on API 36 emulator and connected API 29 Samsung phone (three recreations, one runtime and one attached WebView). No app data reset or BLE fixture was required.
- Frontend lint: the existing mutable mock binding and unkeyed BMS lists were repaired; formatter and ESLint now pass.
- Initial iOS baseline captures were obscured by a persistent OS location permission prompt. They are diagnostic only, not clean timing acceptance. Restarting the test simulator after granting test location authorization cleared the stale prompt; unobstructed captures are collected separately.

- The new browser and Android startup commands are in `just verify`; browser installation is in `just setup`. macOS tests run in the existing Swift test target workflow. The emulator-only Android runner rejects physical serials so the software gate does not require hardware.
- Unobstructed baseline iOS recording confirms a white content frame between the dark native canvas and dark frontend. Source baseline is `fae1dff3`; theme is `system`, iOS 26.3.1. Capture directory: `/Users/cosmotherly/Desktop/codex/visualizations/2026/09/29/01a0eaca-e976-7aa0-ae44-df57a22ddc30/startup/`; files `ios-baseline-unobstructed-{light,dark}-{1,2,3}.mp4`. Earlier files are retained as prompt-obstructed diagnostics only.

## Earlier visual observations (supplemental)

Evidence directory: `/Users/cosmotherly/Desktop/codex/visualizations/2026/09/29/01a0eaca-e976-7aa0-ae44-df57a22ddc30/startup/`.

| Check                                                      | Result                                                     | Evidence / limitation                                                                                                                                                                               |
| ---------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| iOS 26.3.1 system light/dark, three cold launches each     | Pass for canvas handoff                                    | `ios-candidate-system-*.mp4`; clean-installed candidate, matching native/early-web background                                                                                                       |
| iOS baseline dark flash                                    | Reproduced in all three unobstructed dark trials           | `ios-baseline-unobstructed-dark-*.mp4`; center-region analysis found 3, 4, 4 white samples at 60 Hz (about 50–67 ms) between the dark canvas and first controls                                     |
| iOS candidate dark flash                                   | Not present in the three sampled dark trials               | Same center-region analysis found 0 white samples in each candidate dark trial                                                                                                                      |
| iOS saved light/dark × system light/dark                   | Captures retained; further review out of scope             | `ios-candidate-{lightpref,darkpref}-*.mp4`, three trials per combination; reviewed opposite dark preference shows intentional light-to-dark reconciliation                                          |
| iOS actual bind failure and foreground recovery            | Pass                                                       | `ios-startup-bind-failure.png` shows explicit error; `ios-startup-bind-recovery.png` shows restored frontend after freeing the port and returning from background                                   |
| Android API 36 system light/dark                           | Captures retained; further review out of scope             | `android-{baseline,candidate}-authorized-*.mp4`, three trials per combination. Reviewed candidate dark clip retains platform icon animation on a matching dark canvas without a white WebView flash |
| Android API 29 physical phone                              | Native UI test pass                                        | Canvas colors and three real Activity recreations; no physical visual recording yet                                                                                                                 |
| macOS 26.7 native appearance                               | Focused tests pass; manual cold-launch checks out of scope | `macos-{baseline,candidate}-window.mov` begin after content appears and establish steady presentation only; they cannot certify the launch transition or timing                                     |
| Unavailable version coverage                               | Unverified                                                 | Superseded matrix: all versions/devices other than the two connected phones are now outside required device acceptance                                                                              |
| Renderer failure, all resume modes, unavailable navigation | Incomplete                                                 | Only applicable connected-device checks remain required; extra desktop/runtime checks and fault injection requiring unavailable tooling are out of scope                                            |

`ios-timing-analysis.json` and `analyze-ios.py` document approximate video landmarks. Median first-tab appearance from recording start was 1.83 s for both baseline and candidate dark trials; light medians were 1.78 s and 2.18 s, respectively. The runs include OS animation, cold-install effects and concurrent build load. This is evidence of the eliminated dark flash, **not** a statistically established speedup or a verified touch-response latency budget. No fixed dwell or readiness wait was added. Before/after timing observations on the two connected devices remain required; additional platform timing studies are out of scope.

The initial Android captures without `authorized` and initial iOS captures without `unobstructed` contain permission dialogs and remain diagnostic only. Tile contact sheets include padded empty cells after short recordings; those cells are not captured black app frames. The unavailable-Bluetooth simulator can show a frontend service error; it is visible in both baseline and candidate and is not hidden by the new canvas.

## Final software gate

`just verify` passed end to end on the final implementation tested before the matrix-only documentation revision. This includes frontend/native checks, canonical direct/macOS/iOS/Android conformance, the new browser and Android startup checks, and ownership/simulation audits. Full output: `/Users/cosmotherly/Desktop/codex/visualizations/2026/09/29/01a0eaca-e976-7aa0-ae44-df57a22ddc30/startup/final-verify.log`.

The earlier Android disappearance/state timeout did not recur in the successful full run; no protocol assertions were weakened. Frontend lint repairs also passed. This software result does not complete the outstanding visual/lifecycle acceptance on the two connected devices.

## Completion run on connected hardware

- iPhone 12, iOS 18.7.8 (22H352): all six system/app-theme combinations passed the focused XCTest startup/foreground test, each with three cold launches and three returns from background. The successful baseline light/dark tests also have recordings. XCTest screen recordings were used because this phone does not support `devicectl` screen recording and USB AVFoundation capture was unavailable.
- Samsung SM-G965F, Android 10/API 29, WebView 153.0.8010.36: the expanded native test passed retained-resume identity, actual 30-second grace expiration, memory-pressure disposal, and three Activity recreations with the same runtime and exactly one replacement WebView. Deterministic lifecycle callbacks isolate retention from OS memory policy; real background recordings are collected separately.
- Production frontend HTTP 503 bootstrap failure remains visible with reachable navigation in Chromium and WebKit. Browser startup coverage now passes 28 tests.
- The final implementation and expanded regressions passed `just verify`; output is retained as `completion-verify.log` in the evidence directory. No protocol/conformance assertion was weakened.
- Changed-file Prettier checks, SwiftFormat and frontend lint passed. An additional whole-repository `just lint` run reports 39 unrelated/generated formatting findings; these files are outside the implementation diff. This is separate from the passing required `just verify` gate.
- Android's standard `cmd uimode night` command is ignored by this Samsung firmware (`mNightModeLocked=true`). Earlier recordings labeled light without verified system configuration are retained under `diagnostic-unverified-mode-*`; they are not acceptance evidence. Corrected runs use the actual Display settings control and assert the resulting system mode.
- USB interruptions required reconnecting and retrying Android captures. A temporary wireless-debugging attempt was unreachable from the host; USB-only debugging was requested again. Remaining corrected Android visuals are still being collected at this checkpoint.
