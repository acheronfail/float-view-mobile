## ADDED Requirements

### Requirement: Custom-configuration identity and routing

For audited controller firmware, native identity decoding SHALL retain a bounded non-public custom-configuration count and nullable hardware-configuration CRC when present without exposing either as a compatibility shortcut or public raw identity. The selected supported Refloat controller SHALL require custom configuration index `0` before configuration becomes available. Commands 92–95 SHALL be encoded and decoded by matching Swift/Kotlin VESC protocol owners, routed directly or wrapped exactly once through the selected CAN controller, and submitted only through the connection-wide request scheduler with expected command, route, chunk, operation, and connection-generation context.

#### Scenario: Direct custom configuration
- **WHEN** the selected direct controller reports at least one custom configuration
- **THEN** schema, current/default, and eligible write requests use index `0` without CAN wrapping

#### Scenario: CAN custom configuration
- **WHEN** the selected controller is at CAN ID 42
- **THEN** each command 92–95 request is wrapped exactly once for ID 42 and only its correlated response advances the operation

#### Scenario: No registered configuration
- **WHEN** a supported Refloat identity reports zero custom configurations or index `0` never answers
- **THEN** live telemetry may remain ready while Refloat configuration is unavailable and no guessed schema is used

#### Scenario: Stale chunk response
- **WHEN** a command-92 response belongs to an older route, offset, operation, or connection generation
- **THEN** the scheduler discards it without appending bytes or updating replacement configuration state

### Requirement: Centralized Refloat mutation authorization

The connection service SHALL provide one native authorization and exclusivity boundary for semantic Refloat controls and complete custom-configuration writes. It SHALL evaluate current access mode, exact package profile, route, generation, package/ride/footpad state, and pending work immediately before scheduler submission. Test-version firmware and unsupported package versions SHALL permit eligible reads but no Refloat mutations.

#### Scenario: Competing semantic control
- **WHEN** a configuration write is verifying readback and a package, flywheel, lighting, or fatal control is requested
- **THEN** the later mutation is rejected without a second pending write

#### Scenario: Competing configuration write
- **WHEN** a semantic Refloat control is pending and a configuration write or Tune apply is requested
- **THEN** the configuration mutation is rejected without changing its current base revision

#### Scenario: Read-only test firmware
- **WHEN** audited test-version firmware opens Refloat configuration
- **THEN** schema and current/default reads may succeed but write and Tune apply remain unavailable

