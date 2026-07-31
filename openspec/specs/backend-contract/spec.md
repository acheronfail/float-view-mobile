# Backend Contract Specification

## Purpose

Define the complete platform-neutral contract implemented by every supported native backend and consumed by the shared frontend.

## Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.disconnect`, `savedDevice.update`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.get`, `session.getDiagnostics`, `session.export`, `session.delete`, and `sessionGroup.unlink`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory method is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Authenticated loopback transport

The backend SHALL bind only to loopback and expose `/`, `/api/bootstrap`, and `/api/ws`. Bootstrap SHALL provide the WebSocket URL and a random per-launch token; the WebSocket SHALL validate both token and expected Origin.

#### Scenario: Valid frontend connection

- **WHEN** the bundled frontend uses the current bootstrap token and application Origin
- **THEN** the backend accepts the WebSocket connection

#### Scenario: Invalid authentication

- **WHEN** a WebSocket omits or supplies an incorrect token or Origin
- **THEN** the backend closes or rejects it without processing requests

### Requirement: Bounded request protocol

Requests SHALL use `{ id, method, params }`; successes SHALL use `{ id, result }`; failures SHALL use `{ id, error }`. Every accepted request ID MUST receive exactly one terminal response, and IDs, messages, queues, timeouts, retries, and collections MUST be bounded.

#### Scenario: Successful request

- **WHEN** a valid request is processed
- **THEN** exactly one response with the same ID and a result is returned

#### Scenario: Failed request

- **WHEN** a request cannot be completed
- **THEN** exactly one response with the same ID and a structured error is returned

#### Scenario: Oversized message

- **WHEN** a client exceeds the documented message bound
- **THEN** the backend rejects the message or connection without unbounded allocation

### Requirement: Stable errors

The contract SHALL use only `invalid_request`, `invalid_state`, `permission_required`, `not_found`, `conflict`, `unavailable`, and `internal` as frontend-visible error codes.

#### Scenario: Permission required

- **WHEN** scanning requires an unresolved OS permission
- **THEN** `bluetooth.scan` fails with `permission_required`

#### Scenario: Revision conflict

- **WHEN** settings, a saved-device custom name, or a Board Profile is updated with a stale expected revision
- **THEN** the request fails with `conflict` and persisted data is unchanged

### Requirement: Authoritative application state

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, Bluetooth state, discovered and saved devices with resolved identity, active connection with saved-device identity, firmware, active-session summary, and latest successfully persisted sample when known, recent session summaries, session groups, and settings. Historical series and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all state needed to render the main application and recover the current live dashboard without platform-specific requests

#### Scenario: Bounded collections

- **WHEN** persisted history exceeds frontend collection limits
- **THEN** `AppState` contains only the defined bounded recent collections and one latest live sample

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while the board is reconnecting
- **THEN** initial authoritative state contains the current connection state and retained latest sample when one exists

### Requirement: Authoritative state events

Every frontend-visible coarse state change and every successfully persisted active-session sample SHALL increment the state revision and publish `{ event: "state.changed", data: AppState }`. Sample-driven events SHALL occur at the configured phase-appropriate persistence cadence. The backend SHALL conflate pending state events to the newest complete snapshot rather than allowing an unbounded queue or preserving an older pending snapshot.

#### Scenario: Connection transition

- **WHEN** an active connection changes state
- **THEN** the frontend receives a newer complete `AppState`

#### Scenario: Persisted live sample

- **WHEN** the native persistence gate selects a telemetry sample and its durable append succeeds
- **THEN** the frontend receives a newer complete state containing that sample and resulting active-session aggregates

#### Scenario: Skipped probe

- **WHEN** a decoded telemetry probe is not selected by the configured persistence gate
- **THEN** that probe does not cause a frontend state event

#### Scenario: Event ordering

- **WHEN** multiple changes occur while an event is pending
- **THEN** the eventual event contains the newest authoritative revision and stale revisions do not replace newer frontend state

