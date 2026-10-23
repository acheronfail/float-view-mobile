## ADDED Requirements

### Requirement: Typed IMU timeout contract

The shared contract SHALL add `imuTimeout` to `StopCondition` and nullable boolean `LiveSample.imuTimeoutActive`. It SHALL be null on older profiles or without fresh inactive authority, and retain true until a valid clear. Fresh beta alert masks SHALL set it independently of retained stop reasons, firmware faults and fatal latch. Client, mock and both hosts SHALL agree.

#### Scenario: Historical stop differs from active error

- **WHEN** timeout has cleared but the package retains stop condition `imuTimeout`
- **THEN** the sample reports the retained stop reason with `imuTimeoutActive` false without claiming an ongoing error

#### Scenario: Unsupported field on older release

- **WHEN** a supported older package has no audited IMU-timeout reporting
- **THEN** `imuTimeoutActive` is null and existing firmware-fault and stop-condition values retain their meanings

#### Scenario: Frontend recreation and reconnect

- **WHEN** the frontend reloads or the native connection starts another generation
- **THEN** complete authoritative snapshots restore the current known state or null pending fresh evidence, without frontend-owned alert state

### Requirement: Typed timeout history events

Saved history SHALL expose `imuTimeoutHistory` events with bounded details `active`, `sourceTicks` and nullable `occurredElapsedMs`, plus existing event observation time and sequence. Timeout records SHALL NOT occupy `firmwareFaultHistory`, `lastFirmwareFault`, or the firmware-only `faultHistory` list. Existing bounded history queries and exports SHALL carry both event kinds without renaming old records.

#### Scenario: Recovered activation with no firmware fault code

- **WHEN** a beta history record reports alert ID 2, active true and code 0
- **THEN** one `imuTimeoutHistory` activation appears with source ticks and available occurrence timing while firmware-fault fields remain unchanged

#### Scenario: Recovered clear and observation order

- **WHEN** timeout activation and clear are retrieved together after the timeout ended
- **THEN** event sequence preserves received order, observation time remains retrieval time, and unknown occurrence timing remains null

#### Scenario: Existing data and bounds

- **WHEN** supported old events, baseline archives and new timeout events are queried or exported
- **THEN** all retain their original semantics within existing page, string, event-count and serialized-byte bounds

### Requirement: Bounded live timeout history

LiveSample SHALL include `imuTimeoutHistory`, a separate collection of at most 20 timeout entries containing `active`, nullable `elapsedMs` and `observedElapsedMs`. It SHALL retain recovered timeout records and observed activation/clear transitions, exclude initial healthy inactive state, and remain independent of firmware-only `faultHistory`. Both native hosts and the mock SHALL use the same field.

#### Scenario: Cleared timeout retains evidence

- **WHEN** a timeout is recovered or observed and then clears
- **THEN** subsequent live snapshots retain actual timeout history independently of active state
