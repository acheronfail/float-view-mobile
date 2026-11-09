## ADDED Requirements

### Requirement: Stable Refloat minor compatibility

Stable Refloat 1.0.x, 1.1.x, 1.2.x and 1.3.x SHALL be supported across patch versions under the maintainer's no-breaking-configuration contract. This policy supersedes exact stable patch admission restrictions. Unknown minors, majors and unaudited prereleases SHALL remain unsupported. Exact beta2/beta3 support SHALL remain. Observed versions SHALL remain exact in live and saved provenance; legacy INFO 1.0 retains its available major/minor identity.

#### Scenario: Later stable patches

- **WHEN** a supported VESC controller reports a stable patch within a supported Refloat minor on a direct or CAN route
- **THEN** it can reach ready using the minor's existing telemetry and control family
- **AND** 1.3.0 retains its pre-IMU-timeout behavior while 1.3.1 and later use IMU-timeout handling
- **AND** bounded packet validation, configuration schema validation and mutation safety remain required

#### Scenario: Unknown family or prerelease

- **WHEN** INFO reports 1.4.0, 2.0.0 or an unaudited prerelease such as 1.3.2-beta4
- **THEN** the connection fails without telemetry or mutations

### Requirement: Refloat minor support guidance

Unsupported-version guidance SHALL list stable supported minor ranges as 1.0.x, 1.1.x, 1.2.x and 1.3.x and separately retain exact supported prereleases. It SHALL explain that stable patches within those minors are supported and only listed prereleases are supported. This replaces the exact source-release list; the reported unsupported version SHALL retain its exact identity.

#### Scenario: Unsupported package explanation

- **WHEN** a board reports an unsupported Refloat version
- **THEN** its explanation presents minor ranges and exact beta2/beta3 exceptions without claiming all prereleases are unsupported
- **AND** existing explicit retry and disconnect recovery remains available

## MODIFIED Requirements

### Requirement: Actionable Refloat compatibility failures

A confirmed unsupported Refloat identity SHALL expose bounded display version metadata including its reported suffix and supported stable minor ranges and exact prerelease exceptions. The connection SHALL remain failed without automatic retries, telemetry or mutations. Retry and disconnect SHALL remain explicit actions, and a new connection generation SHALL clear prior failure metadata.

#### Scenario: Unsupported package

- **WHEN** a controller reports Refloat 1.3.1-beta
- **THEN** the current complete snapshot retains that display version and the supported minor ranges and prerelease exceptions without admitting the package or creating a session

#### Scenario: Missing or malformed INFO

- **WHEN** INFO times out or cannot be decoded safely
- **THEN** the connection reports a retryable identification failure without claiming a detected unsupported version

#### Scenario: New connection generation

- **WHEN** the user retries or disconnects after compatibility failure
- **THEN** old version metadata is cleared and only current-generation replies can establish the next outcome
