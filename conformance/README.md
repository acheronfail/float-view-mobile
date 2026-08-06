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
1.2.0–1.2.3; legacy 1.0.x reports wire version
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

The separate physical control checkpoint must run each native backend against
both direct and CAN fixture routes. Exercise idempotent reads/writes first, then
package, lighting, flywheel enable/disable, and eligible fatal clear while the
fixture reports idle. Record confirmation, timeout/recovery, and reconnect reset,
and explicitly assert that no motor command or movement occurred. Unavailable
hardware is recorded as unavailable rather than replaced with software evidence.

The controller-shutdown matrix accepts only exact VESC-core `[156, 0, 0]` on
writable stable VESC 6.05, 6.06, and 7.00 routes. It covers direct and one-level
CAN forwarding, idle-only authorization, flywheel and competing-mutation
exclusion, malformed/forced/restart/broadcast/repeated rejection, selected-node
or transport loss without powering down the ESP32, and a deterministic
unsupported-hardware no-op. Shutdown is accepted-only, one-way, and never
retried; neither the simulator nor contract state asserts confirmed power-off.

## Physical BMS checkpoint

The explicit hardware checkpoint remains separate from `just verify`. Run the
available direct-controller-plus-CAN-VBMS, Express-plus-CAN-VBMS, and
direct-VBMS-gateway combinations on both Apple and Android targets with the
`npm run physical` commands above. Record unavailable hardware combinations as
unavailable; do not replace them with weaker software assertions. The ESP32
firmware stays a byte bridge—scenario composition and VBMS interpretation remain
in this directory.

Retained observations live under `physical-checkpoints/`. The Thor301/VBMS32
capture is recorded in `physical-checkpoints/2026-08-06-thor301-vbms32.md`.
