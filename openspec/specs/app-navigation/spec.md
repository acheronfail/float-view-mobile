# Application Navigation Specification

## Purpose

Define the routed application destinations, route-aware primary navigation, and focused connection, session, board, and settings flows.

## Requirements

### Requirement: Four primary application destinations

The frontend SHALL provide Home, Sessions, Boards, and Settings as primary destinations. Home SHALL own discovery and connection, Sessions SHALL own recorded-session browsing, Boards SHALL own saved-board configuration, and Settings SHALL own options that apply across the application.

#### Scenario: Primary navigation

- **WHEN** the user selects a primary destination
- **THEN** the frontend navigates to that destination without reloading the root document or reconnecting the backend client

### Requirement: Route-aware bottom tab bar

The frontend SHALL render a bottom tab bar on exactly the Home, Sessions, Boards, and Settings top-level routes. Its tabs SHALL map `connect` to Home, `sessions` to Sessions, `boards` to Boards, and `settings` to Settings. Each tab SHALL render a recognizable icon above its exact lowercase text label, and the current top-level destination SHALL have a visible selected state. The Devices/Sessions segmented control MUST NOT remain.

#### Scenario: Top-level tab bar

- **WHEN** the user views Home, Sessions, Boards, or Settings
- **THEN** the bottom tab bar presents the four icon-and-text destinations and identifies the current destination

#### Scenario: Select a tab

- **WHEN** the user selects `connect`, `sessions`, `boards`, or `settings`
- **THEN** the frontend navigates to the mapped top-level route without reloading the document or reconnecting the backend client

#### Scenario: Subordinate route

- **WHEN** the user views Live, session detail, board configuration, or another subordinate route
- **THEN** the bottom tab bar is absent and the focused view supplies its own return navigation

#### Scenario: Superseded segmented control

- **WHEN** the routed application shell is rendered
- **THEN** no Devices/Sessions segmented control is presented

### Requirement: Home and live connection flow

Home SHALL show Bluetooth readiness, discovery, and connection controls when no board is active. A successful connection request or a recovered active connection SHALL navigate to a live view that represents connecting, ready, reconnecting, disconnected, and failed backend states.

#### Scenario: Accepted connection

- **WHEN** the backend accepts a device connection request
- **THEN** the frontend navigates to the live view and renders subsequent connection states from authoritative snapshots

#### Scenario: Recovered active connection

- **WHEN** the frontend reconnects after lifecycle recreation and `state.get` contains an active connection
- **THEN** it replaces the inactive Home view with the live view without starting another native connection

#### Scenario: Live route without a connection

- **WHEN** the live route is opened while authoritative state has no active connection
- **THEN** the frontend returns to Home

#### Scenario: Explicit disconnect

- **WHEN** the active board is disconnected from the live view
- **THEN** the frontend returns to Home after authoritative state reports no active connection

### Requirement: Routed session browsing

Sessions SHALL list the bounded recent session summaries supplied by authoritative application state. Selecting a session SHALL navigate to a detail route that loads its bounded review series and retains the existing diagnostics, export, deletion, and group-unlinking operations.

#### Scenario: Open session

- **WHEN** the user selects a listed session
- **THEN** the frontend navigates to its detail view and requests that session by its opaque identifier

#### Scenario: Missing session

- **WHEN** a session detail route identifies a session the backend no longer has
- **THEN** the frontend returns to Sessions and presents a useful not-found message

#### Scenario: Deleted session

- **WHEN** the user confirms deletion and the backend accepts it
- **THEN** the frontend returns to Sessions and no longer renders the deleted detail

### Requirement: Routed saved-board configuration

Boards SHALL list the bounded saved boards supplied by authoritative application state. Selecting a saved board SHALL navigate to a configuration route for its versioned Board Profile, including battery series cells, battery parallel cells, and motor.

#### Scenario: Open saved board

- **WHEN** the user selects a saved board
- **THEN** the frontend navigates to its configuration view and requests its Board Profile by opaque saved-board identifier

#### Scenario: Missing saved board

- **WHEN** a board configuration route identifies a saved board the backend no longer has
- **THEN** the frontend returns to Boards and presents a useful not-found message

#### Scenario: Board profile update

- **WHEN** the user submits valid board configuration values at the current profile revision
- **THEN** the frontend updates the profile and renders the backend-returned revision and completeness

### Requirement: Global settings destination

Settings SHALL contain only application-wide appearance, session-linking, logging cadence, post-ride, and reconnect options. Board-specific values MUST NOT be edited from Settings, and application-wide settings MUST NOT remain embedded in session browsing.

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
