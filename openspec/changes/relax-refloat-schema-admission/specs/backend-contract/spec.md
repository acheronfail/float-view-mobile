## MODIFIED Requirements

### Requirement: Small authoritative configuration availability

`ActiveConnection.refloatConfiguration` SHALL remain a small authoritative availability summary with exact observed schema identity and request-scoped bulk results. It SHALL additionally expose boolean `ridingTuneApplySupported` from backend capability policy. Limited support SHALL keep writability and riding-Tune support false, independently of readable schema/values; live changes SHALL publish complete monotonic snapshots.

#### Scenario: Ready configuration

- **WHEN** schema validation and a fresh current read complete for a ready writable connection
- **THEN** the next authoritative snapshot reports ready and writable with matching version/signature and loaded current values

#### Scenario: Reconnecting connection

- **WHEN** the active device reconnects
- **THEN** configuration becomes loading and non-writable while retained bulk request results cannot authorize a write

#### Scenario: Slow frontend

- **WHEN** telemetry snapshots continue while a schema page or value request is outstanding
- **THEN** authoritative state remains bounded and conflated without embedding or repeatedly delivering the bulk result

#### Scenario: Flywheel configuration contract

- **WHEN** authoritative package mode is flywheel
- **THEN** `read(current)`, `write`, and `tune.apply` return `invalid_state`, schema/default reads may remain available, and the summary reports no loaded authoritative current values

#### Scenario: Limited readable configuration

- **WHEN** a limited connection has a valid matching read profile and fresh normal-mode current values
- **THEN** the summary can be ready/current-loaded while writable and ridingTuneApplySupported remain false; `writeBlockReason` reports `unverifiedFirmware` when no discovery/loading/unavailable condition takes priority

#### Scenario: Backend-owned riding Tune capability

- **WHEN** the frontend considers riding Tune application or recreates its connection state
- **THEN** it consumes `ridingTuneApplySupported` and current authoritative safety state without its own release allowlist or version comparison
- **AND** a true capability does not bypass exact Tune identity, fresh revision or submission-time mutation checks

### Requirement: Bounded Refloat control contract

`ActiveConnection.refloatControls` SHALL be null before Refloat discovery begins and otherwise SHALL contain `status` as `loading`, `ready`, or `unavailable`; `writable`; nullable `packageEnabled` and `flywheelEnabled`; `lightingAvailable`; nullable `ledsEnabled` and `headlightsEnabled`; `fatalErrorClearAvailable`; nullable `fatalErrorLatched`, `fatalErrorActive`, and `fatalErrorClearable`; and nullable `pendingAction` from `package`, `flywheel`, `leds`, `headlights`, or `clearFatalError`. `refloat.control.set` SHALL accept exactly one `control` from `package`, `flywheel`, `leds`, or `headlights` and one boolean `enabled`. `refloat.fatalError.clear` SHALL accept empty parameters. A successful method SHALL return the complete confirmed `RefloatControls`; unsafe, read-only, stale, unavailable, or competing requests SHALL use the existing stable error vocabulary and MUST NOT return optimistic partial state.

#### Scenario: Idempotent requested value

- **WHEN** `refloat.control.set` requests a value already confirmed by current authoritative state
- **THEN** the backend returns the unchanged complete controls without sending a board mutation

#### Scenario: Confirmed control result

- **WHEN** an accepted control is confirmed by its protocol response or later telemetry
- **THEN** the request returns complete controls matching the newer `state.changed` snapshot and the pending action is null

#### Scenario: Invalid control name

- **WHEN** `refloat.control.set` supplies an unknown control or malformed parameter
- **THEN** the backend returns `invalid_request` without sending a board command

#### Scenario: Control unavailable

- **WHEN** a control is requested before controls are ready, during reconnect, for absent lighting or fatal capability, on read-only firmware, on a limited Refloat connection, or while flywheel mode prohibits that mutation
- **THEN** the backend returns `invalid_state` without changing board or authoritative control state

#### Scenario: Flywheel disable remains actionable

- **WHEN** an audited control profile is active and authoritative state reports active flywheel mode and the user requests flywheel disabled, including while its package run state is normalized as riding
- **THEN** the backend may perform only the exact source-defined flywheel-disable operation while every other mutating control remains unavailable

#### Scenario: Confirmation timeout

- **WHEN** an accepted action cannot obtain its required bounded response or confirmation
- **THEN** the backend returns `unavailable`, clears `pendingAction`, and retains the last confirmed values

## ADDED Requirements

### Requirement: Live Refloat support summary

`ActiveConnection.refloatSupport` SHALL be null before compatibility is established and for BMS connections, otherwise contain only `mode` (`verified` or `limited`) and boolean `alertHistorySupported`. It SHALL describe current-generation backend evidence, publish atomically with readiness/feature availability and remain absent from persisted sessions and archives.

#### Scenario: Validated limited support

- **WHEN** a public read probe succeeds for an unknown Refloat release
- **THEN** the complete ready snapshot reports limited support with the actual firmware identity, unavailable/non-writable controls and configuration availability derived independently
- **AND** alertHistorySupported is false, active fault/fatal/timeout state remains unknown and an empty history does not imply absence of alerts

#### Scenario: Verified support

- **WHEN** an audited compatibility mapping is selected
- **THEN** support is verified and alertHistorySupported reflects that profile's actual alert interface, rather than version spelling, telemetry shape or schema equality

#### Scenario: Reconnect or frontend replacement

- **WHEN** reconnect starts, the route/generation changes or the frontend is recreated
- **THEN** reconnect clears old support until new evidence establishes it, and frontend recreation recovers only current complete state without granting permission from retained request results

#### Scenario: BMS and saved history

- **WHEN** a BMS connection or a saved session is displayed
- **THEN** no live Refloat support decision is inferred for it and historical data remains readable under its existing contract

### Requirement: Preserved bounded configuration summary

The configuration summary SHALL retain bounded lifecycle, identity, loaded-value and pending-operation facts without embedding bulk data or acquiring protocol ownership in the frontend.

#### Scenario: Existing summary and lifecycle fields

- **WHEN** configuration state is projected
- **THEN** `ActiveConnection.refloatConfiguration` is null before Refloat discovery begins and otherwise contains only bounded status from `loading`, `ready`, or `unavailable`; writability; nullable exact package version until legacy schema identity resolves; nullable schema fingerprint and wire signature; current/default-loaded flags; boolean riding-Tune capability; nullable write-block reason; and nullable pending operation from `schema`, `readCurrent`, `readDefault`, `write`, or `applyTune`. It MUST NOT contain schema fields, raw XML, raw values, saved configurations, private controller identity, or cache keys. Entry into flywheel mode SHALL clear the current-loaded flag and writability without discarding validated schema/default availability; confirmed exit SHALL not restore them until a fresh current read succeeds. Availability and pending-operation changes SHALL publish newer complete authoritative snapshots, while bulk schema/value results remain request-scoped.
