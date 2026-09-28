## ADDED Requirements

### Requirement: Complete shared service acceptance

The shared service change SHALL pass direct-host and native canonical conformance, focused storage/ABI/lifecycle fault tests, warnings-denied checks, clean Apple host/simulator/device and Android host/four-ABI packaging, strict OpenSpec validation, deletion audits and the complete `just verify` gate. Assertions and protocol simulation SHALL remain in conformance; target adapters SHALL contain only launch and OS-capability integration.

#### Scenario: Direct host exercises product behavior

- **WHEN** canonical product, archive, recording and reconnect scenarios run through the direct Rust test host
- **THEN** they use the same selected service as native production without test-host product policy, SQL or archive implementations

#### Scenario: Fault and saturation coverage

- **WHEN** storage faults occur between session commit and catalog publication, or cancellation races with saturated queues and worker shutdown
- **THEN** the suite verifies durable recovery, bounded resources, monotonic publication and no duplicate accepted mutation

#### Scenario: Delete selected native ownership

- **WHEN** each host migration completes
- **THEN** its software/package checks pass and audits reject remaining portable stores, policy, domain dispatch or snapshot reconstruction in native production

### Requirement: Shared service physical checkpoint

The change SHALL pass explicit Android, iOS and macOS physical conformance with the ESP32 fixture through the selected shared service. It SHALL cover saved/guest connection, telemetry, recording, reconnect, permitted background location, frontend/lifecycle recovery and applicable native file presentation. Unavailable hardware or coverage SHALL remain incomplete unless the user explicitly authorizes a narrower checkpoint; physical availability MUST NOT weaken the software gate.

#### Scenario: Exercise production OS adapters

- **WHEN** the flashed fixture drives each native host and mobile lifecycle transitions occur during recording
- **THEN** shared policy preserves current authenticated state and durable progress while real OS adapters execute Bluetooth, location and background work

#### Scenario: Hardware unavailable

- **WHEN** a required phone, platform or fixture cannot run its checkpoint
- **THEN** its result remains explicitly incomplete while independent software verification continues
