# Conformance

This is the only executable acceptance suite for the frontend/backend contract.
It imports the hand-written contract from `frontend/`, drives every backend as a
black box, and owns both deterministic and physical VESC simulation.

Custom dashboards have canonical Board Profile validation/atomicity coverage in
`src/suite.ts` for mock, direct and native targets. After building the frontend,
run `node --strip-types --test test/custom-dashboard.browser.ts` from this directory
(outside the macOS Codex sandbox). It checks Chromium/WebKit at narrow portrait,
short landscape, tablet/unfolded-width and wide desktop sizes, plus edit/control
isolation, compatible ranges, save conflicts, warning modal layering and restoration.
Screenshots go to ignored `conformance/generated/custom-dashboard/`. Viewport emulation does not
establish physical hinge or fold-device gesture integration.

```sh
npm ci
npm test
npm run mock
npm run physical -- macos <fixture-port>
npm run physical -- android <fixture-port> <adb-serial>
npm run physical -- ios <fixture-port> <device-id>
```

Set `FLOAT_VIEW_PHYSICAL_REFLOAT_VERSION=1.3.0` to run the controller scenarios
with the audited selectable-telemetry package, including ride, reconnect, and
background coverage. The explicit legacy and standalone-BMS scenarios retain
their original identities. Without this selection the older regression matrix
is retained, with the control/shutdown scenarios using 1.3.0.

When a connected mobile device cannot acquire location promptly, rerun the
background scenario with
`FLOAT_VIEW_PHYSICAL_SCENARIOS=background FLOAT_VIEW_DEFER_PHYSICAL_LOCATION=1`.
This explicitly defers location authorization and fix acquisition to manual
verification; the runner still requires background lifecycle recovery,
continuous BLE telemetry, session growth, and non-regressing persistence
boundaries.

Android, iOS, and macOS adapters launch or locate their application and expose a
loopback WebSocket URL; scenarios and assertions remain shared. Physical runs
add a serial-controlled `tools/vesc-test-device` byte bridge. Passing runs retain
no evidence; failures print bounded first-failure context. The physical adapters
contain only launch, loopback tunneling, lifecycle, and cleanup controls; all
contract and VESC assertions remain in shared TypeScript.

## Live viewport rendering

Run `npm run test:live-viewports` from `conformance/` to build the current frontend
and render the normal and detailed live views across 24 CSS viewports in Chromium
and WebKit. Light/dark themes cover normal saved/BMS/guest states and detailed
telemetry (384 render cases, plus bottom-of-scroll captures). The matrix includes
compact phones, modern iPhones and Androids, browser-control height reductions,
tablets, landscape, and square/unfolded shapes. Dimensions come from the installed
Playwright device registry plus explicit compact, landscape and square cases.

Open `generated/live-viewports/index.html` for the browser/theme/scenario gallery.
The same directory contains original PNGs, labelled contact sheets and JSON
geometry reports. Set `FLOAT_VIEW_VIEWPORT_OUTPUT` to choose another output path.
These are web-content emulations; physical OS safe-area insets are not simulated.

The suite rejects horizontal overflow, overlapping widgets, clipped battery bars
or centre boxes, header targets smaller than 44 px and out-of-bounds location
panels. Detailed rows are checked for wrapping, clipping and label/value overlap;
representative widths verify one, two and three telemetry columns, and the final
value must remain reachable by scrolling. Ordinary saved/BMS portrait and modern
landscape views fit without scrolling. The shortest landscape and short guest layouts retain vertical
scrolling, with checks that the entire battery section remains reachable.

## On-demand native layout captures

`Capture iOS layout` is a separate simulator diagnostic, not part of release builds
or the ordinary test matrix. Explicitly dispatch the registered workflow for a
pushed `apple-duo/*` verification branch (CLI dispatch was verified before default-branch integration):

```sh
gh workflow run capture-ios-layout.yml --ref apple-duo/side-controls
```

The workflow has no push, pull-request or schedule trigger. Pushing a branch
never requests a layout capture automatically. A job-level branch guard skips dispatches
from main, tags, and branches outside `apple-duo/*` before allocating a runner.
It is not called by the main checks or distribution workflows.
The job selects Xcode 27.1 and the installed Duo runtime, starts the simulator while
building, and captures the real display plus a bounded native/WebView geometry report.
The diagnostic starts on Sessions to avoid discovery permission prompts. It accepts
the safety agreement only inside an explicitly opted-in Debug simulator instance.
No device data, release signing, publishing, or physical BLE run is involved.
For a one-off device-SDK compilation check, pass `-f verify_device_build=true`;
this adds an unsigned device build after capture and is disabled by default.
The iOS distribution jobs also select Xcode 27.1 so shipped builds can use the
full Duo display; local older-SDK builds retain ordinary navigation.

