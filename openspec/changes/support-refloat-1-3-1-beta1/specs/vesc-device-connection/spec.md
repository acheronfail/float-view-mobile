## MODIFIED Requirements

### Requirement: Actionable Refloat compatibility failures

A confirmed unsupported Refloat identity SHALL expose bounded display version metadata including its reported suffix and the exact supported source-release list. The connection SHALL remain failed without automatic retries, telemetry or mutations. Retry and disconnect SHALL remain explicit actions, and a new connection generation SHALL clear prior failure metadata.

#### Scenario: Unsupported package

- **WHEN** a controller reports Refloat 1.3.1-beta2
- **THEN** the current complete snapshot retains that display version and the audited supported release list without admitting the package or creating a session

#### Scenario: Missing or malformed INFO

- **WHEN** INFO times out or cannot be decoded safely
- **THEN** the connection reports a retryable identification failure without claiming a detected unsupported version

#### Scenario: New connection generation

- **WHEN** the user retries or disconnects after compatibility failure
- **THEN** old version metadata is cleared and only current-generation replies can establish the next outcome

## ADDED Requirements

### Requirement: Exact Refloat beta admission

The application SHALL admit exact INFO identity `1.3.1-beta1` alongside existing audited releases. The suffix SHALL remain part of its identity in discovery, supported-version guidance, configuration and display. Numeric `1.3.1` alone SHALL NOT authorize this beta or any other release.

#### Scenario: Audited beta on a selected route

- **WHEN** the selected direct or CAN controller reports exact `1.3.1-beta1`
- **THEN** discovery selects the audited 1.3 selectable profile without command-32 negotiation and may reach ready after the normal route and telemetry checks

#### Scenario: Neighboring identity

- **WHEN** INFO reports `1.3.1`, `1.3.1-beta`, `1.3.1-beta2`, another suffix, or malformed suffix bytes
- **THEN** the package is not admitted, starts no polling or session, and retains the existing bounded failure behavior

#### Scenario: Reconnect changes package

- **WHEN** a new connection generation reports a different package identity
- **THEN** compatibility is resolved anew and no old beta admission, telemetry, cache authority, or pending mutation authorizes the replacement

#### Scenario: Generic suffix representation remains closed to unaudited releases

- **WHEN** an INFO response carries a well-formed suffix up to 20 bytes, including an unknown beta or release-candidate suffix
- **THEN** the complete numeric-plus-suffix identity remains available for display, but only an exact audited catalogue entry authorizes connection; suffix text does not determine IMU-timeout capability
