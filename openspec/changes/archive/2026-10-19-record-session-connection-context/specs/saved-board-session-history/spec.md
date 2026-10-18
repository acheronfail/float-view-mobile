# Spec Delta

## MODIFIED Requirements

### Requirement: Session archive import preserves board ownership

Session archive export SHALL include the stable opaque board identity and the bounded board snapshot required for cross-platform import, but MUST NOT include private VESC device identifiers (including controller UUIDs), Bluetooth addresses, Apple or Android platform identifiers, or private controller-route UUIDs. Import SHALL associate every session with the board represented by that identity. When a matching saved board exists, imported sessions SHALL be added to that board's history; otherwise import SHALL create a saved-board entry with the imported board snapshot and no local platform association before adding the sessions. Recorded public firmware versions, hardware/firmware names and direct/CAN routing SHALL be preserved as session provenance and SHALL NOT be used to infer board ownership or local discovery associations.

#### Scenario: Import into an existing board

- **WHEN** an archive's stable board identity matches a saved board
- **THEN** imported sessions appear in that board's date-grouped history without creating a duplicate board

#### Scenario: Import creates a board entry

- **WHEN** an archive's stable board identity has no matching saved board
- **THEN** import creates a configurable board entry and places the imported sessions in its history even though the board is not locally discoverable

#### Scenario: Import without stable identity

- **WHEN** an archive has no valid stable board identity
- **THEN** import fails without guessing a board or partially adding sessions

#### Scenario: Import does not expose raw identity

- **WHEN** a session archive is exported or imported
- **THEN** the portable archive contains the stable opaque board identity for ownership, public recorded firmware/version and direct/CAN connection context when available, and no private hardware or platform identifier

#### Scenario: Firmware context does not identify ownership

- **WHEN** two boards share firmware versions, hardware names or a CAN ID
- **THEN** import preserves those context facts but associates sessions only through the archive's stable opaque board identity
