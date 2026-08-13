## ADDED Requirements

### Requirement: Apple Rust Refloat configuration codec cutover evidence

The repository SHALL prove source parity, canonical-record safety, ABI safety, strict Swift adaptation, clean packaging, sole production selection, and all-host physical behavior before accepting the Apple Rust Refloat configuration codec cutover.

#### Scenario: Exercise safe canonical records

- **WHEN** safe-crate tests run
- **THEN** they SHALL round-trip every schema field/default/option/choice/description/group and every typed value
- **AND** SHALL reject truncation, trailing bytes, count and length overflow, unknown versions/kinds/types, nonzero reserved bytes, invalid UTF-8, non-finite numbers, non-canonical booleans, duplicate/missing/extra fields, schema mismatch, and configured size bounds
- **AND** SHALL preserve upstream fixture fingerprint, wire signature, value bytes, and checksum evidence

#### Scenario: Exercise the compiled Apple ABI

- **WHEN** Rust bridge tests and a compiled-C harness invoke all four symbols
- **THEN** they SHALL prove ABI versioning, exact record bytes, fixed status mapping, caller-owned output behavior, too-small output handling, null and oversize rejection, misaligned scalar rejection, overlapping-region rejection, deterministic outputs, panic containment, and no retained state

#### Scenario: Exercise the strict Swift adapter

- **WHEN** focused Apple tests run
- **THEN** they SHALL prove exact projection of schema and values, mechanical value-record construction, stale-generation rejection, current/default/write parity, bridge-error mapping, ABI mismatch rejection, malformed-record rejection, and schema/write/readback service behavior
- **AND** SHALL prove production code cannot select or fall back to superseded Swift codec algorithms

#### Scenario: Verify clean packaging and source ownership

- **WHEN** repository audits and clean package builds run
- **THEN** macOS, iOS device, and iOS simulator SHALL build the selected bridge from locked source and link the intended archive and C module
- **AND** no generated binding or native archive SHALL be committed
- **AND** Android's typed bridge catalogue, Gradle tasks, packages, and artifacts SHALL remain exactly unchanged
- **AND** source audits SHALL find one safe Rust protocol owner and only bounded ABI/record adaptation in Apple production sources

#### Scenario: Pass complete software regression

- **WHEN** the standard repository software gate runs after production selection and Swift deletion
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass

#### Scenario: Verify every production host with the ESP32 fixture

- **WHEN** the flashed ESP32 fixture and production Android, iOS, and macOS apps run the physical checkpoint
- **THEN** Android SHALL continue exercising its selected Kotlin configuration codec
- **AND** iOS and macOS SHALL exercise the selected Rust configuration codec
- **AND** each host SHALL pass BLE connection, direct and CAN telemetry, Refloat configuration schema/current/default/write/readback behavior, controller and BMS routing, reconnect, files, theme, and applicable lifecycle/background checks
- **AND** mobile background logging SHALL remain successful
- **AND** only a bounded unavailable location sample MAY be recorded for later manual verification
- **AND** no BLE, codec, configuration, packaging, routing, persistence, background, or application failure MAY be deferred
