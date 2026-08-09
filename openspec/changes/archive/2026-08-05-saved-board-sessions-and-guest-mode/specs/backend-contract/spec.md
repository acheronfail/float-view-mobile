## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, and `events`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Consistent saved-board identity

Saved-device state SHALL distinguish a nullable custom display-name override, advertised Bluetooth name, optional local platform Bluetooth identifier, stable opaque logical board identity (`deviceHistoryId`), resolved display name, and mutation revision. `savedDeviceId` remains a local record identifier; `deviceHistoryId` SHALL remain stable in session archives and SHALL be used to associate imported sessions with a board. For a meaningful local VESC endpoint firmware identity, native code SHALL derive `deviceHistoryId` with a versioned deterministic one-way digest; when no such identity exists, it SHALL use an opaque random value and SHALL not claim an automatic cross-installation match. The resolved display name SHALL use the first non-empty value from custom name, advertised name, local platform identifier, and stable logical identity in that order. Apple platform identifiers SHALL use the CoreBluetooth peripheral UUID and Android SHALL use the Bluetooth address, matching VESC Tool semantics without frontend platform branching. Imported boards MAY have no local platform identifier and SHALL remain valid saved-board records.

#### Scenario: Custom board name

- **WHEN** a saved board has a non-empty custom name
- **THEN** saved-board and active-connection state use that value as the resolved display name

#### Scenario: Advertised-name fallback

- **WHEN** no custom name exists and the board has a non-empty advertised Bluetooth name
- **THEN** that advertised name is the resolved display name

#### Scenario: Platform-identifier fallback

- **WHEN** neither custom nor advertised name exists and a local platform identifier is present
- **THEN** the opaque local platform identifier is the resolved display name

#### Scenario: Stable-identity fallback

- **WHEN** an imported board has no custom name, advertised name, or local platform identifier
- **THEN** its bounded stable logical identity is the resolved display name and the board remains configurable

#### Scenario: Rediscovery after rename

- **WHEN** a custom-named saved board is rediscovered with an updated advertised name
- **THEN** the backend refreshes advertised identity without replacing the custom-name override

#### Scenario: Archive identity merge

- **WHEN** a session archive is imported with a `deviceHistoryId` already present locally
- **THEN** the backend reuses that saved board and attaches imported sessions to it without creating a duplicate

#### Scenario: Identifier disclosure boundary

- **WHEN** board state is rendered or ordinary diagnostics and exports are produced
- **THEN** canonical VESC firmware/controller/BMS UUIDs may appear in ordinary logs and portable archives while local platform identifiers remain absent

### Requirement: Authoritative application state

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, a separate monotonic `sessionCatalogRevision`, Bluetooth state, discovered devices with an explicit saved-board match when known, saved devices with canonical VESC UUID identity when meaningful, resolved identity, and optional local platform association, active connection with `trackingMode` (`saved` or `guest`) and nullable saved-device identity, connection state, detected direct-or-CAN controller route, bounded controller candidates with canonical UUIDs when selection is required, bounded connection issue when failed, firmware UUID when meaningful, saved-mode active-session summary when present, the latest successfully persisted sample for saved mode or bounded ephemeral latest sample for guest mode when known, and settings. Recent session summaries and session groups SHALL NOT be included; standalone or board-filtered session summaries SHALL be retrieved through `session.list`. Historical series and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all bounded state needed to render the main application, including saved-board matches and tracking mode when active, with no session-summary or session-group collection

#### Scenario: Guest state

- **WHEN** a guest connection publishes a live sample
- **THEN** the authoritative snapshot may expose that bounded ephemeral sample for LiveView while active, but exposes no saved-device identity, active durable session, or catalog entry for it

#### Scenario: Catalog revision is independent

- **WHEN** a persisted saved-mode telemetry sample publishes a newer complete state snapshot without changing session catalog membership or summary visibility
- **THEN** the global state revision advances while `sessionCatalogRevision` remains unchanged

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while a saved or guest board is reconnecting
- **THEN** initial authoritative state contains the current tracking mode, connection state, detected route when known, pending candidates or connection issue when applicable, the latest permitted sample when one exists, and the current session catalog revision

### Requirement: Authoritative state events

Every frontend-visible coarse state change, every successfully persisted saved-mode active-session sample, and every bounded guest live-sample update intended for LiveView SHALL increment the state revision and publish `{ event: "state.changed", data: AppState }`. Saved-mode sample-driven events SHALL occur at the configured phase-appropriate persistence cadence; guest sample-driven events SHALL use the same bounded live update cadence without creating catalog-visible persistence. The backend SHALL conflate pending state events to the newest complete snapshot rather than allowing an unbounded queue or preserving an older pending snapshot.

#### Scenario: Connection transition

- **WHEN** an active saved or guest connection changes state
- **THEN** the frontend receives a newer complete `AppState`

