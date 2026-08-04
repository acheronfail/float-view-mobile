## ADDED Requirements

### Requirement: Mirrored saved and guest tracking policy

Apple and Android SHALL implement one explicit native tracking-mode policy with equivalent behavior. The policy SHALL be selected before connection work begins, retained across reconnect and WebView recreation, and checked at every durable side-effect boundary including saved-device creation, Board Profile writes, controller preference or learned-hint writes, session creation/append/deletion, location recording, catalog revision changes, and exportable ride metadata. Saved mode SHALL use the existing durable owners; guest mode SHALL use bounded in-memory live state and perform none of those durable writes. Swift and Kotlin SHALL not rely on frontend omission as the isolation mechanism.

#### Scenario: Saved native connection

- **WHEN** both native hosts receive saved connection intent
- **THEN** both create or use the saved board, run the existing durable session pipeline, and expose equivalent catalog and live state behavior

#### Scenario: Guest native connection

- **WHEN** both native hosts receive guest connection intent
- **THEN** both expose live telemetry through the same state contract while creating no saved device, profile write, controller hint, session row, location row, catalog revision, or durable ride metadata

#### Scenario: Guest reconnect and recreation

- **WHEN** a guest connection reconnects or its WebView is recreated
- **THEN** both hosts retain guest mode and bounded live state without promoting any in-memory data to persistence

#### Scenario: Native policy coverage

- **WHEN** a future durable write is added to the connection or telemetry lifecycle
- **THEN** it has one explicit saved/guest policy decision and cannot be reached from guest mode accidentally

### Requirement: Native board history projection

Apple and Android SHALL expose equivalent board-scoped session pages and bounded board summary projections from the existing saved-device/session catalog. Summary calculation SHALL use existing catalog/session data or bounded queries and SHALL not add a second aggregate persistence model. Guest activity SHALL be absent from every board projection.

#### Scenario: Equivalent board history

- **WHEN** both hosts contain equivalent sessions for two saved boards
- **THEN** each returns the same board-filtered ordering, summary values, cursor behavior, and catalog revision semantics

#### Scenario: Board deletion or pruning

- **WHEN** a session is deleted or pruned
- **THEN** the owning board's next summary and history query reflect the authoritative catalog without stale guest or deleted rows

#### Scenario: Empty board history

- **WHEN** a saved board has no retained sessions
- **THEN** both hosts return a bounded empty board history and zero-or-unknown summary values according to the shared contract, without creating placeholder sessions

### Requirement: Native board identity, archive, and forget parity

Apple and Android SHALL derive the same versioned stable opaque board identity from the same meaningful VESC identity, keep raw firmware and local platform identifiers out of portable archives, and allow imported boards to exist with a nullable local platform association. Archive import SHALL merge sessions into a matching `deviceHistoryId` or create a configurable board entry before attaching them. Both hosts SHALL implement `savedDevice.delete` with the same all-owned-history cleanup and SHALL reject deletion while that board is actively connected.

#### Scenario: Cross-platform board identity

- **WHEN** equivalent VESC identity data is observed by Apple and Android
- **THEN** both hosts resolve the same stable opaque board identity without exposing the raw identifier

#### Scenario: Imported board without discovery

- **WHEN** either host imports an archive for a board with no local platform association
- **THEN** the board appears in Sessions, remains configurable, and does not become connectable until Home discovers and explicitly saves a matching board

#### Scenario: Forget parity

- **WHEN** an inactive board is forgotten on either host
- **THEN** the board, profile, private routing metadata, and all sessions owned by its history identity are removed with equivalent catalog revision behavior
