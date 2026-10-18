# Spec Delta

## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `diagnostics.list`, `diagnostics.capture`, `diagnostics.prepare`, `diagnostics.export`, `diagnostics.clear`, `diagnostics.uiError`, `diagnostics.report`, `uiState.get`, `uiState.set`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `announcements.test`, `announcements.openVoiceSettings`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.marker.add`, `session.end`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, `events`, `locations`, `markers`, and `connectionContexts`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities, except that the mandatory announcement audio state SHALL determine visibility of the iOS-only audio-lowering control and supported system voice-settings action.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Bounded request protocol

WebSocket control requests SHALL use `{ id, method, params }`; successes SHALL use `{ id, result }`; and failures SHALL use `{ id, error }`. Session-data POST bodies and results SHALL use the bounded discriminated shapes defined for `series`, `samples`, `events`, `locations`, `markers`, and `connectionContexts`, with bounded errors using the same frontend-visible error vocabulary. Every accepted WebSocket request ID and every accepted HTTP request MUST receive exactly one terminal result unless its client connection is cancelled, and IDs, messages, bodies, decoded responses, queues, timeouts, retries, cursors, and collections MUST be bounded.

#### Scenario: Successful request

- **WHEN** a valid control or session-data request is processed
- **THEN** exactly one corresponding bounded result is returned

#### Scenario: Failed request

- **WHEN** a request cannot be completed
- **THEN** exactly one structured bounded error is returned without partial success data

#### Scenario: Oversized message

- **WHEN** a client exceeds the documented WebSocket, HTTP body, decoded response, field, point, page, or cursor bound
- **THEN** the backend rejects the request or connection before unbounded allocation or storage work

## ADDED Requirements

### Requirement: Session context metadata and history reads

`session.get` SHALL include `connectionContextCount` at its published snapshot without embedding history. Every host and mock SHALL implement authenticated `connectionContexts` session-data reads with session ID, published snapshot and nullable cursor, returning at most 64 ordered context events and a nullable next cursor within 262,144 UTF-8 response bytes. Ordering SHALL use elapsed time then stable event sequence.

#### Scenario: Snapshot-scoped traversal

- **WHEN** valid pages are read for one session and published snapshot
- **THEN** every committed context through that boundary is returned once, including equal-time events, independently of telemetry transitions or sample retention

#### Scenario: Old or empty session

- **WHEN** a session contains no context
- **THEN** its metadata count is zero and the history page is empty with no next cursor

#### Scenario: Invalid authentication and cursor scope

- **WHEN** authentication is invalid, parameters are malformed, or a cursor belongs to another session or snapshot
- **THEN** existing authentication and invalid-request/conflict semantics reject the read without exposing another session's data

#### Scenario: Active snapshot refresh

- **WHEN** new context commits after a reader's published boundary
- **THEN** that reader never receives the newer event under its old boundary; superseded snapshots use existing conflict behavior and refreshed metadata supplies the new count and boundary

#### Scenario: Shared contract implementation

- **WHEN** the same context-history request is sent to Apple, Android or the mock
- **THEN** all implement the same bounded typed result and errors without frontend platform branches or exposing context history in AppState