Download the run's `ios-layout-<run-id>` artifact. It records source, SDK/runtime,
phase timestamps, native window/WebView bounds and safe-area insets, system status
and reserved regions, CSS viewport/insets, and control rectangles. Preserve useful
captures under ignored `.local/physical-testing/ios-layout/`; simulator captures
establish layout behavior, not physical-device validation.

A reusable unsigned native app artifact is keyed by tracked Apple, core, SQL and toolchain
content, using Git tree entries rather than scanning generated build directories.
Frontend dependency artifacts are keyed by tracked lockfile/package contents, OS/architecture
and exact Node version. Reuse accepts only unexpired artifacts from this branch and repository,
and checks their embedded input keys before extraction. Frontend-only iterations rebuild and replace the embedded `index.html` in that
matching diagnostic app, skipping native compilation. Native input changes rebuild
it; a separate Rust cache retains Rust/toolchain work even if a later diagnostic fails.
The source and native-cache key accompany every report. Artifact expiration and runner
startup still cause cold runs: use recorded timestamps rather than assuming a fixed
duration. No simulator or runner remains allocated after the job ends.

For the fast UI loop, build the frontend and run the focused fixture test:

```sh
(cd frontend && npm run build)
(cd conformance && node --strip-types --test test/side-controls.browser.ts)
```

