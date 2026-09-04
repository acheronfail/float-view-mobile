## MODIFIED Requirements

### Requirement: Exact audited Refloat control profiles

The native backends and conformance simulator SHALL implement only the product control commands audited for stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0. Package enablement SHALL use source command 12 with a disabled boolean and SHALL be treated as a persistent package configuration write. Flywheel SHALL use source command 22 with Refloat's default calibration parameters and footpad abort enabled. Refloat 1.0.x and 1.1.x lighting SHALL use command 202 with its one-byte mask and flags, while Refloat 1.2.x and 1.3.0 lighting SHALL use command 20 with its four-byte mask and one-byte flags. Fatal-error clearing SHALL use command 36 and SHALL be available only for audited Refloat 1.2.x and 1.3.0. Every other Refloat mutation, tuning, recording, history, Handtest, remote, RC-move, or motor-control command MUST remain unsupported.

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

- **WHEN** any caller or fixture peer attempts a Refloat command outside the exact product-control and read allowlists, including unstable command 35
- **THEN** no unsupported command is sent or simulated, board state is unchanged, and one bounded diagnostic is recorded
