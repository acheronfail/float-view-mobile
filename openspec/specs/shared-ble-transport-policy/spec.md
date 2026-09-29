# shared-ble-transport-policy Specification

## Purpose

Share bounded BLE discovery and write decisions across backend hosts while preserving each operating system's GATT execution and flow-control requirements.

## Requirements

### Requirement: Shared capability-driven BLE policy

All selected native backends SHALL use one shared policy for advertisement eligibility, required profile validation, write-mode preference and outbound segmentation. Native adapters SHALL provide capabilities and execute OS operations. Saved identity SHALL bypass only advertisement eligibility, never required GATT validation. With-response writes SHALL be preferred when supported; otherwise supported no-response writes SHALL remain available.

#### Scenario: Same capabilities on another host

- **WHEN** two hosts provide equivalent saved identity, advertisement and characteristic capabilities
- **THEN** they make equivalent filtering, profile and write-mode decisions without native policy replicas

#### Scenario: Platform payload constraints

- **WHEN** a host reports a 20-byte safe payload or a larger negotiated payload
- **THEN** shared segmentation respects that ceiling and the 244-byte maximum without requiring an increased MTU or overriding a platform safety restriction

### Requirement: Atomic bounded write admission

Pending and in-flight writes together SHALL occupy at most 128 chunks and 65,536 payload bytes, with at most one chunk in flight. Admission SHALL validate total sizes before copying, splitting or emitting bytes. Accepted chunks SHALL preserve frame and byte order.

#### Scenario: Exact boundary and one over

- **WHEN** a frame fits the remaining chunk/byte budget or exceeds either budget by one
- **THEN** the fitting frame is admitted and the exceeding frame is rejected without emitting any of that frame or partially growing the queue

#### Scenario: Platform backpressure

- **WHEN** acknowledgement, local submission acceptance, pacing or write readiness is outstanding
- **THEN** no subsequent chunk is emitted until the corresponding platform condition permits progress; local submission is not represented as remote acknowledgement

### Requirement: Connection-qualified completion and cleanup

Every asynchronous write/setup result SHALL be qualified by service lifetime, connection generation and operation identity. Duplicate, stale and foreign results MUST NOT advance a queue, change replacement state or replay an uncertain write. Disconnect, cancellation and failure SHALL reclaim queued bytes and retain cleanup admission under saturation.

#### Scenario: Reconnect to the same device

- **WHEN** a prior connection's completion arrives after reconnecting to the same device identifier
- **THEN** it is ignored and cannot release or transmit any replacement connection's queued work

#### Scenario: Failed or stalled write

- **WHEN** submission fails or the current write has no required progress for ten seconds
- **THEN** shared policy terminates the affected transport, releases its queue and reports one failure to existing reconnect policy without replaying the failed mutation

### Requirement: Bounded setup without duplicate reconnect policy

Shared policy SHALL bound setup by the existing ten-second connection-attempt deadline and require successful notification setup before ready. Native adapters SHALL execute setup/cancellation and report results without independent application reconnect decisions. Platform descriptor interoperability and OS flow-control behavior SHALL remain supported.

#### Scenario: Missing setup result

- **WHEN** an OS setup stage never completes or completes after the overall deadline
- **THEN** the attempt fails boundedly and the late result cannot publish ready or start telemetry

#### Scenario: Notification capability unavailable

- **WHEN** the required notification capability cannot be established
- **THEN** setup fails without ready publication, regardless of whether the device was previously saved

### Requirement: Native selection and physical evidence

Apple and Android SHALL select the shared policy through their existing binding families and remove superseded native logical policies and queues. Changes SHALL include affected deterministic boundary/fault coverage. Physical checkpoints SHALL cover only affected hosts and scenarios whose OS BLE or radio behavior cannot be established in software, following the root [verification policy](../../../AGENTS.md#verification). Software simulation MUST NOT be counted as OS BLE coverage.

#### Scenario: Audit a future host's responsibilities

- **WHEN** the selected adapters and shared state machine are inspected
- **THEN** native code retains OS objects and bounded execution state without a parallel filtering, segmentation or logical write-queue implementation

#### Scenario: Physical flow-control coverage

- **WHEN** each host runs its fixture checkpoint
- **THEN** evidence records saved/guest discovery, setup, segmented writes, telemetry, controls, reconnect and mobile background continuity; unavailable write-mode or fault coverage is explicitly identified rather than reported as passed
