## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.marker.add`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, `events`, `locations`, and `markers`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Bounded request protocol

WebSocket control requests SHALL use `{ id, method, params }`; successes SHALL use `{ id, result }`; and failures SHALL use `{ id, error }`. Session-data POST bodies and results SHALL use the bounded discriminated shapes defined for `series`, `samples`, `events`, `locations`, and `markers`, with bounded errors using the same frontend-visible error vocabulary. Every accepted WebSocket request ID and every accepted HTTP request MUST receive exactly one terminal result unless its client connection is cancelled, and IDs, messages, bodies, decoded responses, queues, timeouts, retries, cursors, and collections MUST be bounded.

#### Scenario: Successful request

- **WHEN** a valid control or session-data request is processed
- **THEN** exactly one corresponding bounded result is returned

#### Scenario: Failed request

- **WHEN** a request cannot be completed
- **THEN** exactly one structured bounded error is returned without partial success data

#### Scenario: Oversized message

- **WHEN** a client exceeds the documented WebSocket, HTTP body, decoded response, field, point, page, or cursor bound
- **THEN** the backend rejects the request or connection before unbounded allocation or storage work

### Requirement: Backend-owned durable sessions

A ready saved-mode connection SHALL automatically create one provisional durable session, and all saved-mode live display, review data reads, and persistence SHALL derive from one sequenced native sample pipeline. The latest saved-mode sample SHALL become frontend-visible only after the same sample is selected and successfully appended to that active session. A ready guest connection SHALL create no durable session and SHALL use a bounded in-memory live sample path. When a saved-mode session closes or is recovered after interruption, the backend SHALL retain it if it contains a committed ride marker, or if at least one persisted sample entered the `riding` state and its accumulated distance is greater than or equal to `minimumSessionDistance`; otherwise it SHALL delete the catalog entry and session files. Guest samples, locations, summaries, and ride metadata SHALL never enter that retention decision.

#### Scenario: Ready saved board

- **WHEN** a supported saved-mode board becomes ready
- **THEN** the backend starts a provisional durable session without a frontend logging command

#### Scenario: Ready guest board

- **WHEN** a supported guest-mode board becomes ready
- **THEN** the backend starts live telemetry without creating a session, saved-device record, catalog row, or durable ride log

#### Scenario: Idle-only saved connection

- **WHEN** a provisional saved-mode session closes without a committed ride marker or any persisted sample entering `riding`
- **THEN** the backend deletes the session even if idle telemetry accumulated distance

#### Scenario: Ride below the configured minimum

- **WHEN** a provisional saved-mode session has no committed ride marker and entered `riding` but closes below `minimumSessionDistance`
- **THEN** the backend deletes the session instead of exposing it in session history

#### Scenario: Live sample ordering

- **WHEN** a saved-mode telemetry sample is selected for persistence
- **THEN** its append completes before the state revision exposes that sample and updated session summary

#### Scenario: Guest sample ordering

- **WHEN** a guest telemetry sample is selected for live publication
- **THEN** the state update exposes only the bounded ephemeral sample and no durable append, location write, catalog revision, or session summary is produced

#### Scenario: Unexpected disconnect

- **WHEN** a saved or guest BLE connection ends unexpectedly
- **THEN** saved mode retains the last persisted live sample while reconnecting and closes or recovers the session deterministically, while guest mode retains only bounded in-memory live state and reconnects without creating history

#### Scenario: Active connection removed

- **WHEN** the backend removes or replaces the active connection
- **THEN** its retained live sample, tracking mode, and active-session association are cleared; no guest state is promoted to saved history

#### Scenario: Explicitly marked session

- **WHEN** a saved-mode session containing a committed ride marker closes or is recovered without riding or below the configured minimum distance
- **THEN** the backend retains the session and its marker for review; explicit user deletion remains available
