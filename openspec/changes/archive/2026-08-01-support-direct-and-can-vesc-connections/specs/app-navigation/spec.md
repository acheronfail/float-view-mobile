## MODIFIED Requirements

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

### Requirement: Routed saved-board configuration

Boards SHALL list the bounded saved boards supplied by authoritative application state using each board's resolved display name. Selecting a saved board SHALL navigate to a configuration route for its custom name and versioned Board Profile, including read-only platform Bluetooth identifier, battery series cells, battery parallel cells, motor, and an optional preferred CAN ID from `0` through `253`. The routed view and live-board modal SHALL render the same board-owned configuration editor.

#### Scenario: Open saved board

- **WHEN** the user selects a saved board
- **THEN** the frontend navigates to its configuration view and requests its Board Profile by opaque saved-board identifier

#### Scenario: Missing saved board

- **WHEN** a board configuration route identifies a saved board the backend no longer has
- **THEN** the frontend returns to Boards and presents a useful not-found message

#### Scenario: Board profile field change

- **WHEN** a valid text, numeric, selection, or clearable preferred-CAN value commits in either configuration surface
- **THEN** the frontend immediately serializes the corresponding revision-checked update and renders the backend-returned authoritative value

#### Scenario: Multiple rapid changes

- **WHEN** multiple configuration fields commit before earlier writes finish
- **THEN** the frontend applies them in order against backend-returned revisions without losing a valid change to a self-created conflict

#### Scenario: Clear custom name

- **WHEN** the user clears and blurs the custom display-name input
- **THEN** the saved-device override is cleared and both Boards and live state use the advertised name or platform identifier fallback

#### Scenario: Clear CAN preference

- **WHEN** the user clears a saved preferred CAN ID
- **THEN** the frontend persists null and explains that automatic controller discovery will be used on the next connection or reconnect

#### Scenario: Invalid CAN preference

- **WHEN** the user enters a fractional, negative, nonnumeric, or greater-than-253 value
- **THEN** the editor reports the validation error and does not send an invalid Board Profile update
