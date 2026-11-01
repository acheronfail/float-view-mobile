## ADDED Requirements

### Requirement: Limited support excludes device mutations

A limited Refloat connection SHALL expose controls as unavailable/non-writable and SHALL reject every device mutation through authoritative backend admission and immediate submission checks. Matching telemetry or configuration SHALL NOT grant control semantics. Existing audited control profiles and their safety rules SHALL remain unchanged.

#### Scenario: All mutation entry points are denied

- **WHEN** any caller makes a well-formed request for configuration write, Tune apply, package enable/disable, flywheel enable/disable, lighting, fatal clear, remote/motor control, firmware recording or controller shutdown on a limited connection
- **THEN** the backend rejects the request with `invalid_state` and emits no device mutation frame
- **AND** no optimistic state, pending mutation or simulated success is published

#### Scenario: No unverified safety escape

- **WHEN** a limited connection reports active flywheel mode or the caller requests a lighting query encoded through command 20
- **THEN** neither flywheel disable nor the zero-mask lighting query is sent; audited escape/query rules cannot authorize an unverified command family

#### Scenario: Queued or stale request

- **WHEN** a request was prepared with verified capability but the generation or capability changes before submission
- **THEN** the authoritative backend rechecks permission before encoding/scheduler submission and sends no prohibited request to the limited or replacement connection

#### Scenario: Audited behavior remains intact

- **WHEN** an audited beta2/beta3 or other existing verified profile is active
- **THEN** its exact controls, timeout/freshness checks, flywheel-disable escape and mandatory configuration readback retain their existing semantics independently of the limited path