Both Chromium and WebKit run against the captured native bounds; the focused tests
measured about three seconds after building. These previews exclude native status UI.
Use manual CI for occasional native confirmation. The verified artifact-reuse run
[40470640834](https://github.com/acheronfail/float-view-mobile/actions/runs/40470640834)
skipped native compilation and dependency installation, but still took 6m55s from
setup to screenshot because artifact transfers and simulator capture dominated.
Treat that as a measured sample rather than a guaranteed turnaround.

## Interactive fixture development

`just fixture dev [port]` starts the development-only SvelteKit controller on
`127.0.0.1:11004`. Run it beside `just dev <platform>` to operate the physical
ESP32 byte bridge manually. Reusable protocol and simulation code stays in
`fixture/`; server/UI ownership and operating details are documented in
`fixture-dev/README.md`.

The reusable simulator supports direct controllers, VESC Express and generic
VESC-protocol gateways, explicit or omitted CAN targets, multiple motor
controllers, unsupported/non-motor nodes, distinct controller UUIDs, malformed
or excessive ping lists, silent targets, and route-specific responses. The
compatibility matrix covers every direct/Express combination of VESC
6.02/6.05/6.06/7.00 and stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, and
1.2.0–1.2.3, and selectable command-33 Refloat 1.3.0; legacy 1.0.x reports wire version
`1.0` and uses fixed packet 10.

VBMS simulation is independent of controller routing. Canonical CAN-BMS
scenarios use the source-derived VESC Express VBMS32 6.06 identity (`VBMS32`,
hardware type `2`), while a separate fixture retains stock VESC BMS 6.00/type-1
coverage. A scenario may contain no BMS, one direct BMS endpoint with a CAN controller, or one or more CAN BMS
nodes beside a direct or CAN controller. Nodes can be omitted from ping while
remaining directly addressable, return legacy or extended values, time out, or
return bounded malformed variants. Command 96 is accepted only as a read on the
configured direct or CAN VBMS route; other BMS commands are rejected. The three
software native targets use a direct THOR300 plus CAN-42 VBMS and require an
attached, decoded BMS snapshot during black-box conformance.

When multiple VBMS identities are visible, Float View reports ambiguity until
the Board Profile's `preferredBmsCanId` selects one. Clearing that value resumes
automatic discovery; it is independent from the controller's `preferredCanId`.
After an identity has been detected, the live menu retains a BMS sheet through
attached, ambiguous, and stale/unavailable states. BMS values are live-only and
must not appear in session review, persistence, diagnostics, or exports.

The manual workbench and automated runner deliberately have different jobs.
The workbench retains an editable scenario and reports requests without driving
Float View or judging outcomes. The runner owns deterministic assertions and
physical checkpoints. Shareable workbench documents contain only bounded public
simulation inputs; controller UUID bytes stay private to the simulator server.

The telemetry matrix covers Refloat 1.0.0 command 1 plus command 10 detail
levels 1–4, and Refloat 1.1.0/1.2.3 command 31/32 plus periodic command 10
detail 4. Refloat 1.2+ also polls command 35 for bounded firmware fault history;
commands 41–43 remain deferred.

For the focused fault checkpoint, run
`FLOAT_VIEW_PHYSICAL_SCENARIOS=fault-direct,fault-can npm run physical -- macos <fixture-port>`
(or the Android/iOS target with its device ID). It injects a fault start and end
between telemetry polls, verifies the retained latch and recovered reason across
BLE reconnect, rejects clearing an active fault, confirms an eligible clear, and
checks that the completed idle session retains each history event exactly once.
The fixture resets to READY on cleanup. This tests native BLE transport with
source-derived Refloat responses; it does not induce a fault on a real controller.

For Refloat 1.3.1-beta2 IMU timeout behavior over the production BLE route, run
`FLOAT_VIEW_PHYSICAL_SCENARIOS=imu-timeout-beta2 npm run physical -- macos <fixture-port>`.
The conformance-owned scenario injects active, cleared and transient command-35
records through the ESP32 byte bridge, checks the live warning, guarded write
rejection and saved history, then restores the fixture to READY. It does not
induce a sensor failure on a real board.

The product-control matrix covers exact commands 12, 22, 202, 20, and 36 for
representative Refloat 1.0.0, 1.1.0, and 1.2.3 direct and CAN routes. Its
disposable state model confirms one-way writes through later telemetry and
lighting through exact responses, while rejecting malformed, wrong-route,
test-version, moving, active-fault, competing, stale, and flywheel-prohibited
work. Exact flywheel disable remains permitted while the active flywheel run
state is normalized as riding, matching Refloat's stop-before-mode-change
command behavior. The ESP32 remains a byte bridge and has no motor-motion
command path.

Select physical checkpoints using the root [verification policy](../AGENTS.md#verification).
The procedures below describe coverage when the corresponding behavior is affected;
they do not require every checkpoint for every change.

For a physical control checkpoint, run the affected native hosts and direct/CAN
routes. Exercise idempotent reads/writes first, then
package, lighting, flywheel enable/disable, and eligible fatal clear while the
fixture reports idle. Record confirmation, timeout/recovery, and reconnect reset,
and explicitly assert that no motor command or movement occurred. Unavailable
hardware is recorded as unavailable rather than replaced with software evidence.

The Refloat configuration checkpoint is also separate from `just verify`. For
the affected hosts and direct/CAN fixture routes, capture complete current
and default values, change exactly one fixture-safe non-actuating field, verify
the full readback, then restore and verify the complete original configuration.
The run fails on an unverified restore and must record that no motor/remote,
Handtest, history, arbitrary app-data, or other actuation command was observed.
Use `physical-checkpoints/refloat-configuration-template.md`; unavailable real
hardware remains explicitly unavailable rather than being replaced by software
evidence.

The controller-shutdown matrix accepts only exact VESC-core `[156, 0, 0]` on
writable stable VESC 6.05, 6.06, and 7.00 routes. It covers direct and one-level
CAN forwarding, idle-only authorization, flywheel and competing-mutation
exclusion, malformed/forced/restart/broadcast/repeated rejection, selected-node
or transport loss without powering down the ESP32, and a deterministic
unsupported-hardware no-op. Shutdown is accepted-only, one-way, and never
retried; neither the simulator nor contract state asserts confirmed power-off.

## Physical BMS checkpoint

The explicit hardware checkpoint remains separate from `just verify`. Run the
affected available direct-controller-plus-CAN-VBMS, Express-plus-CAN-VBMS, and
standalone-direct-VBMS combinations on the relevant Apple or Android targets with the
`npm run physical` commands above. Record unavailable hardware combinations as
unavailable; do not replace them with weaker software assertions. The ESP32
firmware stays a byte bridge—scenario composition and VBMS interpretation remain
in this directory.

Reusable procedures and brief historical outcomes live under
[`physical-checkpoints/`](physical-checkpoints/README.md), including the
Thor301/VBMS32 compatibility summary. Keep detailed device results and raw
reports under ignored `.local/physical-testing/`; do not commit completed
local result templates.

`npm run direct` runs the canonical suite against the production Rust service
constructor with conformance-owned protocol simulation and synthetic OS facts.
It contains no product CRUD, SQL, archive or reconnect policy and is included in
`just verify` alongside native conformance.

The macOS physical target runs the production save presenter in a disposable app
bundle. Its ride checkpoint pauses for at most 120 seconds: inspect and dismiss
the native Save panel, then press Enter to acknowledge that visual check. Android
asserts and dismisses its scoped document picker; iOS uses its lifecycle UI test
runner to inspect and dismiss the share sheet. Background-location acceptance
requires a persisted observation during the background dwell; lack of a sample
is incomplete physical coverage, not an automatic deferral.

## Temporal battery verification

`fixtures/battery-temporal.json` contains independently calculated charge/load
oracles consumed by `rust/battery-temporal.rs` (compiled in backend-service's
unit tests) and the frontend mock tests. Charge arithmetic is current × elapsed
seconds / 3600 / full-pack Ah, integrated on the original curve axis before
zero-load reserve remapping. Numerical comparisons use a 1e-12 fraction tolerance;
UI checks round the authoritative fraction only at display time.

The vectors cover 9 Ah at 60 A for one second (1/60 Ah, 1/540 charge fraction),
irregular signed trapezoids, observed charging, reserve remapping, an 80-to-30
voltage observation step limited to 0.1 displayed percentage points per second,
and gaps with no invented charge consumption. Additional canonical Rust cases
cover low-load settling/convergence, renewed load, unstable voltage, missing
current/voltage, charge saturation and extreme valid reserve settings. Service
integration tests exercise rejected updates, duplicate/reordered samples,
multiple accepted observations before publication, controller replacement,
reconnects and effective versus identity-only edits. The runtime persistence-gate
test establishes that rejected writes cannot reach the estimator. Direct and
host suites assert preset/estimate contracts; Chromium/WebKit cover both editors
and live views with the mock.

No suitable captured acceleration/release/rest ride traces or reference charge
measurements were found in the repository during the 2026-09-30 implementation.
These synthetic results establish deterministic stability and accounting, not
measured absolute SOC accuracy. Initial 30-second smoothing/settling and
120-second correction constants remain unchanged; no empirical tuning is claimed.

`npm run measure` writes its synthetic performance report to ignored
`.local/physical-testing/software-measurements.json` by default. Pass an explicit
local output filename as its argument when retaining separate runs.

## Auto-connect checkpoint

`FLOAT_VIEW_CONFORMANCE_SCENARIOS=auto-connect npm run direct` (or `mock`,
`macos`, `ios`, `android`) runs focused preference and connection coverage without
unrelated location/export prompts. Targets with lifecycle controls also verify
foreground rearming, preserved Settings checkpoint, and explicit-disconnect
suppression during ordinary discovery.

For real BLE, use `FLOAT_VIEW_PHYSICAL_SCENARIOS=auto-connect npm run physical --
android <fixture-port> <adb-serial>` or the equivalent `ios` command. This uses the
existing direct fixture, saved connection path and RESET/READY cleanup, and
rejects configuration/control/shutdown mutations. The fixture is readvertised
before foreground return and before the post-disconnect suppression scan. macOS
window/activation behavior and first-use OS permission presentation remain
separate physical observations; headless software targets do not establish them.

`test/last-live-view.browser.ts` covers saved view restoration without a process checkpoint, reconnects, failed preference writes and dashboard editor revision consistency in Chromium and WebKit. Board independence and durable reopen are also covered by product-store tests.

## Focused announcement coverage

`FLOAT_VIEW_CONFORMANCE_SCENARIOS=announcements npm run direct` (or `mock`,
`macos`, `ios`, `android`) checks the complete announcement settings contract,
new disabled/one-second footpad defaults, invalid delay atomicity and existing
manual previews on each host. `node --strip-types --test test/announcements.test.ts`
uses the existing interactive telemetry source and Refloat codec through the
direct OS relay to check automatic footpad-only speech, shared repeats, combined
effects, contact recovery and full requalification after reconnect. It observes
typed speech effects and simulates completion; it does not establish audibility.

Build the frontend before running `test/announcements.browser.ts` as described
in the [development reference](../docs/development.md#verification). Chromium and WebKit cover guarded toggles, native delay
slider mouse/touch/keyboard edits, retained disabled delay, frontend reload and
failed/serialized autosaves. Native Swift/Kotlin speech tests check exact
left/right/both wording, footpad-first ordering and the existing audio lifecycle.
