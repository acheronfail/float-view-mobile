## ADDED Requirements

### Requirement: Shared Android JNI support conformance evidence

Repository conformance SHALL prove that centralizing Android JNI boundary mechanics changes no public contract, JNI ABI, packaged artifact, selected protocol behavior, native application behavior, or cross-platform ownership. Evidence SHALL include exhaustive support-unit failures, all six Android bridge host/JNI suites, unchanged Apple ABI suites, dependency and raw-JNI-ownership audits, clean packaging, canonical software conformance, and the required physical fixture checkpoint.

#### Scenario: Exercise shared JNI failures

- **WHEN** support tests supply null environment/function tables, pending and newly raised exceptions, null or aliased references, invalid/excessive lengths, insufficient publication capacity, exact and zero lengths, and contained panics
- **THEN** each operation returns its bounded protocol-neutral failure without retaining VM state, allocating past the caller limit, partially publishing later results, or exposing private diagnostics

#### Scenario: Preserve every host contract

- **WHEN** focused suites invoke every established Android JNI and Apple C symbol with valid and adversarial inputs after migration
- **THEN** statuses, records, lengths, metadata, symbols, libraries, archives, dependency boundaries, and native mappings remain exact

#### Scenario: Verify all-host physical regression

- **WHEN** identity, direct/CAN control, BMS, and mobile background scenarios run as applicable on the connected Android device, connected iOS device, and macOS host against the ESP32 fixture
- **THEN** all non-location discovery, BLE, routing, protocol, telemetry, control, state, session, background, and cleanup assertions pass through the selected production paths

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run receives no location sample during its bounded automated dwell
- **THEN** only that observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, state, session, background, and cleanup failures remain blocking
