## ADDED Requirements

### Requirement: Actionable Refloat compatibility failures

A confirmed unsupported Refloat identity SHALL expose bounded display version metadata including its reported suffix and the exact supported source-release list. The connection SHALL remain failed without automatic retries, telemetry or mutations. Retry and disconnect SHALL remain explicit actions, and a new connection generation SHALL clear prior failure metadata.

#### Scenario: Unsupported package

- **WHEN** a controller reports Refloat 1.3.1-beta
- **THEN** the current complete snapshot retains that display version and the audited supported release list without admitting the package or creating a session

#### Scenario: Missing or malformed INFO

- **WHEN** INFO times out or cannot be decoded safely
- **THEN** the connection reports a retryable identification failure without claiming a detected unsupported version

#### Scenario: New connection generation

- **WHEN** the user retries or disconnects after compatibility failure
- **THEN** old version metadata is cleared and only current-generation replies can establish the next outcome

### Requirement: Refloat compatibility recovery presentation

The shared frontend SHALL show a specific unsupported-version explanation naming the board and reported package, offline supported-version guidance, secondary Check again, and primary Back to boards. The header SHALL identify unsupported Refloat and the generic diagnostics prompt SHALL be absent for this confirmed outcome. Pointer and keyboard actions SHALL activate once.

#### Scenario: Unsupported version recovery

- **WHEN** an unsupported-version snapshot arrives or the frontend is recreated
- **THEN** the dialog explains how a supported package or compatible Float View build resolves the issue and View supported versions reveals the catalog without disconnecting

#### Scenario: Check again

- **WHEN** Check again is activated
- **THEN** one explicit connection attempt starts in the current tracking mode and a repeated failure presents current details again

#### Scenario: Return to boards

- **WHEN** Back to boards, Escape, native Back, or the backdrop dismisses the dialog
- **THEN** the app disconnects and returns to the board list, preserving visible feedback if disconnect fails
