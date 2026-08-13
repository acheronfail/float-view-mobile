## ADDED Requirements

### Requirement: Android Rust Refloat legacy telemetry cutover

The Android backend SHALL use `refloat-legacy-telemetry` as its sole production implementation of fixed Refloat custom-app-data command-10 detail levels 1 through 4 and the active-firmware-fault form. Rust SHALL decide the exact envelope, form, and length; every signed scale and unit conversion; finite distance; package state and mode; footpad, stop, setpoint, alert, and firmware-fault classifications; boolean state flags; every optional detail tail; and field-weakening sentinel behavior. Kotlin SHALL perform only strict versioned-record validation, representation conversion into the existing normalized telemetry model, established error mapping, and higher-level behavior.

#### Scenario: Decode every Android legacy form through Rust
- **WHEN** Android production receives any valid command-10 detail-level or active-fault payload
- **THEN** its selected JNI path returns the canonical safe-component record and Kotlin publishes the exact established normalized telemetry result

#### Scenario: Preserve canonical unavailable values
- **WHEN** a payload omits a detail tail or carries the field-weakening sentinel
- **THEN** the canonical record and Android model retain absence rather than fabricating numeric zero or a sentinel measurement

#### Scenario: Fail closed without a Kotlin fallback
- **WHEN** the linked ABI, JNI call, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, or canonical absent values are invalid
- **THEN** Android returns its established bounded protocol failure without interpreting command-10 bytes in Kotlin or exposing payload data

#### Scenario: Preserve higher-level Android owners
- **WHEN** Rust returns a valid legacy telemetry record
- **THEN** existing Kotlin compatibility, routing, framing, scheduling, compact-cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Apple production ownership
- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking, loading, or calling the Android JNI artifact

#### Scenario: Retire the temporary bridge with the unified runtime
- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Android JNI and Apple FFI legacy-telemetry crates and host record adapters are deleted rather than retained as permanent APIs
