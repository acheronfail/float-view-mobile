## ADDED Requirements

### Requirement: Saved and guest tracking conformance

The canonical suite SHALL exercise explicit saved and guest connection choices against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify known saved-board direct connection, unknown-board save-and-connect, unknown-board guest connection, cancellation before connection, reconnect mode retention, bounded live state, and the absence of guest durable side effects.

#### Scenario: Unknown board guest isolation

- **WHEN** conformance connects an unknown fixture board as guest, publishes live telemetry, location opportunities, and controller selection, then disconnects
- **THEN** no saved-device, Board Profile, controller hint, session, sample, location, catalog revision, exportable ride record, or Sessions entry is created

#### Scenario: Unknown board save and connect

- **WHEN** conformance connects the same fixture board with save-and-connect
- **THEN** the saved board is available, the ready state creates the normal provisional session, accepted samples and location follow saved-mode persistence rules, and the board appears in the saved-board index

#### Scenario: Guest reconnect retention

- **WHEN** a guest fixture connection fails or reconnects and the frontend is recreated
- **THEN** the active connection retains guest mode, live state recovers, and no durable state is promoted

#### Scenario: Unknown-board cancellation

- **WHEN** the frontend cancels the unknown-board choice before `device.connect`
- **THEN** no native connection generation starts and the fixture remains discoverable

### Requirement: Board-scoped session catalog conformance

The canonical suite SHALL verify the required saved-device filter on `session.list`, deterministic board-scoped keyset continuation, bounded per-board summary projection, stale filter/cursor conflicts, missing-filter rejection, local-date grouping inputs, and session-detail return metadata. It SHALL verify that global `All sessions` UI data and an unfiltered backend query are not required and that guest activity never appears in any board-scoped result.

#### Scenario: Board filter and continuation

- **WHEN** conformance creates sessions for at least two saved boards and requests successive pages for one board
- **THEN** every target returns only that board's sessions in deterministic order, continues without duplicates, and keeps the cursor bound to the board filter and catalog revision

#### Scenario: Board summary projection

- **WHEN** conformance queries a board with retained sessions, deleted sessions, and no sessions
- **THEN** each target returns equivalent bounded count, riding-time, distance, and most-recent-session values with defined unavailable handling and no maximum-speed or active/in-progress count

#### Scenario: Guest absent from history

- **WHEN** a guest fixture produces the same telemetry as a saved fixture
- **THEN** board summaries, board-scoped session pages, and catalog revisions are unchanged by guest activity

#### Scenario: Stale board cursor

- **WHEN** a board-scoped cursor is reused after catalog mutation or with another board identifier
- **THEN** every target returns the same bounded conflict and the next first-page request succeeds

### Requirement: Archive identity and board lifecycle conformance

The canonical suite SHALL verify stable opaque board identity, archive import association, configurable undiscovered boards, Home-only connection initiation, and irreversible board forgetting across the mock and native targets. It SHALL verify that portable archives omit raw VESC firmware identifiers, Apple UUIDs, Android addresses, and private controller-route UUIDs.

#### Scenario: Archive merges into existing board

- **WHEN** a session archive is imported after a saved board with the same `deviceHistoryId` already exists
- **THEN** every target attaches the imported sessions to that board without creating a duplicate

#### Scenario: Archive creates configurable board

- **WHEN** a session archive is imported for an unknown `deviceHistoryId`
- **THEN** every target creates a board entry with no local platform association, exposes it in Sessions, and makes it configurable despite zero local discovery

#### Scenario: Invalid archive identity

- **WHEN** conformance imports an archive with a missing or malformed `deviceHistoryId`
- **THEN** every target rejects the archive before creating a board or inserting a session

#### Scenario: Home owns connection initiation

- **WHEN** an imported board is absent from a Home scan
- **THEN** its history and configuration work without starting Bluetooth, and only an explicit Home save-and-connect operation can bind a local association

#### Scenario: Forget board cleanup

- **WHEN** conformance confirms Forget board for an inactive board
- **THEN** every target removes that board's profile, private routing metadata, and all owned session files/catalog rows while preserving other boards

#### Scenario: Active board cannot be forgotten

- **WHEN** conformance requests Forget board while the board is actively connected
- **THEN** every target returns `invalid_state` and preserves the board and all sessions
