## ADDED Requirements

### Requirement: Apple Rust Refloat dynamic telemetry cutover evidence

The Apple Refloat dynamic-telemetry cutover SHALL NOT be complete until focused safe-record, compiled C-ABI, Swift-adapter, connection-lifetime, dependency, source-ownership, exact-symbol, clean-packaging, Android-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact source-derived command-32 layouts; 1.1.x and 1.2.x command-31 running, non-running, charging, active-alert, and firmware-fault forms; complete source float16 values; every field/name/aggregate/extension bound; ordered extensions; exact record bytes/extents; layout reset/replacement; malformed inputs/records; caller-region failures; contained panics; payload-safe diagnostics; and deletion of superseded Swift wire logic.

Physical Apple evidence SHALL exercise the production Rust command-31/32 decoder through real CoreBluetooth, direct/CAN routing, framing, polling, application state, sessions, controls, background execution, reconnect recovery, and cleanup. Android SHALL exercise its unchanged Kotlin decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial layout/realtime form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes exact bounded versioned records or stable failures with no unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the compiled Apple boundary

- **WHEN** compiled C and Swift tests exercise every valid and invalid ABI, region, capacity, status, record, layout-lifetime, and model-mapping path
- **THEN** the three public symbols, fixed statuses, exact records, errors, reset behavior, and publication remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean macOS/iOS packaging, Android host/four-ABI packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** both Apple hosts link the correct Rust bridge from source, superseded Swift algorithms are absent, Android artifacts and Kotlin ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Apple physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on connected iOS and macOS against the ESP32 fixture
- **THEN** both hosts complete through production Rust dynamic decoding with unchanged routing, state, session, control, background where applicable, recovery, and cleanup behavior

#### Scenario: Verify Android physical regression

- **WHEN** the same applicable scenarios run on the connected Android device
- **THEN** Android completes through its unchanged Kotlin dynamic decoder without resolving, packaging, or calling the Apple bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory
