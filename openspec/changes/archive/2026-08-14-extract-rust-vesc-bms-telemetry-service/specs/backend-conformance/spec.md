## ADDED Requirements

### Requirement: Rust VESC BMS telemetry service candidate evidence

The Rust VESC BMS telemetry-service extraction SHALL be backed by shared source-derived lifecycle vectors, exhaustive and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime composition.

#### Scenario: Replay shared BMS lifecycle vectors

- **WHEN** focused Rust tests run every committed source-derived controller-attached and standalone BMS discovery/polling lifecycle vector
- **THEN** request bytes/facts, ordered effects, route, cadence, failures, status, observation presence, generations, reset, and stable diagnostics SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover direct/preferred/reused/enumerated routes, every CAN ID, maximum inventories and command-96 collections, all scheduler completions, malformed payloads, stale and mismatched input, three-failure recovery, timer replacement, reset, repeated transitions, counter rollover, and private values
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, read-only, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, executor, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe crates, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native BMS lifecycle owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, BMS polling, standalone behavior, compatibility, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
