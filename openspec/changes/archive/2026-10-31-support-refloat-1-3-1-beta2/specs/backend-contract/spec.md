# Spec Delta

## ADDED Requirements

### Requirement: Typed Refloat IMU timeout contract

The shared contract SHALL expose nullable active IMU-timeout state, `imuTimeout` stop condition, and at most 20 recent timeout activation or clearing records in a live sample. Saved-session event pages SHALL expose typed recovered timeout details with nullable approximate occurrence time. Timeout data MUST remain distinct from firmware-fault code, fatal latch, and generic alert reason.

#### Scenario: Active and cleared live state

- **WHEN** a current-generation beta2 alert response first confirms timeout active and later confirms it cleared
- **THEN** Apple, Android, and the mock publish the same complete monotonic snapshots with true then false state and bounded distinct history records

#### Scenario: Unavailable state

- **WHEN** the timeout state has not been observed or previously inactive evidence becomes stale
- **THEN** the field is null rather than a fabricated inactive or cleared value

#### Scenario: Historical timeout event

- **WHEN** a saved ride contains a recovered timeout activation or clearing record
- **THEN** authenticated event pages and exports retain its kind, active value, source ticks, observation time, and nullable occurrence time without assigning a firmware-fault code

#### Scenario: Older supported data

- **WHEN** a supported pre-beta session, archive, or live fixture has no timeout events
- **THEN** existing firmware-fault fields and event reads remain valid without a compatibility reset

### Requirement: Authoritative IMU-timeout write blocker

Configuration summaries SHALL report distinct bounded reasons for active IMU timeout and unavailable or stale inactive confirmation on timeout-capable releases. Both hosts and the mock MUST use the same codes; frontend guidance SHALL not infer write eligibility from a stale live sample.

#### Scenario: Active or unconfirmed timeout

- **WHEN** a beta2 board has an active timeout or lacks fresh confirmed inactive alert state
- **THEN** the configuration summary reports the corresponding reason and guarded writes remain unavailable before request submission
