## ADDED Requirements

### Requirement: Rust connection application coordinator candidate evidence

The Rust connection application coordinator extraction SHALL be backed by shared source-derived lifecycle vectors, exhaustive and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime selection.

#### Scenario: Replay shared connection lifecycle vectors

- **WHEN** focused Rust tests run committed source-derived direct, gateway, explicit-selection, reconnect, standalone-BMS, telemetry, attached-BMS, timeout, failure, activation, and reset vectors
- **THEN** generations, request bytes/facts, scheduler/deadline order, child timers, selected facts, states, observations, samples, failures, hints, and diagnostics SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover every route and response family, maximum CAN/queue/telemetry/BMS collections, all scheduler completions, malformed payloads, activation order, child timer replacement, stale and mismatched input, nested-effect limits, reset, repeated transitions, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, read-only during discovery/telemetry, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the named safe Rust components, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native connection orchestration owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, BMS polling, standalone behavior, compatibility, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
