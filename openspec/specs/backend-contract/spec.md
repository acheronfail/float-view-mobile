# Backend Contract Specification

## Purpose

Define the complete platform-neutral contract implemented by every supported native backend and consumed by the shared frontend.
## Requirements
### Requirement: Complete mandatory contract

Every supported backend SHALL implement `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `savedDevice.update`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.get`, `session.getDiagnostics`, `session.export`, `session.delete`, and `sessionGroup.unlink`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

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

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, Bluetooth state, discovered and saved devices with resolved identity, active connection with saved-device identity, connection state, detected direct-or-CAN controller route, bounded controller candidates when selection is required, bounded connection issue when failed, firmware, active-session summary, and latest successfully persisted sample when known, recent session summaries, session groups, and settings. Historical series, private controller UUIDs, and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all state needed to render the main application, recover controller selection, and recover the current live dashboard without platform-specific requests

#### Scenario: Bounded collections

- **WHEN** persisted history or controller discovery exceeds frontend collection limits
- **THEN** `AppState` contains only the defined bounded recent collections, bounded candidates, and one latest live sample

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while the board is reconnecting
- **THEN** initial authoritative state contains the current connection state, detected route when known, pending candidates or connection issue when applicable, and retained latest sample when one exists

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

The backend SHALL own permission resolution, bounded scanning, BLE stream setup, connection, controller identification and selection validation, reconnect, disconnection, protocol polling, and firmware discovery. Exactly zero or one BLE device MAY be active.

#### Scenario: Bounded scan

- **WHEN** the frontend requests `bluetooth.scan`
- **THEN** the backend starts discovery, stops it within the contract bound, and publishes discovered devices through state events

#### Scenario: Competing connection

- **WHEN** a second device is requested while another device is active
- **THEN** the backend returns `invalid_state` and preserves the existing connection

#### Scenario: Retry failed device

- **WHEN** `device.connect` names the same active device while its state is `failed`
- **THEN** the backend accepts a new bounded connection generation using the latest persisted Board Profile

#### Scenario: WebView recreation

- **WHEN** the frontend WebView is destroyed and recreated
- **THEN** the native connection, identification, pending controller selection, and backend jobs continue where the OS permits and the new frontend recovers them through `state.get`

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

### Requirement: Controller connection state shape

`DeviceState` SHALL include `identifying` and `selecting_controller`. `ActiveConnection` SHALL contain `controllerConnection`, which is null before a route is verified in the current generation or is exactly `{ kind: "direct" }` or `{ kind: "can", canId }`; a `controllerCandidates` array bounded to 16; and nullable `connectionIssue`. A ready connection MUST have a non-null route, supported controller and Refloat identity, an established telemetry plan, an empty candidate array, and no issue.

#### Scenario: Direct route becomes ready

- **WHEN** identification completes for a supported direct controller
- **THEN** authoritative state reports `ready`, `{ kind: "direct" }`, complete firmware identity, and no candidates

#### Scenario: CAN route becomes ready

- **WHEN** identification completes for supported controller CAN ID `42`
- **THEN** authoritative state reports `ready`, `{ kind: "can", canId: 42 }`, complete firmware identity, and no candidates

#### Scenario: Identification is incomplete

- **WHEN** the BLE link exists but controller route or compatibility negotiation is incomplete
- **THEN** state is not `ready` and no durable session or telemetry polling starts

#### Scenario: New reconnect generation

- **WHEN** reconnect begins after a verified route
- **THEN** the public route, candidates, and issue are cleared until that generation verifies or fails them while the latest persisted sample remains retained

### Requirement: Controller candidate and selection contract

Each controller candidate SHALL contain a CAN ID, bounded nullable display-safe hardware/model name and firmware name, VESC version, and `vescSupported`. Candidates SHALL be sorted by CAN ID and MUST NOT be truncated when more than 16 motor controllers exist. `device.selectController` SHALL accept the active `deviceId` and a candidate `canId` and return the standard accepted result for a valid current supported selection.

#### Scenario: Current candidate selected

- **WHEN** `device.selectController` names the active device and one of its current candidates
- **THEN** the backend accepts exactly one selection and publishes newer authoritative state as negotiation resumes

#### Scenario: Selection in the wrong state

- **WHEN** a well-formed request targets a different device, an unsupported or non-candidate ID, a stale selection, or a connection not awaiting selection
- **THEN** the backend returns `invalid_state` and leaves state and persistence unchanged

#### Scenario: Malformed selection

- **WHEN** the request omits required fields or supplies an invalid CAN ID
- **THEN** the backend returns `invalid_request` and leaves state and persistence unchanged

#### Scenario: Selection merges current profile

- **WHEN** another Board Profile field changed after the selection candidates were published
- **THEN** a valid selection atomically merges only `preferredCanId` into the latest values and advances the profile revision once without overwriting that change

### Requirement: Optional preferred CAN ID

The versioned Board Profile SHALL accept `preferredCanId` as absent or null for automatic discovery or as an integer in the inclusive range `0` through `253`. The field SHALL be optional, clearable, revision-checked, and excluded from `missingFields` and profile completeness.

#### Scenario: Valid preference update

- **WHEN** `boardProfile.update` supplies a current revision and `preferredCanId` equal to `0`, `42`, `253`, or null
- **THEN** the backend persists the exact value at a newer revision and applies it on the next gateway connection, reconnect, or same-device retry

#### Scenario: Invalid preference update

- **WHEN** an update supplies a negative, fractional, nonnumeric, or greater-than-253 CAN ID
- **THEN** the backend returns `invalid_request` and leaves the Board Profile unchanged

#### Scenario: Selection persists preference

- **WHEN** `device.selectController` accepts a discovered CAN candidate
- **THEN** the saved board's Board Profile stores that ID and advances its revision without making the optional field required for completeness

### Requirement: Stable connection issue shape

`ActiveConnection.connectionIssue` SHALL be null unless unresolved asynchronous connection work has failed. Otherwise it SHALL contain one bounded message and exactly one of `transport_setup_failed`, `identification_timeout`, `no_controller_found`, `too_many_can_devices`, `too_many_controllers`, `preferred_controller_unavailable`, `controller_changed`, `unsupported_controller`, or `unsupported_refloat`. The failed active connection SHALL remain recoverable through retry or disconnect.

#### Scenario: Asynchronous configured-route failure

- **WHEN** an accepted connection later cannot validate its preferred CAN controller
- **THEN** authoritative state becomes `failed` with `preferred_controller_unavailable`, `unsupported_controller`, or `controller_changed` rather than relying on a past request response

#### Scenario: Retry starts

- **WHEN** the frontend retries the same failed device
- **THEN** the backend clears the prior issue before publishing the new connection generation

