# UI State Restoration Spec Delta

## MODIFIED Requirements

### Requirement: Minimal supported presentation

Restoration SHALL cover the route and its session/board target, Live normal/detailed/custom mode and at most one allowlisted ordinary open surface. Surfaces SHALL include Live actions, Live charts, attached BMS details, Refloat configuration, Tunes, full or battery-only Board Profile configuration, and board-history actions/configuration. Unsupported transient UI SHALL reopen in its default state. Custom mode SHALL require a matching eligible saved board.

#### Scenario: Session browsing during a connection

- **WHEN** the user backgrounds from a session detail screen while a board remains connected
- **THEN** return restores that session detail destination and its known board return context rather than replacing it with Live

#### Scenario: Live presentation

- **WHEN** the user backgrounds from detailed Live with an eligible ordinary surface open
- **THEN** return restores detailed Live and that surface after current data makes it eligible

#### Scenario: Board configuration

- **WHEN** the user backgrounds with Board Profile configuration open from Live or board history
- **THEN** return reopens the same configuration surface for the same saved board using current authoritative values

#### Scenario: Excluded presentation details

- **WHEN** a frontend is recreated
- **THEN** chart/map position, selections, field visibility, follow-live mode, scroll/focus offsets, live-chart samples and unsaved editor inputs are initialized normally rather than restored

#### Scenario: Custom presentation

- **WHEN** the frontend is recreated in the same process with an accepted Custom View checkpoint for the still-active saved board
- **THEN** Custom View loads the board's current accepted dashboard in ordinary viewing mode with empty live-chart history and without an editing or widget-configuration surface

#### Scenario: Custom target no longer eligible

- **WHEN** a custom checkpoint targets a different board or the current connection is a guest
- **THEN** the current connection's ordinary eligible view is used without applying the old board's layout or draft

## ADDED Requirements

### Requirement: Dashboard drafts stay outside checkpoints

UI checkpoints SHALL NOT contain dashboard layouts, widget editor inputs, drag state or save intent. Saved dashboard data SHALL remain in Board Profiles. Restoration SHALL NOT replay dashboard writes or control actions, and ordinary connection recovery SHALL NOT discard an existing local draft for the same board.

#### Scenario: Teardown during editing or saving

- **WHEN** the frontend is destroyed with an unsaved dashboard draft or an unacknowledged save
- **THEN** recreation loads current accepted profile values without restoring the draft or resubmitting the request

#### Scenario: Same-board transport recovery

- **WHEN** an existing frontend reconnects to the same board without being recreated
- **THEN** its draft survives locally and fresh state reconciles readings without replaying edits or lighting actions

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
