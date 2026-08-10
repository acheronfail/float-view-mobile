## ADDED Requirements

### Requirement: Shared Apple FFI support conformance evidence

Shared Apple FFI support SHALL NOT be accepted until component tests prove exact valid, null, zero-length, alignment, overflow, overlap, capacity, publication, and panic behavior; every retained Apple bridge passes its existing C ABI and adversarial fixtures; all supported Apple target architectures package the existing archives and symbols; Android software remains unchanged; and `just verify` succeeds. Physical regression SHALL exercise production BLE and selected bridge artifacts on Android, iOS, and macOS against the ESP32 fixture without bypassing host routing, protocol, state, session, background, or cleanup paths.

#### Scenario: Verify the shared boundary component

- **WHEN** focused Rust tests exercise valid and adversarial regions, input borrowing, byte/scalar publication, and contained panics
- **THEN** every operation is bounded and deterministic, invalid boundaries fail before unsafe access or partial output, and diagnostics contain neither addresses nor payloads

#### Scenario: Verify every Apple bridge ABI

- **WHEN** the six focused Apple C ABI suites and architecture/symbol audits run after migration
- **THEN** every public symbol, status, record, archive, header, dependency boundary, and Swift integration remains compatible while copied primitives are absent

#### Scenario: Verify all-host physical regression

- **WHEN** focused identity, direct/CAN control, BMS, and background scenarios run as applicable on the connected Android device, connected iOS device, and macOS host
- **THEN** production behavior and cleanup pass through the existing selected host paths, proving the Apple refactor caused no observed cross-platform regression

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking
