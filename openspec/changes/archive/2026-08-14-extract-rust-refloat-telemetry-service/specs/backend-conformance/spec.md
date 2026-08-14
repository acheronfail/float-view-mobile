## ADDED Requirements

### Requirement: Rust Refloat telemetry service candidate evidence

The Rust Refloat telemetry-service extraction SHALL be backed by source-derived parity, adversarial bounds, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior with the ESP32 fixture before it may be treated as ready for later unified-runtime composition.

#### Scenario: Replay complete native telemetry policy
- **WHEN** focused Rust tests run vectors derived from the complete Swift and Kotlin telemetry services, protocol models, and canonical telemetry requirements
- **THEN** layout retry, exact request facts/bytes, pending attribution, timeout recovery, cadence, compact replacement/merge, normalization, sequence, elapsed time, reset, and ordered effects SHALL match selected behavior

#### Scenario: Exercise adversarial service bounds
- **WHEN** tests reach invalid settings/time arithmetic, maximum routes/generations/tokens/sequences/extensions, stale or repeated timers/completions/responses, active faults, cache replacement, decreasing totals, stop/reset races, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public inputs, and free of duplicate, stale, or misattributed publication

#### Scenario: Prove direct composition and production isolation
- **WHEN** dependency, symbol, package, build-catalogue, source, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe Rust components, neither host SHALL select or package it, and no new Apple FFI, Android JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification
- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests/audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native services still selected

#### Scenario: Preserve all-host physical behavior
- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, connection, telemetry cadence, session, background, import/export, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
