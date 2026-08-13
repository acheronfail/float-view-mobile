## ADDED Requirements

### Requirement: Android Rust Refloat dynamic telemetry cutover evidence

The Android Refloat dynamic-telemetry cutover SHALL NOT be complete until focused safe-record, JNI-boundary, host-JVM adapter, connection-lifetime, dependency, source-ownership, exact-symbol, clean-packaging, Apple-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact source-derived command-32 layouts; 1.1.x and 1.2.x command-31 running, non-running, charging, active-alert, and firmware-fault forms; complete source float16 values; every field/name/aggregate/extension bound; ordered extensions; exact record bytes/extents; layout reset/replacement; malformed inputs/records; JVM environment, exception, reference, array, distinctness, and capacity failures; contained panics; payload-safe diagnostics; and deletion of superseded Kotlin wire logic.

Physical Android evidence SHALL exercise the production Rust command-31/32 decoder through real BLE, direct/CAN routing, framing, polling, application state, sessions, controls, background execution, reconnect recovery, and cleanup. Apple SHALL exercise its unchanged selected Rust decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial layout/realtime form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes exact bounded versioned records or stable failures with no unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the Android boundary and adapter

- **WHEN** focused JNI and host-JVM tests exercise every valid and invalid ABI, environment, exception, reference, array, capacity, status, record, layout-lifetime, and model-mapping path
- **THEN** the three public symbols, fixed statuses, exact records, errors, reset behavior, and publication remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean host and four-ABI Android packaging, macOS/iOS packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** Android loads the correct Rust bridge from source, superseded Kotlin algorithms are absent, Apple artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Android physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on the connected Android device against the ESP32 fixture
- **THEN** Android completes through production Rust dynamic decoding with unchanged routing, state, session, control, background, recovery, and cleanup behavior

#### Scenario: Verify Apple physical regression

- **WHEN** the same applicable scenarios run on connected iOS and macOS
- **THEN** both Apple hosts complete through their unchanged selected Rust decoder without resolving, packaging, or calling the Android JNI bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory
