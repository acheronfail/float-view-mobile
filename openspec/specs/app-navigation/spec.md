# Application Navigation Specification

## Purpose

Define the routed application destinations, route-aware primary navigation, and focused connection, session, board, and settings flows.

## Requirements

### Requirement: Four primary application destinations

The frontend SHALL provide Home, Sessions, and Settings as primary destinations. Home SHALL own discovery and connection, Sessions SHALL own the saved-board index and saved-board history, and Settings SHALL own options that apply across the application. Boards SHALL NOT remain a primary destination; saved-board configuration SHALL be reachable from a saved-board history route and LiveView.

#### Scenario: Primary navigation

- **WHEN** the user selects a primary destination
- **THEN** the frontend navigates to Home, Sessions, or Settings without reloading the root document or reconnecting the backend client

#### Scenario: Saved-board subordinate route

- **WHEN** the user opens a saved board from Sessions
- **THEN** the frontend navigates to that board's history route without presenting Boards as a primary destination

### Requirement: Route-aware bottom tab bar

The frontend SHALL render a bottom tab bar on exactly the Home, Sessions, and Settings top-level routes. Its tabs SHALL map `connect` to Home, `sessions` to Sessions, and `settings` to Settings. Each tab SHALL render a recognizable icon above its exact lowercase text label, and the current top-level destination SHALL have a visible selected state. A Boards tab and the Devices/Sessions segmented control MUST NOT remain.

#### Scenario: Top-level tab bar

- **WHEN** the user views Home, Sessions, or Settings
- **THEN** the bottom tab bar presents the three icon-and-text destinations and identifies the current destination

#### Scenario: Select a tab

- **WHEN** the user selects `connect`, `sessions`, or `settings`
- **THEN** the frontend navigates to the mapped top-level route without reloading the document or reconnecting the backend client

#### Scenario: Subordinate route

- **WHEN** the user views Live, saved-board history, session detail, or board configuration
- **THEN** the bottom tab bar is absent and the focused view supplies its own return navigation

#### Scenario: Superseded Boards navigation

- **WHEN** the routed application shell is rendered
- **THEN** no Boards tab, Boards top-level route, or Devices/Sessions segmented control is presented

### Requirement: Home and live connection flow

Home SHALL show Bluetooth readiness, discovery, and connection controls when no board is active. Discovery SHALL include an advertising saved board whose exact platform identifier matches even when its advertisement omits the UART service, while unrelated advertisements remain hidden. A successful connection request or a recovered active connection SHALL navigate to a live view that represents connecting, identifying, selecting-controller, ready, reconnecting, disconnected, and failed backend states.

#### Scenario: Accepted connection

- **WHEN** the backend accepts a device connection request
- **THEN** the frontend navigates to the live view and renders subsequent identification, selection, and connection states from authoritative snapshots

#### Scenario: Recovered active connection

- **WHEN** the frontend reconnects after lifecycle recreation and `state.get` contains an active connection
- **THEN** it replaces the inactive Home view with the live view without starting another native connection

#### Scenario: Recovered controller selection

- **WHEN** the recovered active connection is awaiting a controller choice
- **THEN** the live route presents that choice from authoritative candidates without returning to Home

#### Scenario: Recovered failed connection

- **WHEN** the recovered active connection is failed with a connection issue
- **THEN** the live route retains its configuration, retry, and disconnect actions without returning to Home

#### Scenario: Live route without a connection

- **WHEN** the live route is opened while authoritative state has no active connection
- **THEN** the frontend returns to Home

#### Scenario: Explicit disconnect

- **WHEN** the active board is disconnected from the live view
- **THEN** the frontend returns to Home after authoritative state reports no active connection

### Requirement: Routed session browsing

Sessions SHALL list saved boards rather than a global session catalog. Selecting a saved board SHALL navigate to a board-history route that loads its bounded overall summary and board-filtered session pages grouped by local date. A saved board SHALL remain browsable and configurable when it has no currently discovered local Bluetooth association; connection initiation remains owned by the Home scan. Selecting a session from that route SHALL navigate to a detail view that retains metadata-first authenticated series/sample/event pages, export, deletion, and prune-state operations.

#### Scenario: Open saved-board history

- **WHEN** the user selects a saved board in Sessions
- **THEN** the frontend navigates to its board-history route and requests only sessions for that saved board

#### Scenario: Open session

- **WHEN** the user selects a listed session from a board-history date group
- **THEN** the frontend navigates to its detail view and requests that session by its opaque identifier

#### Scenario: Missing session

- **WHEN** a session detail route identifies a session the backend no longer has
- **THEN** the frontend returns to the owning board-history route when board identity is known, otherwise Sessions, and presents a useful not-found message

#### Scenario: Deleted session

- **WHEN** the user confirms deletion and the backend accepts it
- **THEN** the frontend returns to the owning board-history route when board identity is known and refreshes that board's summary and date groups

#### Scenario: No global session feed

- **WHEN** the user views Sessions
- **THEN** no `All sessions` group, global date grouping, or second session sub-tab is rendered

#### Scenario: Undiscovered saved board

