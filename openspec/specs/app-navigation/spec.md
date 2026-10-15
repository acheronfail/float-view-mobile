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

Home SHALL show Bluetooth readiness, discovery, and connection controls when no board is active. Discovery SHALL include an advertising saved board whose exact platform identifier matches even when its advertisement omits the UART service, while unrelated advertisements remain hidden. A successful connection request or a recovered active connection SHALL navigate to a live view that represents connecting, identifying, selecting-controller, ready, reconnecting, disconnected, and failed backend states. During frontend startup, an eligible process-lifetime UI checkpoint destination SHALL take precedence over recovered Home-to-Live navigation.

#### Scenario: Accepted connection

- **WHEN** the backend accepts a device connection request
- **THEN** the frontend navigates to the live view and renders subsequent identification, selection, and connection states from authoritative snapshots

#### Scenario: Recovered active connection

- **WHEN** the frontend reconnects after lifecycle recreation, `state.get` contains an active connection and no eligible checkpoint destination takes precedence
- **THEN** it replaces the inactive Home view with the live view without starting another native connection

#### Scenario: Restored non-live destination

- **WHEN** the frontend is recreated with an eligible Sessions, session detail, board history or Settings checkpoint while a connection remains active
- **THEN** it restores that destination without briefly navigating to Live, scanning Home or starting another native connection

#### Scenario: Recovered controller selection

- **WHEN** the recovered active connection is awaiting a controller choice and the chosen destination is Live
- **THEN** the live route presents that choice from authoritative candidates without returning to Home

#### Scenario: Recovered failed connection

- **WHEN** the recovered active connection is failed with a connection issue and the chosen destination is Live
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

Settings SHALL contain only application-wide appearance, session-retention, session-linking, logging cadence, post-ride, reconnect, and ride-announcement options. Board-specific values MUST NOT be edited from Settings, and application-wide settings MUST NOT remain embedded in session browsing.

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

The Live action menu SHALL offer Charts… and Sessions SHALL offer Current ride for an active saved session. Live SHALL NOT reserve dashboard space for a separate current-ride button. It SHALL open the shared session viewer without disconnecting or changing recording. The active viewer SHALL provide Back to Live and Recording or Reconnecting status. Guest connections SHALL NOT offer recorded current-ride review.

#### Scenario: Open and return

- **WHEN** a saved-mode rider opens Charts… from Live or Current ride from Sessions and returns to Live
- **THEN** the same connection and session remain active throughout

#### Scenario: Guest connection

- **WHEN** the connection is in guest mode
- **THEN** Charts… and Current ride are absent

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

### Requirement: Announcement settings controls

Settings SHALL provide independent speed/load switches and thresholds with explicit units, a shared repeat interval, announcement volume, audio readiness, and a test action. The same frontend SHALL support pointer and keyboard activation without duplicate actions. Validation and save/test failures SHALL preserve edits and remain visible. A test SHALL identify its selected speed, load or combined content even when both rules are off. Unavailable audio SHALL not silently disable the saved rule.

#### Scenario: Configure announcements
- **WHEN** users enable load and set 65 percent with a five-second repeat interval
- **THEN** Settings saves a revision-checked patch and presents authoritative accepted values

#### Scenario: Automatic announcement preference saving

- **WHEN** users change an announcement switch or slider, or commit a numeric input by changing or leaving it
- **THEN** valid preferences save automatically without a Save button; delayed saves preserve newer edits, and failures retain edits with an explicit retry action

#### Scenario: Invalid announcement numeric input

- **WHEN** a user commits an empty or out-of-range number, or a repeat interval that is not a whole number
- **THEN** a persistent danger-styled hint appears under that input with its valid range and current units, the field is marked invalid for assistive technology, and neither saving nor testing submits the invalid preferences
- **AND** correction clears the field's error immediately and valid committed preferences save automatically without native validation popovers

#### Scenario: Test after editing preferences

- **WHEN** users activate a test with valid uncommitted edits or a save in progress
- **THEN** the test waits for those preferences to save and does not play using older settings if saving fails

#### Scenario: Test feedback
- **WHEN** users activate a test by primary pointer or keyboard
- **THEN** exactly one test is sent, repeat activation is prevented while pending, and the terminal outcome is displayed without claiming that a finished utterance was necessarily heard

#### Scenario: Audio unavailable
- **WHEN** audio readiness is unavailable or a test cannot play
- **THEN** Settings explains the bounded reason or outcome and retains the user's announcement configuration and other working controls

### Requirement: Conditional audio lowering control

Settings SHALL show “Lower other audio while speaking” only when authoritative announcement audio state says lowering is adjustable. Helper text SHALL explain that music and other audio become quieter temporarily and that some apps may pause. Android and macOS SHALL hide this control while retaining announcement volume and other common settings. The frontend SHALL derive visibility from the mandatory state field, not detect the platform.

#### Scenario: iOS audio controls
- **WHEN** Settings receives adjustable lowering state
- **THEN** it shows the saved lowering switch and its explanation alongside announcement volume

#### Scenario: Android or macOS audio controls
- **WHEN** Settings receives automatic lowering state
- **THEN** it hides the lowering switch without hiding speech volume, thresholds, enable switches or testing

### Requirement: System voice settings action

Announcement settings SHALL show a guarded Voice settings button only when
announcementAudio.voiceSettingsAvailable is true. It SHALL open the native
chooser via the authenticated backend, preserve unsaved edits, prevent duplicate
activation while pending and display launch failures. Help text SHALL explain
that users choose an English offline voice in device text-to-speech settings.
Returning SHALL refresh the selected engine/voice without restarting the app.

#### Scenario: Guarded voice settings activation
- **WHEN** a primary pointer or keyboard activates Voice settings
- **THEN** exactly one native launch is requested and failures remain visible without discarding announcement edits
