# Spec Delta

## ADDED Requirements

### Requirement: Authoritative battery estimate projection

The authenticated snapshot SHALL expose a bounded cell-profile ID/label catalog and one shared-service battery estimate for the active connection. The estimate SHALL identify its nullable fraction, selected profile, resolved series count, sample sequence, profile revision, catalog version, calculation mode, and closed unavailable reason. Fraction SHALL be null when unavailable and finite in [0,1] otherwise. Modes SHALL be `load_compensated`, `load_clamped`, `voltage_only`, or `unavailable`; unavailable reasons SHALL be `not_configured`, `series_mismatch`, `missing_voltage`, or `not_applicable`. Apple, Android, frontend mock, and canonical conformance SHALL share these semantics. Raw `LiveSample.batteryLevel` and existing recorded/exported telemetry SHALL retain their meanings. Estimation SHALL NOT introduce BLE polls or bypass saved-session persistence before publication.

#### Scenario: Saved sample publication
- **WHEN** a saved board produces a new accepted telemetry sample
- **THEN** its sample and matching estimate appear together only after the existing persistence gate succeeds in a complete monotonic snapshot

#### Scenario: Guest or BMS connection
- **WHEN** an active connection is a guest or BMS-only device
- **THEN** its board estimate has null fraction and reason `not_applicable` without creating durable profile state

#### Scenario: Raw telemetry retained
- **WHEN** a controller reports a battery fraction differing from the calculated estimate
- **THEN** both values remain distinguishable and the recorded controller field is unchanged

### Requirement: Battery estimation profile validation

Saved profile requests and imported profiles SHALL validate optional `batteryCellProfileId` against the bounded catalog and optional `batteryMinimumCellVoltageV` against [2.7,4.2), alongside existing series/parallel validation. Updates SHALL retain existing revision conflict behavior and SHALL NOT write controller settings. Absent estimation fields SHALL NOT change existing profile-completeness policy or infer a cell identity.

#### Scenario: Invalid selection or voltage
- **WHEN** a save supplies an unknown profile ID, nonfinite minimum, or minimum outside the allowed range
- **THEN** the request fails with the existing invalid-request error and leaves saved configuration and live estimate unchanged

#### Scenario: Conflicting update
- **WHEN** a battery-profile save uses a stale revision
- **THEN** it fails with the existing conflict behavior without publishing an estimate based on the rejected values

#### Scenario: Archive round trip
- **WHEN** a valid configured board is exported and imported
- **THEN** its selected profile and empty voltage round trip through the existing profile mechanism and invalid values are rejected by the same validation rules
