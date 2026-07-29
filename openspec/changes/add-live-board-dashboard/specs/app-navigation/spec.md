## MODIFIED Requirements

### Requirement: Routed saved-board configuration

Boards SHALL list the bounded saved boards supplied by authoritative application state using each board's resolved display name. Selecting a saved board SHALL navigate to a configuration route for its custom name and versioned Board Profile, including read-only platform Bluetooth identifier, battery series cells, battery parallel cells, and motor. The routed view and live-board modal SHALL render the same board-owned configuration editor.

#### Scenario: Open saved board

- **WHEN** the user selects a saved board
- **THEN** the frontend navigates to its configuration view and requests its Board Profile by opaque saved-board identifier

#### Scenario: Missing saved board

- **WHEN** a board configuration route identifies a saved board the backend no longer has
- **THEN** the frontend returns to Boards and presents a useful not-found message

#### Scenario: Board profile field change

- **WHEN** a valid text, numeric, or selection value commits in either configuration surface
- **THEN** the frontend immediately serializes the corresponding revision-checked update and renders the backend-returned authoritative value

#### Scenario: Multiple rapid changes

- **WHEN** multiple configuration fields commit before earlier writes finish
- **THEN** the frontend applies them in order against backend-returned revisions without losing a valid change to a self-created conflict

#### Scenario: Clear custom name

- **WHEN** the user clears and blurs the custom display-name input
- **THEN** the saved-device override is cleared and both Boards and live state use the advertised name or platform identifier fallback
