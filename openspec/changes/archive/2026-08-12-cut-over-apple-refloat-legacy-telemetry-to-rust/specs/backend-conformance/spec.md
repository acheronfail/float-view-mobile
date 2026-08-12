## ADDED Requirements

### Requirement: Apple Rust Refloat legacy telemetry cutover evidence

The Apple Refloat legacy-telemetry cutover SHALL NOT be complete until focused safe-record, compiled C-ABI, Swift-adapter, dependency, source-ownership, exact-symbol, clean-packaging, Android-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover detail levels 1 through 4, active known and unknown firmware faults, every enum/flag and optional tail, field-weakening sentinel absence, numeric boundaries, exact record bytes and extents, malformed inputs and records, caller-region failures, contained panics, payload-safe diagnostics, and deletion of superseded Swift wire logic.

Physical Apple evidence SHALL exercise the production Rust command-10 decoder through real CoreBluetooth, direct/CAN routing, framing, polling, application state, sessions, controls, and cleanup. Android SHALL exercise its unchanged Kotlin decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell MAY be deferred to manual coverage but MUST NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records
- **WHEN** focused Rust tests decode every source-derived and adversarial command-10 form
- **THEN** the safe component alone produces the exact bounded versioned record or stable failure with no allocation, unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the compiled Apple boundary
- **WHEN** compiled C and Swift tests exercise every valid and invalid ABI, region, capacity, status, record, and model-mapping path
- **THEN** the two public symbols, fixed statuses, exact records, errors, and publication behavior remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation
- **WHEN** clean macOS/iOS packaging, dependency and source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** both Apple hosts link the correct Rust bridge from source, superseded Swift algorithms are absent, Android artifacts and Kotlin ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Apple physical selection
- **WHEN** focused identity, direct/CAN control, applicable BMS, telemetry, and mobile-background scenarios run on connected iOS and macOS against the ESP32 fixture
- **THEN** both hosts complete through production Rust legacy decoding with unchanged routing, state, session, control, background, and cleanup behavior

#### Scenario: Verify Android physical regression
- **WHEN** the same applicable scenarios run on the connected Android device
- **THEN** Android completes through its unchanged Kotlin legacy decoder without resolving or packaging the Apple bridge

#### Scenario: Defer only bounded location acquisition
- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory
