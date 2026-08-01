## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `savedDevice.update`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.get`, `session.getDiagnostics`, `session.export`, `session.delete`, and `sessionGroup.unlink`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory method is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

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

## ADDED Requirements

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
