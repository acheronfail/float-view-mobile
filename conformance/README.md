# Conformance

This is the only executable acceptance suite for the frontend/backend contract.
It imports the hand-written contract from `frontend/`, drives every backend as a
black box, and owns both deterministic and physical VESC simulation.

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
detail 4. Command 35 history and commands 41–43 are intentionally deferred.

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

Retained observations live under `physical-checkpoints/`. The Thor301/VBMS32
capture is recorded in `physical-checkpoints/2026-08-06-thor301-vbms32.md`.

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
