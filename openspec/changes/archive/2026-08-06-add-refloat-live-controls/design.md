## Context

Float View currently identifies supported Refloat packages and decodes their telemetry, but deliberately rejects package writes. The requested controls are implemented by Refloat package commands rather than the VESC motor configuration protocol, and their wire formats vary across the exact Refloat releases that Float View supports. The shared SvelteKit frontend cannot own those details: the Apple and Android application services must expose authoritative control state through the existing authenticated loopback contract.

The concurrent `support-vesc-bms-telemetry` change introduces a connection-wide `VescRequestScheduler`. Refloat control requests must use that scheduler so direct and CAN-routed requests cannot race riding telemetry, BMS polling, or one another. Implementation of this change therefore follows the scheduler portion of that change, or rebases its equivalent into the connection service first.

The upstream Refloat sources show stable commands for package enablement and flywheel mode across the audited releases, two lighting encodings across release families, and fatal-error clearing only in Refloat 1.2.x. Several commands are one-way and must be confirmed from subsequent telemetry rather than treated as successful when bytes are written.

Upstream VESC firmware adds `COMM_SHUTDOWN` as command 156 after VESC 6.02. VESC Tool sends the exact payload `[156, 0, 0]`, where the first zero prohibits forced shutdown while the motor is spinning and the second selects shutdown rather than restart. The command is one-way. Hardware compiled with a shutdown hold pin powers off; the common fallback returns false and does nothing, and the protocol exposes no shutdown-capability bit. Float View can therefore offer the command on audited writable VESC 6.05 and later while warning that unsupported controller hardware may remain on.

## Goals / Non-Goals

### Goals

- Add authoritative controls for Refloat package enablement, flywheel mode, LEDs, headlights, and conditional fatal-error clearing.
- Add an explicit, safety-gated controller shutdown action for audited writable stable VESC 6.05, 6.06, and 7.00 firmware without claiming that one-way transmission proves power-off.
- Support every exact Refloat stable release already classified as supported by Float View, while keeping test-version access read-only.
- Present normal controls in the existing `/live` connected-board action menu, but present active flywheel mode as a non-dismissible application overlay with one Disable action so the recovery path cannot be hidden in a disabled menu.
- Preserve riding telemetry priority and route every command and response through the connection-wide scheduler.
- Confirm one-way operations from fresh Refloat telemetry and fail closed on unknown versions, stale connections, unsafe state, or malformed responses.
- Reject every device mutation while authoritative package mode is flywheel except the exact command 22 transition needed to disable flywheel and return to normal mode.
- Treat controller or transport loss following a user-requested shutdown as intentional and suppress automatic reconnection without disconnecting a controller that ignores the command.
- Exercise identical behavior through the mock backend, native implementations, canonical conformance suite, and interactive fixture.

### Non-Goals

- Refloat Tunes/profile discovery or selection, including Float View Tunes.
- Reverse stop, simple start, Handtest, remote/RC movement, runtime tuning, package data recording/history, or arbitrary configuration editing.
- Persisting Refloat control state in Float View or restoring controls automatically after reconnect.
- Adding controls for unsupported, development, or test Refloat versions.
- Forced shutdown, controller restart, direct or broadcast `CAN_PACKET_SHUTDOWN`, automatic shutdown, or retrying a shutdown command.

## Decisions

### 1. Expose semantic operations and authoritative state, never raw package bytes

The backend contract adds `refloat.control.set` for the four boolean controls, `refloat.fatalError.clear` for the guarded action, and `controller.shutdown` for the VESC-level action. Complete `state.changed` snapshots include a `refloat.controls` value with availability, writability, nullable observed values, fatal-error state, and at most one pending action, plus bounded active-connection controller-action state that reports whether shutdown may be requested and whether shutdown intent is pending.

The frontend renders only that normalized state and invokes semantic operations. It does not select command numbers, build payloads, infer support from a version string, or optimistically mutate a control. A successful shutdown result means only that the native service accepted the command for transmission; the UI never presents it as proof that controller power turned off. This keeps protocol ownership in the native application service and keeps Apple, Android, mock, and conformance behavior comparable.