#### Scenario: Slow frontend

- **WHEN** frontend consumption is slower than persisted sample production
- **THEN** pending delivery converges on the newest complete snapshot without delaying or dropping durable persistence

### Requirement: Backend-owned Bluetooth operations

The backend SHALL own permission resolution, bounded scanning, connection, reconnect, disconnection, protocol polling, and firmware discovery. Exactly zero or one device MAY be active.

#### Scenario: Bounded scan

- **WHEN** the frontend requests `bluetooth.scan`
- **THEN** the backend starts discovery, stops it within the contract bound, and publishes discovered devices through state events

#### Scenario: Competing connection

- **WHEN** a second device is requested while another device is active
- **THEN** the backend returns `invalid_state` and preserves the existing connection

#### Scenario: WebView recreation

- **WHEN** the frontend WebView is destroyed and recreated
- **THEN** the native connection and backend jobs continue where the OS permits and the new frontend recovers them through `state.get`

### Requirement: Backend-owned durable sessions

A ready connection SHALL automatically create one provisional durable session, and all live display, review, diagnostics, and persistence SHALL derive from one sequenced native sample pipeline. The latest live sample SHALL become frontend-visible only after the same sample is selected and successfully appended to that active session. When the session closes or is recovered after interruption, the backend SHALL retain it only if at least one persisted sample entered the `riding` state and its accumulated distance is greater than or equal to `minimumSessionDistance`; otherwise it SHALL delete the catalog entry and session files.

#### Scenario: Ready board

- **WHEN** a supported board becomes ready
- **THEN** the backend starts a provisional durable session without a frontend logging command

#### Scenario: Idle-only connection

- **WHEN** a provisional session closes without any persisted sample entering `riding`
- **THEN** the backend deletes the session even if idle telemetry accumulated distance

#### Scenario: Ride below the configured minimum

- **WHEN** a provisional session entered `riding` but closes below `minimumSessionDistance`
- **THEN** the backend deletes the session instead of exposing it in session history

#### Scenario: Live sample ordering

- **WHEN** a telemetry sample is selected for persistence
- **THEN** its append completes before the state revision exposes that sample and updated session summary

#### Scenario: Unexpected disconnect

- **WHEN** the BLE connection ends unexpectedly during a session
- **THEN** the backend retains the last persisted live sample while reconnecting and closes or recovers the session with deterministic state and integrity metadata

#### Scenario: Active connection removed

- **WHEN** the backend removes or replaces the active connection
- **THEN** its retained live sample and active-session association are cleared

### Requirement: Current frontend product operations

The contract SHALL support revision-checked saved-device custom-name updates, persisted settings, local versioned Board Profiles, bounded session review series, bounded lossless diagnostic windows, native export presentation, deletion, and session-group unlinking with platform-neutral semantics.

#### Scenario: Saved board rename

- **WHEN** the frontend supplies a valid custom display name or clears the override at the current saved-device revision
- **THEN** the backend persists it and returns saved-device state with a newer revision and newly resolved display name

#### Scenario: Session review

- **WHEN** the frontend requests `session.get`
- **THEN** the backend returns the session summary and bounded display series required by the review screen

#### Scenario: Diagnostic review

- **WHEN** the frontend requests diagnostics around a valid elapsed time
- **THEN** the backend returns a bounded lossless window without creating a separate logging path

#### Scenario: Native export

- **WHEN** the frontend requests a supported session export
- **THEN** the backend creates the export and invokes the host platform save or share presentation without frontend platform branching

### Requirement: Disposable current storage schema

The native backends SHALL execute the shared current SQLite schemas, retain WAL recovery and integrity handling, and recreate test-stage storage on schema-version mismatch rather than running migrations.

#### Scenario: Schema mismatch

- **WHEN** the current storage version differs from persisted test data
- **THEN** the backend deletes its logging directory and recreates the current schema

#### Scenario: Interrupted session recovery