#### Scenario: Persisted saved live sample

- **WHEN** the native persistence gate selects a saved-mode telemetry sample and its durable append succeeds
- **THEN** the frontend receives a newer complete state containing that sample and resulting active-session aggregates

#### Scenario: Ephemeral guest sample

- **WHEN** a guest telemetry sample is available for live display
- **THEN** the frontend receives a bounded state update containing it without a durable session append or catalog revision change

#### Scenario: Skipped probe

- **WHEN** a decoded telemetry probe is not selected for saved persistence or guest live publication
- **THEN** that probe does not cause a frontend state event

#### Scenario: Event ordering

- **WHEN** multiple changes occur while an event is pending
- **THEN** the eventual event contains the newest authoritative revision and stale revisions do not replace newer frontend state

#### Scenario: Slow frontend

- **WHEN** frontend consumption is slower than native sample production
- **THEN** pending delivery converges on the newest complete snapshot without delaying durable saved-mode persistence or growing an unbounded queue

### Requirement: Backend-owned Bluetooth operations

The backend SHALL own permission resolution, bounded scanning, saved-board matching, stable logical identity derivation, explicit saved-or-guest connection intent, BLE stream setup, connection, controller identification and selection validation, reconnect, disconnection, protocol polling, and firmware discovery. Exactly zero or one BLE device MAY be active. A connection generation SHALL retain its tracking mode for retries and lifecycle recovery. Only a Home discovery result may initiate a connection; saved-board history and configuration routes SHALL not start Bluetooth work.

#### Scenario: Bounded scan

- **WHEN** the frontend requests `bluetooth.scan`
- **THEN** the backend starts discovery, stops it within the contract bound, and publishes discovered devices with exact saved-board match information when available

#### Scenario: Competing connection

- **WHEN** a second device is requested while another device is active
- **THEN** the backend returns `invalid_state` and preserves the existing connection

#### Scenario: Known saved connection

- **WHEN** `device.connect` requests a discovered device that matches a saved board in saved mode
- **THEN** the backend accepts the saved connection and does not require a second save decision

#### Scenario: Unknown guest connection

- **WHEN** `device.connect` requests an unknown device in guest mode
- **THEN** the backend accepts the bounded connection without creating a saved-device record or reading a Board Profile as a durable write source

#### Scenario: Retry failed device

- **WHEN** `device.connect` names the same active device while its state is `failed`
- **THEN** the backend accepts a new bounded connection generation using the retained tracking mode, using the latest Board Profile only for saved mode

#### Scenario: WebView recreation

- **WHEN** the frontend WebView is destroyed and recreated
- **THEN** the native connection, tracking mode, identification, pending controller selection, and backend jobs continue where the OS permits and the new frontend recovers them through `state.get`

### Requirement: Backend-owned durable sessions

A ready saved-mode connection SHALL automatically create one provisional durable session, and all saved-mode live display, review data reads, and persistence SHALL derive from one sequenced native sample pipeline. The latest saved-mode sample SHALL become frontend-visible only after the same sample is selected and successfully appended to that active session. A ready guest connection SHALL create no durable session and SHALL use a bounded in-memory live sample path. When a saved-mode session closes or is recovered after interruption, the backend SHALL retain it only if at least one persisted sample entered the `riding` state and its accumulated distance is greater than or equal to `minimumSessionDistance`; otherwise it SHALL delete the catalog entry and session files. Guest samples, locations, summaries, and ride metadata SHALL never enter that retention decision.

#### Scenario: Ready saved board

- **WHEN** a supported saved-mode board becomes ready
- **THEN** the backend starts a provisional durable session without a frontend logging command

#### Scenario: Ready guest board

- **WHEN** a supported guest-mode board becomes ready
- **THEN** the backend starts live telemetry without creating a session, saved-device record, catalog row, or durable ride log

#### Scenario: Idle-only saved connection

- **WHEN** a provisional saved-mode session closes without any persisted sample entering `riding`
- **THEN** the backend deletes the session even if idle telemetry accumulated distance

#### Scenario: Ride below the configured minimum

- **WHEN** a provisional saved-mode session entered `riding` but closes below `minimumSessionDistance`
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

### Requirement: Bounded pageable standalone session catalog

The WebSocket control contract SHALL expose `session.list` for board-scoped session summaries outside complete `state.changed` snapshots. The request SHALL require a saved-board identifier, accept a page size from 1 through 100 and an optional opaque cursor, and order by `startedAt` descending and `sessionId` descending as a deterministic tie-break. The result SHALL contain at most the requested number of summaries, the catalog revision observed by the query, and either a bounded continuation cursor or null. Cursors SHALL be versioned and bound to the required board filter, query shape, catalog revision, and last `(startedAt, sessionId)` key; a cursor for another board, query, revision, or malformed context SHALL fail with bounded conflict or invalid-request semantics. The list SHALL contain no grouping metadata or linked-session state. An unfiltered/global session-list query and its backend callers SHALL not remain. Guest connections SHALL never create rows returned by this operation.

