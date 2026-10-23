# Backend Contract Spec Delta

## ADDED Requirements

### Requirement: Bounded board dashboard contract

Board Profiles SHALL support optional nullable `customDashboard` through existing authenticated get/update operations. Version 1 SHALL contain at most 24 ordered widgets with unique bounded IDs, closed content/presentation choices and bounded sizes and optional valid numeric ranges. Encoded dashboard data SHALL fit 8,192 UTF-8 bytes and the complete profile SHALL retain its 16,384-byte limit.

#### Scenario: Flexible size compatibility

- **WHEN** a widget size is `{columns, rows}` with both integers from 1 through 4
- **THEN** the frontend, mock and shared backend accept and round-trip it through profiles and archives
- **WHEN** either dimension is missing, fractional, outside 1–4 or has unknown members
- **THEN** validation fails atomically without changing accepted values
- **WHEN** existing named compact/medium/large sizes are read or imported
- **THEN** they remain valid and unchanged, retaining their recognised footprints without a reset or implicit write

#### Scenario: Valid dashboard

- **WHEN** an authenticated client submits a supported dashboard at the current board-profile revision
- **THEN** the complete configuration is returned and persisted atomically with a newer profile revision on Apple, Android and the mock

#### Scenario: Invalid or oversized dashboard

- **WHEN** a dashboard contains an unknown version or member, duplicate widget IDs, an unsupported content/presentation pair, invalid ranges, more than 24 widgets or exceeds either byte limit
- **THEN** the request fails with the existing invalid-request semantics without changing profile contents or revisions

#### Scenario: Absent, cleared and empty dashboard

- **WHEN** `customDashboard` is absent or is cleared with null
- **THEN** the board uses its derived default dashboard
- **WHEN** a version-1 dashboard explicitly contains an empty widget list
- **THEN** that empty dashboard is preserved as a user choice

### Requirement: Revision-safe dashboard persistence

Dashboard updates SHALL use existing Board Profile conflict and error semantics and preserve unrelated profile fields. Reads and writes SHALL remain owned by the shared service. Failed persistence, stale revisions and stale board targets SHALL NOT cause partial writes, cross-board updates or compatibility resets. Frontend recreation SHALL reload accepted data without replaying a save.

#### Scenario: Concurrent profile edit

- **WHEN** a dashboard save uses an outdated profile revision after another editor changes battery configuration
- **THEN** it returns conflict without overwriting either accepted battery values or the stored dashboard

#### Scenario: Save failure or frontend teardown

- **WHEN** saving fails or the frontend disappears before acknowledgement
- **THEN** the service preserves transactional profile semantics and the next frontend reads current values rather than automatically resubmitting the old mutation

#### Scenario: Removed board

- **WHEN** a dashboard request refers to a board that no longer exists
- **THEN** it returns the existing not-found result without creating a replacement board

### Requirement: Additive dashboard compatibility

Adding dashboard data SHALL preserve supported storage, existing Live View slots, session snapshots and baseline archive imports. Missing dashboard data SHALL remain valid. Archive profile snapshots containing valid dashboards SHALL retain them through normal round trips; import SHALL follow existing board-merge policy rather than overwrite an existing board's current dashboard.

#### Scenario: Supported existing data

- **WHEN** a supported store or frozen baseline archive without a dashboard is opened or imported
- **THEN** its boards, settings and sessions remain readable and Custom View uses defaults without resetting storage or rewriting historical profiles

#### Scenario: Archive creates or merges board

- **WHEN** an archive profile snapshot contains a valid dashboard
- **THEN** it is preserved in the imported session snapshot and initializes a newly created board through existing import policy, while an already-existing board keeps its current profile

#### Scenario: Stored dashboard is unreadable

- **WHEN** existing dashboard data has an unsupported version or is malformed
- **THEN** loading fails actionably without replacing it with defaults, deleting owned data or treating corruption as authorization for a reset

### Requirement: Durable per-board live view selection

Saved boards SHALL persist optional nullable `lastLiveView` (`normal`, `detailed`, `custom`) through revisioned profile updates preserving unrelated fields. Missing/null SHALL default to Live. Entry SHALL restore this choice without writing, with eligible process checkpoints taking precedence. Guests SHALL remain temporary. Stale reads or completions SHALL NOT change another board's view.

#### Scenario: Restart or reconnect to a saved board

- **WHEN** a user selects Custom for one board and Live for another, then reconnects or restarts without a process checkpoint
- **THEN** each board opens its own saved view using current accepted dashboard data

#### Scenario: Existing profile or guest

- **WHEN** a saved board has no view preference or a guest connection starts
- **THEN** Live is used without creating a preference merely by entering

#### Scenario: Concurrent configuration or failed preference save

- **WHEN** selection races with a profile change
- **THEN** only the selected-view field may be retried once against a fresh revision, without overwriting dashboard or other fields
- **WHEN** persistence fails
- **THEN** the prior view remains selected and the user receives feedback permitting retry
