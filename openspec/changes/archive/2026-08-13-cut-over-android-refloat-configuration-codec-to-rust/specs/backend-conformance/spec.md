## ADDED Requirements

### Requirement: Android Rust Refloat configuration codec cutover evidence

The repository SHALL prove safe-record reuse, JNI safety, strict Kotlin adaptation, clean packaging, sole Android production selection, Apple isolation, and all-host physical behavior before accepting the Android Rust Refloat configuration codec cutover.

#### Scenario: Exercise the compiled Android JNI boundary

- **WHEN** focused Rust bridge tests and host-JVM calls invoke all four methods
- **THEN** they SHALL prove ABI versioning, exact schema/value records and payloads, fixed status mapping, atomic two-pass output behavior, null/exception/reference/length/capacity/distinctness rejection, deterministic outputs, panic containment, and no retained state

#### Scenario: Exercise the strict Kotlin adapter

- **WHEN** focused Android tests run against every audited schema and value fixture
- **THEN** they SHALL prove exact schema/value projection, mechanical value-record construction, every typed value, stale-generation rejection, current/default/write/readback parity, ABI/status mapping, malformed-record rejection, and service behavior
- **AND** SHALL prove production code cannot select or fall back to superseded Kotlin codec algorithms

#### Scenario: Verify clean packaging and source ownership

- **WHEN** repository audits and clean package builds run
- **THEN** host JVM and every supported Android ABI in Debug and Release SHALL build and consume the selected JNI library from locked source
- **AND** no generated binding or native artifact SHALL be committed
- **AND** Apple's package graph, bridge ABI, adapter, archives, symbols, and selected behavior SHALL remain exactly unchanged
- **AND** source audits SHALL find one safe Rust protocol owner and only bounded JNI/record adaptation in Android production sources

#### Scenario: Pass complete software regression

- **WHEN** the standard repository software gate runs after Android production selection and Kotlin algorithm deletion
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass

#### Scenario: Verify every production host with the ESP32 fixture

- **WHEN** the flashed ESP32 fixture and production Android, iOS, and macOS apps run the physical checkpoint
- **THEN** every host SHALL exercise its selected Rust Refloat configuration codec
- **AND** each host SHALL pass BLE connection, direct and CAN telemetry, configuration schema/current/default/write/readback behavior, controller and BMS routing, reconnect, files, theme, persistence, and applicable lifecycle/background checks
- **AND** mobile background logging SHALL remain successful
- **AND** only a bounded unavailable location sample MAY be recorded for later manual verification
- **AND** no BLE, codec, configuration, packaging, routing, persistence, background, application, or cleanup failure MAY be deferred
