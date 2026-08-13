## ADDED Requirements

### Requirement: Android Rust Refloat compatibility cutover evidence

The Android Refloat compatibility cutover SHALL NOT be complete until focused safe-record, JNI-boundary, host-JVM adapter, dependency, source-ownership, exact-symbol, clean-packaging, Apple-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact INFO request bytes; minimal/source legacy, compact major/minor, and complete version-2 responses; legacy patch absence; every audited release/profile; unknown releases; canonical compatibility records and corruption; required layout groups; bounded future identifiers/order; every malformed payload/record/layout/environment/reference/array/capacity boundary; contained panics; payload-safe diagnostics; and deletion of superseded Kotlin algorithms.

Physical Android evidence SHALL exercise production Rust INFO/profile/layout admission through real BLE, direct/CAN routing, framing, discovery, telemetry, controls, application state, sessions, background execution, reconnect recovery, and cleanup. Apple SHALL exercise its unchanged selected compatibility owner and real CoreBluetooth path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe compatibility records

- **WHEN** focused Rust tests exercise every source-derived and adversarial INFO/profile/layout form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes the exact bounded decision or stable failure with no unsafe code, private diagnostic, or bridge-local serializer

#### Scenario: Verify the Android boundary and adapter

- **WHEN** focused JNI and host-JVM tests exercise every valid and invalid ABI, environment, exception, reference, array, capacity, status, record, request, layout, and model-mapping path
- **THEN** the exact public symbols, fixed statuses, canonical records, errors, and publication behavior remain deterministic with no partial output or fallback policy

#### Scenario: Verify software selection and isolation

- **WHEN** clean host and four-ABI Android packaging, macOS/iOS packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** Android loads the correct Rust bridge from source, superseded Kotlin algorithms are absent, Apple artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Android physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on the connected Android device against the ESP32 fixture
- **THEN** Android completes through production Rust compatibility decisions with unchanged routing, state, session, control, background, recovery, and cleanup behavior

#### Scenario: Verify Apple physical regression

- **WHEN** the same applicable scenarios run on connected iOS and macOS
- **THEN** both Apple hosts complete through their unchanged selected compatibility bridge without resolving, packaging, or calling the Android JNI bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory
