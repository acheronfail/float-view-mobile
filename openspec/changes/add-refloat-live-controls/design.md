## Context

Float View currently identifies supported Refloat packages and decodes their telemetry, but deliberately rejects package writes. The requested controls are implemented by Refloat package commands rather than the VESC motor configuration protocol, and their wire formats vary across the exact Refloat releases that Float View supports. The shared SvelteKit frontend cannot own those details: the Apple and Android application services must expose authoritative control state through the existing authenticated loopback contract.

The concurrent `support-vesc-bms-telemetry` change introduces a connection-wide `VescRequestScheduler`. Refloat control requests must use that scheduler so direct and CAN-routed requests cannot race riding telemetry, BMS polling, or one another. Implementation of this change therefore follows the scheduler portion of that change, or rebases its equivalent into the connection service first.

The upstream Refloat sources show stable commands for package enablement and flywheel mode across the audited releases, two lighting encodings across release families, and fatal-error clearing only in Refloat 1.2.x. Several commands are one-way and must be confirmed from subsequent telemetry rather than treated as successful when bytes are written.

## Goals / Non-Goals

### Goals

- Add authoritative controls for Refloat package enablement, flywheel mode, LEDs, headlights, and conditional fatal-error clearing.
- Support every exact Refloat stable release already classified as supported by Float View, while keeping test-version access read-only.
- Present the controls in the existing `/live` connected-board action menu with clear current, unavailable, and pending states.
- Preserve riding telemetry priority and route every command and response through the connection-wide scheduler.
- Confirm one-way operations from fresh Refloat telemetry and fail closed on unknown versions, stale connections, unsafe state, or malformed responses.
- Exercise identical behavior through the mock backend, native implementations, canonical conformance suite, and interactive fixture.

### Non-Goals

- Refloat Tunes/profile discovery or selection, including Float View Tunes.
- Reverse stop, simple start, Handtest, remote/RC movement, runtime tuning, package data recording/history, or arbitrary configuration editing.
- Persisting Refloat control state in Float View or restoring controls automatically after reconnect.
- Adding controls for unsupported, development, or test Refloat versions.

## Decisions

### 1. Expose semantic operations and authoritative state, never raw package bytes

The backend contract adds `refloat.control.set` for the four boolean controls and `refloat.fatalError.clear` for the guarded action. Complete `state.changed` snapshots include a `refloat.controls` value with availability, writability, nullable observed values, fatal-error state, and at most one pending action.

The frontend renders only that normalized state and invokes semantic operations. It does not select command numbers, build payloads, infer support from a version string, or optimistically mutate a control. This keeps protocol ownership in the native application service and keeps Apple, Android, mock, and conformance behavior comparable.

Control state is connection-scoped and live-only. Disconnect, board-generation change, or WebView recreation clears it to loading/unavailable until the native service observes the current connection again. Nothing is written to Float View persistence.

### 2. Route exact releases through a closed control-profile table

Each native backend maps the exact package version discovered by the existing Refloat compatibility classifier to a closed control profile. The profile contains only audited capabilities and codecs:

| Refloat family | Package enable | Flywheel | LEDs/headlights | Fatal clear |
| --- | --- | --- | --- | --- |
| 1.0.0–1.0.1 | command 12 | command 22 | command 202, one-byte mask/flags | unavailable |
| 1.1.0–1.1.2 | command 12 | command 22 | command 202, one-byte mask/flags | unavailable |
| 1.2.0–1.2.3 | command 12 | command 22 | command 20, uint32 mask plus uint8 flags | command 36 |

Package command 12 accepts the package's `disabled` value, so the semantic `enabled` input is inverted by the codec. Flywheel command 22 uses the exact audited on/off payloads. Lighting updates use masks so LEDs and headlights remain independently addressable. Fatal clear sends only the audited clear value.

Unknown versions, known test versions, and a response whose route or generation no longer matches the request never fall back to a nearby codec. They expose read-only or unavailable state.

Alternative considered: read, edit, and rewrite the complete Refloat configuration. This was rejected because it would require versioned codecs for unrelated settings, widen the write surface, and create a lost-update risk. The dedicated package commands provide the requested behavior without arbitrary configuration access.

### 3. Add a native Refloat control owner above the protocol codec

Apple and Android each add a mirrored Refloat control owner in the application-service layer. It consumes typed package information/telemetry events, selects the exact control profile, applies safety policy, submits typed requests to `VescRequestScheduler`, and publishes normalized control state. Protocol code remains responsible only for encoding, decoding, route correlation, and typed events.

The owner permits one Refloat control action at a time per board connection. A duplicate request for the already confirmed value is idempotent. A request that conflicts with the pending action returns `invalid_state`; requests without a supported writable Refloat connection return `unavailable` or `invalid_state` according to the contract.

