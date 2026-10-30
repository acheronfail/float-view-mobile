# Spec Delta

## MODIFIED Requirements

### Requirement: Exact audited Refloat control profiles

The native backends and conformance simulator SHALL implement only the product control commands audited for stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0 plus exact 1.3.1-beta2. Package enablement SHALL use source command 12 with a disabled boolean and SHALL be treated as a persistent package configuration write. Flywheel SHALL use source command 22 with Refloat's default calibration parameters and footpad abort enabled. Refloat 1.0.x and 1.1.x lighting SHALL use command 202 with its one-byte mask and flags, while Refloat 1.2.x, 1.3.0, and 1.3.1-beta2 lighting SHALL use command 20 with its four-byte mask and one-byte flags. Fatal-error clearing SHALL use command 36 and SHALL be available only for audited Refloat 1.2.x, 1.3.0, and 1.3.1-beta2. Every other Refloat mutation, including runtime tuning, data recording, Handtest, remote, RC-move, or motor-control commands, MUST remain unsupported; the existing bounded command-35 alert read remains available.

#### Scenario: Legacy lighting control

- **WHEN** LEDs or headlights are read or changed on an audited Refloat 1.0.x or 1.1.x package
- **THEN** the backend sends route-correct command 202 with the exact legacy mask shape and accepts only its bounded lighting-state response

#### Scenario: Stable lighting control

- **WHEN** LEDs or headlights are read or changed on an audited Refloat 1.2.x or 1.3.0 package
- **THEN** the backend sends route-correct command 20 with the exact stable mask shape and accepts only its bounded lighting-state response

#### Scenario: Package enablement

- **WHEN** an eligible user requests the opposite Refloat package enabled state
- **THEN** the backend sends command 12 with the corresponding disabled value and completes only after later release-selected telemetry confirms the requested package state

#### Scenario: Flywheel transition

- **WHEN** an eligible user requests flywheel on or off
- **THEN** the backend sends command 22 with the audited default payload and completes only after later release-selected telemetry confirms the requested package mode

#### Scenario: Version-limited fatal clearing

- **WHEN** an audited Refloat 1.2.x or 1.3.0 package has a clearable fatal latch and no active firmware fault
- **THEN** the backend sends command 36 value 1 and completes only after later telemetry reports that the latch is clear

#### Scenario: Audited beta2 controls

- **WHEN** exact 1.3.1-beta2 is ready on a direct or CAN route
- **THEN** the existing stable package, flywheel, lighting, and fatal-clear profiles apply without a beta-specific command implementation

#### Scenario: Excluded mutation

- **WHEN** any caller or fixture peer attempts a Refloat command outside the exact product-control and read allowlists, excluding the existing bounded command-35 alert read
- **THEN** no unsupported command is sent or simulated, board state is unchanged, and one bounded diagnostic is recorded

## ADDED Requirements

### Requirement: IMU-timeout mutation safety

For an audited release with IMU-timeout alerts, the backend SHALL require a fresh, confirmed inactive timeout state before package state changes, flywheel enable, edited configuration writes, or Tune application. Active or unknown state SHALL block those submissions. Timeout alone MUST NOT create a fatal latch or authorize fatal clear. Existing flywheel-disable and lighting rules SHALL remain in force.

#### Scenario: Active timeout

- **WHEN** command 35 reports an active IMU timeout and a guarded operation is requested
- **THEN** the backend rejects it before board traffic without treating the timeout as a VESC firmware fault

#### Scenario: Unknown or stale inactive state

- **WHEN** the first command-35 observation is unavailable, a read fails, or previously inactive evidence expires
- **THEN** guarded operations remain unavailable even if fast telemetry continues

#### Scenario: Timeout authority changes while a write is queued

- **WHEN** a guarded operation waits in the scheduler and an intervening alert read reports an active timeout, fails, or leaves inactive evidence expired
- **THEN** the backend cancels the operation before transmitting it, releases its pending state, and preserves eligible flywheel-disable and lighting actions

#### Scenario: Recovery and escape

- **WHEN** current-generation command 35 confirms timeout inactive
- **THEN** ordinary safety and access conditions determine guarded-write eligibility; an eligible flywheel-disable action remains possible during timeout