#### Scenario: First board-filtered page

- **WHEN** the frontend requests a valid page with a saved-device identifier
- **THEN** the backend returns only that board's newest standalone session summaries in deterministic order and no more than the requested count

#### Scenario: Missing board filter

- **WHEN** the frontend requests `session.list` without a saved-device identifier
- **THEN** the backend returns `invalid_request` and does not execute an unfiltered catalog query

#### Scenario: Continue beyond the page bound

- **WHEN** more sessions exist for the selected board after the first page
- **THEN** the result supplies a cursor and later pages reach every remaining summary for that board without offset scanning or duplication

#### Scenario: Cursor filter conflict

- **WHEN** a board-filtered cursor is reused with another board filter, query shape, or catalog revision
- **THEN** the backend rejects it with the same bounded conflict or invalid-request semantics and returns no mixed page

#### Scenario: Guest isolation

- **WHEN** a guest connection starts and ends after producing live samples
- **THEN** `session.list` returns no guest session and the catalog revision is unchanged by those samples

## ADDED Requirements

### Requirement: Irreversible saved-board deletion

The `savedDevice.delete` method SHALL accept a saved-device identifier and return `Accepted` only after the native owner has removed that saved board, its Board Profile, private routing metadata, and all session files and catalog rows owned by its `deviceHistoryId`. The operation SHALL be irreversible, SHALL increment catalog state once after cleanup, and SHALL fail with `invalid_state` when that board has an active connection. It SHALL be idempotently rejected as `not_found` after the board has already been removed.

#### Scenario: Delete inactive board

- **WHEN** the frontend requests `savedDevice.delete` for an inactive saved board
- **THEN** the backend removes the board and all of its owned sessions in one native operation and returns `{ accepted: true }`

#### Scenario: Active board deletion is rejected

- **WHEN** `savedDevice.delete` races with or follows an active connection for the same board
- **THEN** the backend returns `invalid_state` and leaves the board, profile, routing metadata, and sessions unchanged

#### Scenario: Delete updates authoritative state

- **WHEN** an inactive board deletion completes
- **THEN** the next complete state and board-scoped session requests omit the deleted board and its sessions, while other boards remain available

### Requirement: Portable session archive board ownership

The existing native session archive export/import flow SHALL carry the canonical VESC UUID as `deviceHistoryId` when meaningful, canonical controller/BMS UUID metadata where available, and the bounded saved-board snapshot needed to reconstruct board ownership. It MUST NOT carry Apple CoreBluetooth UUIDs or Android Bluetooth addresses. Import SHALL merge into the saved board with the same `deviceHistoryId`, or create that board before inserting sessions when no match exists; the resulting board SHALL have a nullable local platform association.

#### Scenario: Cross-platform archive merge

- **WHEN** an archive is imported on another supported platform and its `deviceHistoryId` already exists
- **THEN** the sessions are attached to the existing board without requiring the source platform identifier or creating a duplicate board

#### Scenario: Cross-platform archive creates board

- **WHEN** an archive is imported and its `deviceHistoryId` does not exist locally
- **THEN** the backend creates a configurable saved-board entry with no local platform association and attaches all imported sessions to it

#### Scenario: Archive without board identity

- **WHEN** an archive has no valid stable `deviceHistoryId`
- **THEN** import fails with the bounded invalid-archive error and does not guess a board or partially insert sessions

#### Scenario: Portable identity privacy

- **WHEN** a session archive is written or read
- **THEN** the stable opaque board identity is available for association but raw hardware and platform identifiers are absent from the portable data

### Requirement: Explicit connection tracking intent

The `device.connect` method SHALL accept `{ deviceId, trackingMode }`, where `trackingMode` is exactly `saved` or `guest`. The backend SHALL reject a missing or unknown mode with `invalid_request`, retain the accepted mode on the connection generation, and use it for retries, reconnect, WebView recreation, session lifecycle, location capture, controller preference writes, and learned routing hints. The frontend SHALL send `saved` only for a known local saved-board association or an explicit Save and connect choice, and SHALL send `guest` only for an explicit Connect as guest choice.

#### Scenario: Saved intent

- **WHEN** Home sends `device.connect` with `trackingMode: saved`
- **THEN** the backend follows the saved lifecycle and may establish or resolve saved-board identity only after valid VESC identification

#### Scenario: Guest intent

- **WHEN** Home sends `device.connect` with `trackingMode: guest`
- **THEN** the backend follows the guest lifecycle and never promotes the connection to saved persistence

#### Scenario: Invalid intent

- **WHEN** `device.connect` omits or changes `trackingMode` to an unsupported value
- **THEN** the backend returns `invalid_request` before starting a connection generation
