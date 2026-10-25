# UI State Restoration Specification

## Purpose

Preserve the user's destination and ordinary open surface across disposable frontend recreation within one native process, while retaining minimal background resource use.

## Requirements

### Requirement: Process-lifetime UI checkpoint

The application SHALL retain at most one UI checkpoint in the existing native process owner. It SHALL survive frontend replacement and listener stop/start, expire with that process, and use no durable or browser storage. Capture and restoration SHALL add no background timers, workers or requirement to retain a WebView.

#### Scenario: Background and return

- **WHEN** a frontend with an accepted checkpoint is destroyed on backgrounding and recreated in the same native process
- **THEN** its checkpoint remains available while native recording and lifecycle policy continue independently

#### Scenario: Process termination

- **WHEN** the native process ends and a new process starts
- **THEN** no previous UI checkpoint is available and ordinary startup navigation applies

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

### Requirement: Capture on meaningful UI changes

The frontend SHALL update its checkpoint when the supported destination, view mode or open surface changes. It SHALL deduplicate unchanged records and bound capture to one request in flight and one latest pending replacement. It SHALL not capture on telemetry updates, depend on periodic work or require a final background handshake.

#### Scenario: Open and close a panel

- **WHEN** the user opens a supported panel and later closes it
- **THEN** both choices produce checkpoint updates so an accepted close does not reopen the old panel

#### Scenario: Rapid navigation

- **WHEN** supported UI changes occur faster than checkpoint requests finish
- **THEN** queued capture retains only the latest replacement and accepted updates follow the frontend's submission order

#### Scenario: Teardown during capture

- **WHEN** background teardown interrupts an unacknowledged checkpoint request
- **THEN** return uses the last checkpoint accepted by the service without delaying teardown, replaying actions or promising the interrupted change was saved

### Requirement: Ordered restoration and target validation

Restoration SHALL run once per frontend startup after authentication and fresh authoritative state. It SHALL precede recovered-connection fallback navigation, suppress capture of initialization defaults, and open surfaces only after required data loads. Missing targets SHALL fall back to the nearest valid parent; transient failures SHALL retain the checkpoint and expose retry without repeated automatic restore loops.

#### Scenario: Initial loading

- **WHEN** the initial checkpoint and authoritative state are still loading
- **THEN** the frontend presents neutral loading feedback without mounting Home to scan or publishing a default checkpoint over the saved one; independently triggered bounded foreground discovery and auto-connect may proceed without changing restoration order

#### Scenario: Session no longer exists

- **WHEN** the restored session has been removed or discarded
- **THEN** the frontend reports that it is unavailable and returns to its known existing board history or otherwise Sessions

#### Scenario: Board no longer exists

- **WHEN** a restored board history or Board Profile target has been removed
- **THEN** the frontend falls back to Sessions or closes the Live configuration surface while preserving a valid Live route

#### Scenario: Changed active device or unavailable feature

- **WHEN** the active device differs from the saved Live target or its panel is no longer eligible
- **THEN** the frontend keeps an appropriate current connection route but skips the old device-specific presentation

#### Scenario: Load fails temporarily

- **WHEN** checkpoint retrieval or required screen data fails for an operational reason
- **THEN** the frontend offers explicit retry, retains the accepted checkpoint and does not classify the target as missing

#### Scenario: Explicit incoming destination

- **WHEN** a new frontend starts with an explicit non-root destination different from its saved route
- **THEN** it honors that navigation intent rather than replacing it with the checkpoint; root startup and reloading the saved destination remain eligible for restoration

#### Scenario: Ordinary transport reconnect

- **WHEN** the existing frontend reconnects its transport without being recreated
- **THEN** its current UI remains in place and startup restoration is not rerun

#### Scenario: Lifecycle discovery during restoration

- **WHEN** a foreground auto-connect window starts while restoration is pending
- **THEN** discovery does not require Home or replay checkpoint actions, and a resulting connection does not supersede an eligible non-live restored destination

### Requirement: Restore presentation without action replay

Restoration SHALL not replay saves, writes, Tune application, deletion, pruning, disconnection, shutdown or other pending actions. It SHALL not restore confirmation or discard dialogs. Authoritative controller-selection, issue and flywheel overlays SHALL continue to derive from current state and take precedence over restored ordinary surfaces.

#### Scenario: Unsaved editor

- **WHEN** an editor is restored after frontend recreation
- **THEN** it loads current values through its normal read path without reinstating or submitting previous unsaved input

#### Scenario: Background during confirmation or operation

- **WHEN** the frontend was showing an action confirmation or waiting for a mutating request
- **THEN** it restores only the underlying supported screen, reconciles current native state and submits no replacement mutation

#### Scenario: Flywheel active on return

- **WHEN** fresh state requires the non-dismissible flywheel overlay
- **THEN** that overlay takes precedence and restoration does not reopen a prohibited ordinary surface

### Requirement: Bounded restoration diagnostics

Debug diagnostics SHALL describe capture/restore outcomes using fixed route/surface kinds and closed reason codes. They SHALL omit checkpoint payloads, opaque target identifiers, addresses, launch tokens and telemetry, and SHALL not produce per-sample logs.

#### Scenario: Skipped target

- **WHEN** debug diagnostics are enabled and a surface cannot be restored
- **THEN** a bounded outcome identifies the surface kind and reason without printing its target or checkpoint contents

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