- **WHEN** the backend starts with an interrupted active session
- **THEN** it performs bounded integrity recovery and exposes the recovered result through the current contract

### Requirement: Complete persisted settings shape

The mandatory `Settings` value SHALL contain `revision`, `idleProbeIntervalMs`, `idlePersistIntervalMs`, `ridingPersistIntervalMs`, `postRideTailMs`, `reconnectDurationMs`, `minimumSessionDistance`, `automaticSessionLinkingEnabled`, `sessionLinkWindowMs`, `themePreference`, and `unitPreference`. `minimumSessionDistance` SHALL be expressed in meters and default to `100`; `themePreference` SHALL accept exactly `system`, `light`, or `dark`; `unitPreference` SHALL accept exactly `metric` or `imperial` and default to `metric`; `rawPacketCaptureEnabled` MUST NOT remain in the contract or persisted settings payload.

#### Scenario: Default settings

- **WHEN** a backend creates current default settings
- **THEN** it returns the existing timing and linking defaults with `minimumSessionDistance` equal to `100`, `themePreference` equal to `system`, `unitPreference` equal to `metric`, and no raw-packet field

#### Scenario: Settings round trip

- **WHEN** the frontend updates a valid appearance, units, timing, or linking field at the current revision
- **THEN** every supported backend persists and returns the same complete settings shape at a newer revision

#### Scenario: Unknown settings field

- **WHEN** a settings patch contains `rawPacketCaptureEnabled` or another field outside the complete shape
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Invalid unit preference

- **WHEN** a settings patch contains a unit preference other than `metric` or `imperial`
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Cross-platform conformance

- **WHEN** canonical conformance exercises settings on Android, iOS, or macOS
- **THEN** each target exposes the same defaults, accepted preference values, revision behavior, and invalid-field rejection

### Requirement: Explicit live sample shape

The contract SHALL define one bounded latest live sample with sequence, elapsed time, ride state, board state, footpad state, pack voltage, duty cycle, controller temperature, motor temperature, motor current, battery current, eRPM, and speed. Every field SHALL have fixed platform-neutral semantics and the contract MUST NOT expose an unbounded or catch-all live field map.

#### Scenario: Complete supported telemetry

- **WHEN** either native backend serializes a successfully persisted telemetry sample
- **THEN** it supplies the same complete named live sample shape with unavailable nullable readings represented as null

#### Scenario: New telemetry

- **WHEN** support for another live telemetry field is introduced
- **THEN** the TypeScript contract, both native backends, persistence where applicable, conformance, and frontend placement change together

### Requirement: Consistent saved-board identity

Saved-device state SHALL distinguish a nullable custom display-name override, advertised Bluetooth name, platform Bluetooth identifier, resolved display name, and mutation revision. The resolved display name SHALL use the first non-empty value from custom name, advertised name, and platform identifier in that order. Apple platform identifiers SHALL use the CoreBluetooth peripheral UUID and Android SHALL use the Bluetooth address, matching VESC Tool semantics without frontend platform branching.

#### Scenario: Custom board name

- **WHEN** a saved board has a non-empty custom name
- **THEN** saved-board and active-connection state use that value as the resolved display name

#### Scenario: Advertised-name fallback

- **WHEN** no custom name exists and the board has a non-empty advertised Bluetooth name
- **THEN** that advertised name is the resolved display name

#### Scenario: Platform-identifier fallback

- **WHEN** neither custom nor advertised name exists
- **THEN** the opaque platform Bluetooth identifier is the resolved display name

#### Scenario: Rediscovery after rename

- **WHEN** a custom-named saved board is rediscovered with an updated advertised name
- **THEN** the backend refreshes advertised identity without replacing the custom-name override

#### Scenario: Identifier disclosure boundary

- **WHEN** board state is rendered or ordinary diagnostics and exports are produced
- **THEN** the platform identifier is available as explicit read-only board metadata but is not added to ordinary logs or exports
