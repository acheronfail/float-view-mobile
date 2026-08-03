## ADDED Requirements

### Requirement: Location setting and permission state

The authenticated contract SHALL add `locationRecordingEnabled` and `includeGpsLocationWhenExporting` to the complete persisted settings shape and SHALL expose bounded location authorization/status and action availability through authoritative state. Both settings SHALL default to `false`, and the state shape SHALL not contain location coordinates.

#### Scenario: Settings round trip

- **WHEN** the frontend updates `locationRecordingEnabled` at the current settings revision
- **THEN** the backend persists and returns the boolean at a newer revision on every supported target

#### Scenario: State status

- **WHEN** location authorization or recording eligibility changes
- **THEN** the backend publishes a newer complete state containing bounded status/action information without a location track

### Requirement: Location permission actions

The mandatory contract SHALL provide platform-neutral requests to ask for location permission and to open platform settings when the OS requires settings navigation. These methods SHALL use the existing stable error envelopes and SHALL be safe to invoke repeatedly.

#### Scenario: Request permission

- **WHEN** the frontend invokes the location permission action
- **THEN** the backend requests the next supported OS permission and returns the resulting bounded status or `permission_required`/`unavailable` error

#### Scenario: Open settings

- **WHEN** the user invokes the settings action after a non-repromptable denial
- **THEN** the native host presents the platform settings destination or returns a bounded `unavailable` error without altering board connection state

### Requirement: Published location boundary

Session metadata and the published session snapshot SHALL include bounded location availability/count and the last durable location sequence/elapsed boundary. A successful location append SHALL advance the published boundary only after the corresponding SQLite write is durable.

#### Scenario: Durable append

- **WHEN** a native location batch commits successfully
- **THEN** the next session metadata and state revision expose the new location boundary

#### Scenario: Partial write

- **WHEN** a location batch fails before commit
- **THEN** the published boundary remains at the previous durable row and a bounded internal error is reported without coordinates

### Requirement: Authenticated locations session-data method

The session-data contract SHALL add a bounded `locations` kind with cursor, limit, time-range, and published-snapshot validation semantics matching existing session-data kinds. Results SHALL return location points and explicit gaps without embedding them in `AppState` or `session.get` history payloads.

#### Scenario: Valid query

- **WHEN** the frontend sends a correctly authenticated bounded `locations` request for the current published snapshot
- **THEN** the backend returns one bounded page with a continuation cursor when applicable

#### Scenario: Invalid query

- **WHEN** the request exceeds the documented limit, names an unknown session, or uses a stale snapshot
- **THEN** the backend returns a stable bounded error and allocates no unbounded result
