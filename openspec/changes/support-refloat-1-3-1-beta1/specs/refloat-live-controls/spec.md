## MODIFIED Requirements

### Requirement: Exact audited Refloat control profiles

The native backends and conformance simulator SHALL implement only the product control commands audited for stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0 plus exact 1.3.1-beta1. Package enablement SHALL use source command 12 with a disabled boolean and SHALL be treated as a persistent package configuration write. Flywheel SHALL use source command 22 with Refloat's default calibration parameters and footpad abort enabled. Refloat 1.0.x and 1.1.x lighting SHALL use command 202 with its one-byte mask and flags, while Refloat 1.2.x and 1.3.0 plus exact 1.3.1-beta1 lighting SHALL use command 20 with its four-byte mask and one-byte flags. Fatal-error clearing SHALL use command 36 and SHALL be available only for audited Refloat 1.2.x and 1.3.0 plus exact 1.3.1-beta1. Every other Refloat mutation, tuning, recording, Handtest, remote, RC-move, or motor-control command MUST remain unsupported. Read-only command 35 for the audited beta SHALL be governed solely by the telemetry read profile.

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

#### Scenario: Excluded mutation

- **WHEN** any caller or fixture peer attempts a Refloat command outside the exact product-control and read allowlists, including command 35 outside an audited read profile
- **THEN** no unsupported command is sent or simulated, board state is unchanged, and one bounded diagnostic is recorded

## ADDED Requirements

### Requirement: IMU timeout mutation safety

An active beta IMU timeout SHALL make package enablement changes, flywheel enablement, persistent configuration writes and Tune application unavailable before submission. The existing guarded flywheel-disable escape SHALL remain available. Timeout SHALL NOT create a fatal latch or enable Clear fatal error by itself; clearing a separate firmware-fault latch SHALL retain its existing gates.

#### Scenario: Timeout during proposed mutation

- **WHEN** current authoritative beta state reports active IMU timeout and an affected write is requested
- **THEN** the backend rejects it before board traffic even if ride state is idle

#### Scenario: Timeout is the only error

- **WHEN** IMU timeout is active without an independent fatal latch
- **THEN** Clear fatal error is unavailable and the app does not send command 36 as a timeout remedy

#### Scenario: Timeout clears

- **WHEN** fresh current-generation evidence confirms the timeout cleared
- **THEN** affected writes become eligible only when all ordinary safety and fresh-current-base requirements also pass

#### Scenario: Flywheel escape

- **WHEN** flywheel is active during an IMU timeout
- **THEN** the existing guarded disable transition remains available without enabling other writes

### Requirement: Unknown beta timeout state cannot authorize guarded writes

On the beta, package changes, flywheel enable, configuration writes and Tune application SHALL require confirmed inactive timeout state. Unknown state before the first valid history response SHALL fail without mutation traffic. Profiles without IMU timeout support SHALL retain existing eligibility, and guarded flywheel disable SHALL remain available.

#### Scenario: First history response unavailable

- **WHEN** beta telemetry is available but timeout state remains unknown
- **THEN** guarded mutations fail without emitting a write

#### Scenario: Inactive alert authority expires or fails

- **WHEN** a previously inactive command-35 observation exceeds the configured idle/history cadence plus two request timeout budgets, or a current pending history read fails
- **THEN** guarded writes fail before traffic, including before the next fast sample; fast telemetry cannot renew inactive authority, and a fresh valid inactive mask restores ordinary eligibility