- **WHEN** a saved board is absent from the current Home scan
- **THEN** its history and configuration remain available, but its board-history route does not start or offer a connection

### Requirement: Routed saved-board configuration

Saved-board configuration SHALL be reachable from the selected board's history-route menu and from the LiveView action menu. Both surfaces SHALL render the same Board Profile editor for the saved board's custom name and versioned profile, including an optional read-only local platform Bluetooth identifier, battery series cells, battery parallel cells, motor, and an optional preferred CAN ID from `0` through `253`. An imported board may have no local platform identifier and SHALL still be editable. There SHALL be no standalone Boards route required to edit a board.

#### Scenario: Open saved-board configuration

- **WHEN** the user selects Configure board from a saved board's history-route menu
- **THEN** the frontend opens the configuration sheet for that board and requests its Board Profile by opaque saved-board identifier

#### Scenario: Live configuration remains available

- **WHEN** the user selects Configure board from LiveView for an identified saved board
- **THEN** the same editor opens with the same revision-checked persistence behavior

#### Scenario: Configure board without discovery

- **WHEN** the user selects Configure board for an imported or otherwise undiscovered saved board
- **THEN** the same Board Profile editor opens without attempting Bluetooth connection, and an unavailable local platform identifier is not treated as an error

#### Scenario: Forget board from history

- **WHEN** the user opens a board-history menu for an inactive saved board
- **THEN** the menu includes `Forget board`, whose destructive confirmation states that all saved sessions for the board will also be removed and the action cannot be undone

#### Scenario: Forget board during active connection

- **WHEN** the selected board is actively connected
- **THEN** `Forget board` is unavailable and any raced backend request is rejected without deleting the board or its sessions

#### Scenario: Board profile field change

- **WHEN** a valid text, numeric, selection, or clearable preferred-CAN value commits in either configuration surface
- **THEN** the frontend immediately serializes the corresponding revision-checked update and renders the backend-returned authoritative value

#### Scenario: Multiple rapid changes

- **WHEN** multiple configuration fields commit before earlier writes finish
- **THEN** the frontend applies them in order against backend-returned revisions without losing a valid change to a self-created conflict

#### Scenario: Invalid CAN preference

- **WHEN** the user enters a fractional, negative, nonnumeric, or greater-than-253 value
- **THEN** the editor reports the validation error and does not send an invalid Board Profile update

### Requirement: Global settings destination

Settings SHALL contain only application-wide appearance, session-retention, session-linking, logging cadence, post-ride, and reconnect options. Board-specific values MUST NOT be edited from Settings, and application-wide settings MUST NOT remain embedded in session browsing.

#### Scenario: Open global settings

- **WHEN** the user selects Settings
- **THEN** the frontend renders the current authoritative global options grouped by purpose

#### Scenario: Native-owned option update

- **WHEN** the user changes a valid global option
- **THEN** the frontend sends a revision-checked settings patch and renders the backend-returned settings

### Requirement: Route failure containment

Route parameters and navigation state SHALL remain bounded and MUST NOT be used as filesystem paths or trusted backend data. A failed detail request SHALL preserve the backend connection and other authoritative state.

#### Scenario: Invalid detail identifier

- **WHEN** a detail route contains an invalid, unknown, or stale opaque identifier
- **THEN** the frontend leaves the detail view without reconnecting the backend or discarding unrelated application state

### Requirement: Current ride access

Live and Sessions SHALL offer Current ride for an active saved session. It SHALL open the shared session viewer without disconnecting or changing recording. The active viewer SHALL provide Back to Live and Recording or Reconnecting status. Guest connections SHALL NOT offer recorded current-ride review.

#### Scenario: Open and return

- **WHEN** a saved-mode rider opens Current ride and returns to Live
- **THEN** the same connection and session remain active throughout

#### Scenario: Guest connection

- **WHEN** the connection is in guest mode
- **THEN** Current ride is absent

### Requirement: Ongoing ride review

The active viewer SHALL follow the growing persisted ride by default, using bounded consistent snapshots. Inspection SHALL pause following and preserve chart selections, field visibility and map position. Jump to live SHALL resume following. Refresh SHALL stop while hidden or unmounted, allow at most one metadata refresh in flight, and reject stale responses after navigation.

#### Scenario: Inspect and resume

- **WHEN** the user pans, zooms, selects a point, or navigates or filters the map
- **THEN** automatic following pauses until Jump to live, with recording continuing independently

#### Scenario: Growing ride

- **WHEN** the visible viewer follows an active ride
- **THEN** charts, markers, events and recorded location refresh at a bounded cadence without resetting field choices

#### Scenario: Reconnect and completion

- **WHEN** a connection reconnects or its session completes
- **THEN** existing history stays visible, reconnecting is indicated, and a retained completed session becomes ordinary review with final data

#### Scenario: Refresh failure or discarded ride

- **WHEN** refresh fails or a provisional ride is discarded at close
- **THEN** the viewer preserves the last successful data, reports the problem, and provides return navigation

#### Scenario: Hidden or replaced viewer

- **WHEN** the viewer is hidden, closed or replaced by another session
- **THEN** polling stops and outstanding responses cannot replace another session's data
