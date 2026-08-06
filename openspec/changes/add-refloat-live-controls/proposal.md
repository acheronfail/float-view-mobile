## Why

Float View can observe Refloat state but cannot perform the small set of routine controls riders need while connected, forcing them to switch to VESC Tool for package enablement, flywheel, lighting, recovery from a cleared fatal fault, and supported-controller shutdown. Adding a bounded, safety-gated native control path makes those actions available from the shared `/live` menu without opening broader tuning or motor-control access.

## What Changes

- Add authoritative connected-board control state for Refloat package enablement, flywheel mode, LED power, headlight power, lighting availability, and a latched-versus-active fatal fault.
- Add mandatory backend operations to refresh and set the supported Refloat controls through the selected direct-or-CAN controller route, with exact audited Refloat-version wire profiles, bounded completion, read-back or telemetry confirmation, and centralized rejection on read-only firmware.
- Add a mandatory `controller.shutdown` operation for writable stable VESC 6.05, 6.06, and 7.00 controllers. It sends only the upstream non-forced, non-restart `COMM_SHUTDOWN` request through the selected direct-or-CAN route, reports only that the command was accepted for transmission, and never claims that unsupported hardware powered off.
- Extend the `/live` header menu with stateful Refloat enable/disable, flywheel, LEDs, and headlights actions, a conditional Clear fatal error action when the underlying firmware fault is no longer active, and a shutdown action whose confirmation warns that unsupported controllers may do nothing. When authoritative telemetry detects active flywheel mode, replace normal application interaction with a non-dismissible modal overlay whose sole action disables flywheel.
- Enforce native safety policy: persistent package disablement, flywheel enablement, and controller shutdown require an idle board; flywheel engagement follows Refloat's ready/upright behavior; the exact flywheel-disable command remains available from active flywheel mode because upstream command 22 owns the stop-before-mode-change sequence; while flywheel mode is active every other device mutation is rejected; lighting remains runtime-only and is shown only when supported; fatal clearing is unavailable while the source fault remains active.
- Treat transport loss after a user-requested shutdown as intentional and suppress automatic reconnection. If the selected controller or transport remains available because shutdown is unsupported, clear the bounded shutdown intent and continue the connection without fabricating success or disconnecting it.
- Extend the mock, conformance simulator, interactive fixture model, immutable protocol fixtures, and canonical suite with successful, no-op, rejected, timed-out, malformed, version-specific, direct, CAN-routed, reconnect, and read-only control cases.
- Replace the existing blanket rejection of Refloat lighting, flywheel, configuration, and VESC-level shutdown writes with an exact allowlist for these product controls while continuing to reject every other mutation or actuator command.

Non-goals are forced shutdown, controller restart, direct `CAN_PACKET_SHUTDOWN` or broadcast shutdown, automatic shutdown or retry, Refloat Tunes/profiles, reverse stop, simple start, Handtest, remote or RC motor movement, runtime tuning, package data recording, alert history, arbitrary package configuration editing, persistent lighting preferences, and support for Refloat releases outside the audited 1.0.0–1.2.3 catalog.

## Capabilities

### New Capabilities

- `refloat-live-controls`: Define the supported control state, exact versioned commands, safety gates, route-correct execution, confirmation, lifecycle, and failure behavior.

### Modified Capabilities

- `backend-contract`: Add mandatory bounded Refloat control methods, `controller.shutdown`, and complete authoritative active-connection control state.
- `live-board-dashboard`: Add accessible, stateful, capability-gated Refloat actions and confirmed controller shutdown to the `/live` header menu.
- `refloat-telemetry-pipeline`: Expose the fatal latch separately from an active firmware fault and permit only the newly allowlisted Refloat control requests.
- `vesc-device-connection`: Extend exact compatibility profiles and centralized mutation checks to the supported Refloat control families and non-forced VESC 6.05+ shutdown.
- `backend-conformance`: Cover every new control method, state transition, safety rejection, wire profile, route, no-op, timeout, malformed request, intentional disconnect, and read-only case.
- `interactive-fixture-development`: Model deterministic Refloat control state and allowlisted shutdown behavior while retaining strict rejection of unrelated writes and motor actuation.

## Impact

The hand-written TypeScript contract and mock, frontend state and `/live` menu, mirrored Apple and Android application/protocol owners, Refloat/VESC compatibility fixtures, conformance classifier and scenarios, and interactive fixture state change together. Implementation reuses the connection-wide request scheduler introduced by `support-vesc-bms-telemetry`, broadens the native connection-scoped mutation boundary to serialize the allowlisted Refloat mutations with controller shutdown, adds no runtime dependency or durable database migration, stores no Tune or arbitrary Refloat configuration data, and removes the affected blanket write-rejection assumptions rather than adding a parallel request path.