### 4. Confirm effects from telemetry and bound every operation

Lighting commands have a command response and update their observed values only after the response is matched to the active route and connection generation. A low-cadence lighting read (target: once per second while the Refloat control surface is available, skipped when higher-priority work is due) detects changes made by VESC Tool or another client.

Package enablement, flywheel, and fatal clear are one-way commands. Their operations stay pending until a fresh Refloat telemetry sample satisfies the expected predicate:

- package enablement matches the requested enabled state;
- flywheel mode matches the requested state;
- fatal clear observes the fatal latch cleared while no active firmware fault is present.

An operation times out after three seconds, returns a stable `unavailable` result, clears its pending marker, and retains the last observed state. Disconnect, route change, or connection-generation change cancels the operation immediately. No command is reported as successful merely because the transport accepted its bytes.

### 5. Preserve telemetry priority through the connection-wide scheduler

All Refloat reads and writes are scheduler transactions. Due riding telemetry remains highest priority; an explicit user control follows it and precedes background lighting refresh and BMS discovery/polling. Only one request owns the matching response route at a time, and canceled or late responses cannot update a newer connection generation.

The scheduler may defer the one-second lighting refresh while higher-priority work is active, but it must not queue catch-up reads. This bounds traffic and avoids degrading the live dashboard during riding.

### 6. Apply explicit safety and interaction gates

Package enable/disable and flywheel changes require an idle board according to the existing live-board motion state. Disabling the package and enabling flywheel also require user confirmation in `/live`; the flywheel confirmation explains the package's upright/nose-up engagement expectation. Refloat remains the final authority for its own internal engagement checks.

LED and headlight changes are allowed while riding because they are operational lighting controls. Fatal-error clear is shown only for Refloat 1.2.x and enabled only when the fatal latch is present, no active firmware fault is reported, the board is idle, and the connection is writable. The fatal latch bit is decoded separately from the active fault/alert field so clearing a historical latch cannot conceal a current fault.

The menu uses the existing primary-pointer activation pattern plus a guarded keyboard/assistive click path. Pending controls are disabled, and confirmation dismissal or completion restores focus to the invoking menu item.

### 7. Make the fixture stateful but retain a strict allowlist

The VESC test device gains a small normalized Refloat control model. It recognizes only the exact command/profile combinations above for representative stable packages, mutates simulated package/flywheel/lighting/fatal state, and emits the same responses or subsequent telemetry that a real package uses. Direct and CAN-routed scenarios share the model.

The fixture continues to reject every other package write, malformed payload, unsupported command, and all writes for test-version scenarios. Canonical conformance covers each release-family codec, safety failure, timeout/cancellation, reconnect reset, direct route, and CAN route. This replaces the former blanket-write rule with a narrower audited allowlist rather than generally enabling package writes.

## Risks / Tradeoffs

- **Package enablement command 12 is source-stable but not a public VESC contract.** Exact-version profiles, upstream byte fixtures, and fail-closed version matching limit exposure. A future Refloat version remains unavailable until audited.
- **One-way commands can be accepted but ignored.** Telemetry confirmation and a three-second timeout avoid false success, at the cost of a short pending state.
- **Lighting state can change outside Float View.** A one-second best-effort refresh makes the menu converge without adding high-frequency traffic; values may briefly lag another client.
- **Fatal latch and active fault are easy to conflate.** Separate normalized fields and the no-active-fault gate make the distinction testable in both native implementations.
- **The shared scheduler is being introduced by another active change.** Sequencing this implementation after its scheduler avoids parallel request coordinators and duplicate lifecycle logic, but creates a delivery dependency.
- **More actions increase menu density.** Compact state labels, conditional fatal clear, and the existing sheet/confirmation components keep the connected-board menu usable without adding a separate settings surface.

## Migration Plan

1. Land or integrate the connection-wide request scheduler from `support-vesc-bms-telemetry`.
2. Add golden command/response fixtures and the normalized interactive-fixture model before enabling native writes.
3. Extend the contract, frontend client/mock, both native services, and conformance schema together so no partial backend is treated as compatible.
4. Implement exact-version codecs and native control owners behind the existing Refloat compatibility classification.
5. Add `/live` menu controls only after authoritative state and conformance behavior are available.
6. Run focused frontend/native/conformance tests and the repository-wide `just verify` gate. Physical BLE validation remains the explicit device checkpoint.

Rollback removes the menu actions and semantic handlers while leaving existing Refloat telemetry intact. Because the change adds no database fields or persisted control state, no data migration or cleanup is required; the fixture returns to rejecting the removed commands.

## Open Questions

None blocking. The command mappings, supported versions, safety gates, confirmation policy, polling target, and timeout are fixed by this design; implementation may adjust internal names without changing the contract semantics.
