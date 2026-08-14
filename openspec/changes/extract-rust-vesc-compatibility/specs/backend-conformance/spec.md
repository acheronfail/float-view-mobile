## ADDED Requirements

### Requirement: Rust VESC compatibility candidate evidence

The Rust VESC compatibility extraction SHALL be backed by immutable fixture composition, exhaustive source-policy parity, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before it may be used by later Rust application services.

#### Scenario: Compose immutable firmware fixtures
- **WHEN** focused Rust tests unwrap and decode the audited THOR300 6.02, 6.05, 6.06, and 7.00 firmware fixtures
- **THEN** each exact stable identity SHALL receive the expected read/write and shutdown decisions through the safe identity and compatibility components

#### Scenario: Exhaust the finite compatibility policy
- **WHEN** focused tests enumerate every major/minor pair, hardware-type and test-marker boundaries, both operations, exact shutdown releases, adjacent versions, and unrelated identity-field changes
- **THEN** every decision SHALL match the complete selected Swift/Kotlin policy without inference or partial admission

#### Scenario: Prove direct composition and production isolation
- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the safe firmware-identity component, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification
- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native compatibility owners still selected

#### Scenario: Preserve all-host physical behavior
- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, compatibility, access, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