Control state is connection-scoped and live-only. Disconnect, board-generation change, or WebView recreation clears it to loading/unavailable until the native service observes the current connection again. Nothing is written to Float View persistence.

### 2. Route exact releases through a closed control-profile table

Each native backend maps the exact package version discovered by the existing Refloat compatibility classifier to a closed control profile. The profile contains only audited capabilities and codecs:

| Refloat family | Package enable | Flywheel   | LEDs/headlights                          | Fatal clear |
| -------------- | -------------- | ---------- | ---------------------------------------- | ----------- |
| 1.0.0–1.0.1    | command 12     | command 22 | command 202, one-byte mask/flags         | unavailable |
| 1.1.0–1.1.2    | command 12     | command 22 | command 202, one-byte mask/flags         | unavailable |
| 1.2.0–1.2.3    | command 12     | command 22 | command 20, uint32 mask plus uint8 flags | command 36  |

Package command 12 accepts the package's `disabled` value, so the semantic `enabled` input is inverted by the codec. Flywheel command 22 uses the exact audited on/off payloads. Lighting updates use masks so LEDs and headlights remain independently addressable. Fatal clear sends only the audited clear value.

Unknown versions, known test versions, and a response whose route or generation no longer matches the request never fall back to a nearby codec. They expose read-only or unavailable state.

VESC-level shutdown uses a separate exact core capability table because command 156 does not exist in the audited VESC 6.02 source:

| VESC firmware | Shutdown command                          |
| ------------- | ----------------------------------------- |
| 6.02          | unavailable                               |
| 6.05          | `[156, 0, 0]` on stable writable firmware |
| 6.06          | `[156, 0, 0]` on stable writable firmware |
| 7.00          | `[156, 0, 0]` on stable writable firmware |

The table deliberately does not use a hardware-name allowlist. Upstream hardware without a shutdown implementation treats the command as a no-op, and firmware provides no authoritative capability bit. Test builds remain read-only and unknown versions remain unsupported. A direct target receives the three-byte command; a selected CAN target receives it once through the existing `COMM_FORWARD_CAN` route. Float View never emits the separate low-level `CAN_PACKET_SHUTDOWN` packet.

Alternative considered: read, edit, and rewrite the complete Refloat configuration. This was rejected because it would require versioned codecs for unrelated settings, widen the write surface, and create a lost-update risk. The dedicated package commands provide the requested behavior without arbitrary configuration access.

### 3. Add a native Refloat control owner above the protocol codec

Apple and Android each add a mirrored Refloat control owner in the application-service layer. It consumes typed package information/telemetry events, selects the exact control profile, applies safety policy, submits typed requests to `VescRequestScheduler`, and publishes normalized control state. Protocol code remains responsible only for encoding, decoding, route correlation, and typed events.

The connection service broadens the existing native `RefloatMutationCoordinator` into one connection-scoped device mutation coordinator (internal name may vary) used immediately before scheduler submission. It is the sole authority for Refloat writes and controller shutdown; owners may not retain a parallel lock or direct write path. The coordinator permits one mutation at a time per board connection. A duplicate Refloat request for the already confirmed value is idempotent and sends no packet. A request that conflicts with the pending action returns `invalid_state`; requests without a supported writable connection return `unavailable` or `invalid_state` according to the contract.

When authoritative package mode is flywheel, the boundary rejects every device mutation, including package enablement, lighting, and fatal clearing. The sole exception is the exact command 22 transition that disables flywheel. Refloat's source accepts that off transition while its run state is active, calls `state_flywheel_off`, and then calls `flywheel_stop` before returning to normal mode. Float View therefore does not apply its derived ride-state gate to this one escape command; the UI tells the rider to disengage first, and no other command family gains an exception.

### 4. Confirm effects from telemetry and bound every operation

Lighting commands have a command response and update their observed values only after the response is matched to the active route and connection generation. A low-cadence lighting read (target: once per second while the Refloat control surface is available, skipped when higher-priority work is due) detects changes made by VESC Tool or another client.

Package enablement, flywheel, and fatal clear are one-way commands. Their operations stay pending until a fresh Refloat telemetry sample satisfies the expected predicate:

