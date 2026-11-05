## ADDED Requirements

### Requirement: Verified controllers determine saved-board ownership

Saved activation SHALL require a meaningful selected controller UUID and resolve board ownership by its derived portable identity. It SHALL reuse an exact saved/imported match or create a separate board with default configuration. A local Bluetooth association SHALL be a reconnect hint and SHALL NOT override verified identity. Reassociation SHALL atomically detach that transport from any other board, preserving existing board identities, profiles and history.

#### Scenario: Unknown controller on a familiar transport

- **WHEN** a familiar Bluetooth transport identifies a controller UUID with no saved match
- **THEN** activation creates a separate board and its future sessions use that board's identity and default configuration
- **AND** the previous board retains its identity, configuration and history with no local association

#### Scenario: Known controller on another board's transport

- **WHEN** a verified controller matches an existing or imported board while the local transport is associated with another board
- **THEN** activation reuses the exact UUID match and atomically moves the local association without merging histories or copying profiles

#### Scenario: Same controller through a new transport

- **WHEN** an existing controller is verified through a different local Bluetooth identifier
- **THEN** its board and history remain the same and its local reconnect association is refreshed

#### Scenario: Missing or invalid identification

- **WHEN** saved activation lacks a meaningful controller UUID
- **THEN** it fails without creating a board or changing an existing association, configuration or session ownership

#### Scenario: Reassociation fails

- **WHEN** board capacity or a catalog failure prevents creation or association
- **THEN** the entire reassociation rolls back and the former association and owned data remain intact

#### Scenario: Supported stored identities

- **WHEN** supported stored or imported history has an opaque identity that does not match the verified controller
- **THEN** it remains supported and intact without automatic renaming, merging or deletion
