## Why

Float View can observe Refloat state but cannot perform the small set of routine controls riders need while connected, forcing them to switch to VESC Tool for package enablement, flywheel, lighting, and recovery from a cleared fatal fault. Adding a bounded, safety-gated native control path makes those actions available from the shared `/live` menu without opening broader tuning or motor-control access.

## What Changes

- Add authoritative connected-board control state for Refloat package enablement, flywheel mode, LED power, headlight power, lighting availability, and a latched-versus-active fatal fault.
- Add mandatory backend operations to refresh and set the supported Refloat controls through the selected direct-or-CAN controller route, with exact audited Refloat-version wire profiles, bounded completion, read-back or telemetry confirmation, and centralized rejection on read-only firmware.
- Extend the `/live` header menu with stateful Refloat enable/disable, flywheel, LEDs, and headlights actions plus a conditional Clear fatal error action when the underlying firmware fault is no longer active.
- Enforce native safety policy: persistent package disablement and flywheel transitions require an idle board in an allowed package mode; flywheel engagement follows Refloat's ready/upright behavior; while flywheel mode is active every device mutation is rejected except the exact safety-gated command 22 transition that disables flywheel; lighting remains runtime-only and is shown only when supported; fatal clearing is unavailable while the source fault remains active.
- Extend the mock, conformance simulator, interactive fixture model, immutable protocol fixtures, and canonical suite with successful, rejected, timed-out, malformed, version-specific, direct, CAN-routed, reconnect, and read-only control cases.
- Replace the existing blanket rejection of Refloat lighting, flywheel, and configuration writes with an exact allowlist for these product controls while continuing to reject every other mutation or actuator command.

Non-goals are Refloat Tunes/profiles, reverse stop, simple start, Handtest, remote or RC motor movement, runtime tuning, package data recording, alert history, arbitrary package configuration editing, persistent lighting preferences, and support for Refloat releases outside the audited 1.0.0–1.2.3 catalog.

## Capabilities

### New Capabilities

- `refloat-live-controls`: Define the supported control state, exact versioned commands, safety gates, route-correct execution, confirmation, lifecycle, and failure behavior.

### Modified Capabilities

- `backend-contract`: Add mandatory bounded Refloat control methods and complete authoritative active-connection control state.
- `live-board-dashboard`: Add accessible, stateful, capability-gated Refloat actions to the `/live` header menu.
- `refloat-telemetry-pipeline`: Expose the fatal latch separately from an active firmware fault and permit only the newly allowlisted Refloat control requests.
- `vesc-device-connection`: Extend exact compatibility profiles and centralized read/write access checks to the supported control command families.
- `backend-conformance`: Cover every new control method, state transition, safety rejection, wire profile, route, timeout, malformed response, reconnect, and read-only case.
- `interactive-fixture-development`: Model deterministic Refloat control state and allowlisted control responses while retaining strict rejection of unrelated writes and motor actuation.

## Impact

The hand-written TypeScript contract and mock, frontend state and `/live` menu, mirrored Apple and Android application/protocol owners, Refloat/VESC compatibility fixtures, conformance classifier and scenarios, and interactive fixture state change together. Implementation reuses the connection-wide request scheduler introduced by `support-vesc-bms-telemetry`, establishes one native authorization boundary for the allowlisted Refloat mutations, adds no runtime dependency or durable database migration, stores no Tune or arbitrary Refloat configuration data, and removes the affected blanket write-rejection assumptions rather than adding a parallel request path.
