# Backend Contract Specification

## Purpose

Define the complete platform-neutral contract implemented by every supported native backend and consumed by the shared frontend.

## Requirements

### Requirement: Complete mandatory contract
Every supported backend SHALL implement `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.disconnect`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.get`, `session.getDiagnostics`, `session.export`, `session.delete`, and `sessionGroup.unlink`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

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
- **WHEN** settings or a Board Profile is updated with a stale expected revision
- **THEN** the request fails with `conflict` and persisted data is unchanged

### Requirement: Authoritative application state
`state.get` SHALL return a bounded `AppState` containing a monotonic revision, Bluetooth state, discovered and saved devices, active connection with firmware identity when known, recent session summaries, session groups, and settings. Series and diagnostic samples MUST NOT be included.

#### Scenario: Initial state
- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all state needed to render the main application without platform-specific requests

#### Scenario: Bounded collections
- **WHEN** persisted history exceeds frontend collection limits
- **THEN** `AppState` contains only the defined bounded recent collections

### Requirement: Authoritative state events
Every frontend-visible coarse state change SHALL increment the state revision and publish `{ event: "state.changed", data: AppState }`. The backend SHALL coalesce pending state events rather than allowing an unbounded event queue.

#### Scenario: Connection transition
- **WHEN** an active connection changes state
- **THEN** the frontend receives a newer complete `AppState`

#### Scenario: Event ordering
- **WHEN** multiple changes occur while an event is pending
- **THEN** the eventual event contains the newest authoritative revision and stale revisions do not replace newer frontend state

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
A ready connection SHALL automatically create one durable session, and all live display, review, diagnostics, and persistence SHALL derive from one sequenced native sample pipeline.

#### Scenario: Ready board
- **WHEN** a supported board becomes ready
- **THEN** the backend starts a durable session without a frontend logging command

#### Scenario: Unexpected disconnect
- **WHEN** the BLE connection ends unexpectedly during a session
- **THEN** the backend closes or recovers the session with deterministic state and integrity metadata

### Requirement: Current frontend product operations
The contract SHALL support persisted settings, local versioned Board Profiles, bounded session review series, bounded lossless diagnostic windows, native export presentation, deletion, and session-group unlinking with platform-neutral semantics.

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

The mandatory `Settings` value SHALL contain `revision`, `idleProbeIntervalMs`, `idlePersistIntervalMs`, `ridingPersistIntervalMs`, `postRideTailMs`, `reconnectDurationMs`, `automaticSessionLinkingEnabled`, `sessionLinkWindowMs`, and `themePreference`. `themePreference` SHALL accept exactly `system`, `light`, or `dark`; `rawPacketCaptureEnabled` MUST NOT remain in the contract or persisted settings payload.

#### Scenario: Default settings

- **WHEN** a backend creates current default settings
- **THEN** it returns the existing timing and linking defaults with `themePreference` equal to `system` and without a raw-packet field

#### Scenario: Settings round trip

- **WHEN** the frontend updates a valid appearance, timing, or linking field at the current revision
- **THEN** every supported backend persists and returns the same complete settings shape at a newer revision

#### Scenario: Unknown settings field

- **WHEN** a settings patch contains `rawPacketCaptureEnabled` or another field outside the complete shape
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Cross-platform conformance

- **WHEN** canonical conformance exercises settings on Android, iOS, or macOS
- **THEN** each target exposes the same defaults, accepted preference values, revision behavior, and invalid-field rejection
