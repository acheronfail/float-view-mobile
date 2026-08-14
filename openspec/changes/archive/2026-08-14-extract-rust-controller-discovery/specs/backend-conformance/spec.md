## ADDED Requirements

### Requirement: Rust controller-discovery candidate evidence

The Rust controller-discovery extraction SHALL be backed by shared source-derived vectors, exhaustive and adversarial parity, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime composition.

#### Scenario: Replay shared discovery vectors
- **WHEN** focused Rust tests run every committed controller-discovery TSV vector and source-derived native lifecycle case
- **THEN** route, ordered effects, candidates, inventory, generations, readiness, and stable issue outcomes SHALL match the selected Swift and Kotlin state machines

#### Scenario: Exercise finite bounds and adversarial order
- **WHEN** tests cover every CAN ID boundary, duplicate/order normalization, 64/65 devices, 16/17 controllers, supported/unsupported/test firmware, meaningful and sentinel UUIDs, configured/hinted/continuity paths, timeouts, stale facts, invalid selections, reset, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation
- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe components, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification
- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests/audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native discovery owners still selected

#### Scenario: Preserve all-host physical behavior
- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, compatibility, candidate selection, continuity, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