- package enablement matches the requested enabled state;
- flywheel mode matches the requested state;
- fatal clear observes the fatal latch cleared while no active firmware fault is present.

An operation times out after three seconds, returns a stable `unavailable` result, clears its pending marker, and retains the last observed state. Disconnect, route change, or connection-generation change cancels the operation immediately. No command is reported as successful merely because the transport accepted its bytes.

### 5. Preserve telemetry priority through the connection-wide scheduler

All Refloat reads and writes and the one-way VESC shutdown request are scheduler transactions. Due riding telemetry remains highest priority; an explicit user control follows it and precedes background lighting refresh and BMS discovery/polling. Only one request owns the matching response route at a time, and canceled or late responses cannot update a newer connection generation.

The scheduler may defer the one-second lighting refresh while higher-priority work is active, but it must not queue catch-up reads. This bounds traffic and avoids degrading the live dashboard during riding.

### 6. Apply explicit safety and interaction gates

Package disablement and flywheel enablement require an idle board according to the existing live-board motion state. Disabling the package and enabling flywheel also require user confirmation in `/live`; the flywheel confirmation explains the package's upright/nose-up engagement expectation. Flywheel disable is the sole motion-state exception because flywheel's running package state is itself normalized as riding and upstream command 22 performs the ordered stop. Refloat remains the final authority for that stop sequence.

Controller shutdown requires authoritative ride state to be exactly idle, a ready writable stable VESC 6.05-or-later core profile, the current route and generation, non-flywheel package mode, and no competing mutation. Float View adds no footpad, balancing-state, or hardware-name precondition. The payload always keeps the firmware's independent non-forced RPM guard enabled. The `/live` action requires a destructive confirmation explaining that not every controller supports software shutdown and that unsupported hardware may remain on.

LED and headlight changes are allowed while riding because they are operational lighting controls, but not while package mode is flywheel. Fatal-error clear is shown only for Refloat 1.2.x and enabled only when the fatal latch is present, no active firmware fault is reported, the board is idle, the package is not in flywheel mode, and the connection is writable. Package enablement is likewise unavailable in flywheel mode. The only actionable mutation while flywheel is active is an eligible flywheel-disable request. The fatal latch bit is decoded separately from the active fault/alert field so clearing a historical latch cannot conceal a current fault.

The menu uses the existing primary-pointer activation pattern plus a guarded keyboard/assistive click path. Pending controls are disabled, and confirmation dismissal or completion restores focus to the invoking menu item. When authoritative state reports flywheel enabled, a full-application modal overlay blocks the normal interface, explains that the rider must disengage first, and exposes only a primary-pointer and keyboard-operable Disable button. It cannot be dismissed while flywheel remains enabled; the backend accepts only the exact source-defined flywheel-off transition and the overlay remains until fresh telemetry confirms normal mode.

### 7. Make the fixture stateful but retain a strict allowlist

The VESC test device gains a small normalized Refloat control model. It recognizes only the exact command/profile combinations above for representative stable packages, mutates simulated package/flywheel/lighting/fatal state, and emits the same responses or subsequent telemetry that a real package uses. It also recognizes only the exact `[156, 0, 0]` shutdown request for eligible VESC profiles and simulates target disappearance without powering off the ESP32 fixture. Direct and CAN-routed scenarios share the model.

The fixture continues to reject every other package write, malformed payload, unsupported command, and all writes for test-version scenarios. Canonical conformance covers each release-family codec, safety failure, timeout/cancellation, reconnect reset, direct route, and CAN route. This replaces the former blanket-write rule with a narrower audited allowlist rather than generally enabling package writes.

### 8. Model shutdown as accepted intent followed by observed connection loss

`controller.shutdown` sends exactly one one-way request and returns only that it was accepted for transmission. Immediately before submission, the native service records bounded shutdown intent against the current connection generation. It does not retry, synthesize a device acknowledgement, or proactively tear down a connection that remains healthy.

