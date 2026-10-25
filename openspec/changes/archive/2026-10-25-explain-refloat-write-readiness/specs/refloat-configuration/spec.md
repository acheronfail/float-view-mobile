## ADDED Requirements

### Requirement: Authoritative edited-write readiness reason

Every configuration summary SHALL include nullable `writeBlockReason`, identifying the highest-priority backend condition blocking an edited configuration write. Reasons SHALL be bounded closed codes shared across hosts. Existing capability-level `writable` and supported riding Tune admission SHALL retain their meanings.

#### Scenario: Actionable blocked state

- **WHEN** schema availability, connection readiness, read-only firmware, competing work, flywheel, motion, unknown telemetry, package state, package mode, or missing current values blocks an edited write
- **THEN** the complete snapshot reports the corresponding reason without the frontend guessing from generic writable state

#### Scenario: Ready state and recovery

- **WHEN** all backend edited-write preconditions are satisfied
- **THEN** the reason is null and subsequent snapshots update it atomically as readiness changes without sending a write

#### Scenario: Riding Tune compatibility

- **WHEN** riding blocks a general configuration write on a release supporting riding Tune application
- **THEN** the edited-write reason identifies riding without changing eligibility for that Tune application

### Requirement: Visible persistent configuration write feedback

The editor SHALL show actionable write-block guidance in its fixed footer and open confirmation. Failed-write feedback SHALL remain visible in the footer after dismissal until a successful write or explicit read replaces it. Feedback SHALL preserve edits, scroll position, focus behavior, and guarded pointer/keyboard activation, without automatic mutation retries.

#### Scenario: Scrolled blocked editor

- **WHEN** writing is blocked while the user is scrolled through fields
- **THEN** the reason and disabled write action remain visible together without scrolling to the start

#### Scenario: Readiness changes inside confirmation

- **WHEN** a backend blocker appears while confirmation is open
- **THEN** the confirmation displays its actionable reason beside the disabled confirm action and remains dismissible

#### Scenario: Failed write dismissed

- **WHEN** a write cannot be confirmed and the user closes its confirmation
- **THEN** the footer retains the failure and recovery guidance without claiming that the write did not reach the board or clearing the draft

#### Scenario: Successful recovery

- **WHEN** a subsequent write succeeds or an explicit read succeeds
- **THEN** the previous failed-write feedback clears without being replayed as a mutation