If the selected controller or BLE transport disappears while that intent is current, the backend treats the loss as user initiated, ends the active session, and suppresses automatic reconnection. The intent remains current for a bounded 30-second window so Android GATT status 8 and equivalent BLE link-supervision timeouts can report real controller power loss after the one-way write. This covers the common topology where a motor controller power-off also removes the attached VESC Express. For a CAN-routed controller whose independently powered gateway remains connected, loss of the selected controller is treated the same way. If neither target nor transport disappears within that window, the backend clears shutdown-pending state and continues normally because unsupported hardware is permitted to ignore the command.

Alternative considered: disconnect immediately after the write. This was rejected because it would hide an unsupported-hardware no-op and could make the UI falsely imply that shutdown succeeded.

## Risks / Tradeoffs

- **Package enablement command 12 is source-stable but not a public VESC contract.** Exact-version profiles, upstream byte fixtures, and fail-closed version matching limit exposure. A future Refloat version remains unavailable until audited.
- **One-way commands can be accepted but ignored.** Telemetry confirmation and a three-second timeout avoid false success, at the cost of a short pending state.
- **Lighting state can change outside Float View.** A one-second best-effort refresh makes the menu converge without adding high-frequency traffic; values may briefly lag another client.
- **Fatal latch and active fault are easy to conflate.** Separate normalized fields and the no-active-fault gate make the distinction testable in both native implementations.
- **Refloat temporarily changes in-memory configuration while flywheel is active.** A fail-closed mutation boundary permits only flywheel disable until normal mode is authoritatively restored, preventing another write from persisting or interacting with that temporary state.
- **Derived ride state cannot authorize the flywheel escape command.** Refloat reports its active flywheel run state as running, which Float View necessarily normalizes as riding. Applying the generic idle gate would make command 22 off unreachable. The exception is limited to the exact off payload, the overlay instructs physical disengagement, and upstream performs the stop before restoring normal mode.
- **The shared scheduler is being introduced by another active change.** Sequencing this implementation after its scheduler avoids parallel request coordinators and duplicate lifecycle logic, but creates a delivery dependency.
- **More actions increase menu density.** Compact state labels and conditional fatal clear keep the normal connected-board menu usable, while active flywheel mode moves its sole permitted mutation into a blocking overlay so recovery is always visible.
- **Shutdown has no acknowledgement or capability bit.** The result is labelled as command acceptance, the confirmation warns about unsupported hardware, and continued connectivity is treated as a valid no-op rather than an error or success signal.
- **An idle classification can still lag physical reality.** Authorization uses the current authoritative generation and exactly idle ride state immediately before submission, while the immutable `force = 0` payload retains upstream firmware's independent 100-ERPM rejection.
- **A powered-off controller can resemble an unexpected link failure.** Generation-scoped bounded shutdown intent suppresses reconnection only for loss following the explicit action; expiration restores normal recovery behavior.
- **Custom firmware could assign different behavior to command 156.** Float View exposes shutdown only for its exact stable upstream VESC 6.05, 6.06, and 7.00 compatibility profiles and keeps audited test builds read-only.

## Migration Plan

1. Land or integrate the connection-wide request scheduler from `support-vesc-bms-telemetry`.
2. Add golden command/response fixtures, the exact VESC shutdown request fixture, and the normalized interactive-fixture model before enabling native writes.
3. Extend the contract, frontend client/mock, both native services, and conformance schema together so no partial backend is treated as compatible.
4. Implement exact-version codecs and native control owners behind the existing Refloat and VESC compatibility classifications, with one shared mutation coordinator.
5. Add `/live` menu controls and the warned shutdown confirmation only after authoritative state and conformance behavior are available.
6. Run focused frontend/native/conformance tests and the repository-wide `just verify` gate. Physical BLE validation includes real supported-controller shutdown and manual power recovery as an explicit device checkpoint.

Rollback removes the menu actions and semantic handlers while leaving existing Refloat telemetry intact. Because the change adds no database fields or persisted control state, no data migration or cleanup is required; the fixture returns to rejecting the removed commands.

## Open Questions

None blocking. The command mappings, supported versions, safety gates, confirmation policy, shutdown-intent lifecycle, polling target, and timeout are fixed by this design; implementation may adjust internal names without changing the contract semantics.
