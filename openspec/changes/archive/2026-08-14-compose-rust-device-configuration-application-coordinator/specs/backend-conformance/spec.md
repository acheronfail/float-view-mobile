## ADDED Requirements

### Requirement: Rust device configuration composition evidence

The Rust device/configuration connection-coordinator composition SHALL be backed by shared source-derived lifecycle vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later runtime selection.

#### Scenario: Replay shared application composition vectors

- **WHEN** focused Rust tests replay committed direct/CAN activation, cache hit/miss/corruption, schema paging, current/default reads, control/shutdown/configuration exclusion, write/acknowledgement/readback, Tune lease, telemetry/lighting confirmation, flywheel, failure, reconnect, and reset vectors
- **THEN** snapshots, scheduler facts, mutation tokens, configuration operations, cache effects, confirmations, failures, diagnostics, and effect order SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise the complete composition safety matrix

- **WHEN** tests cover every supported/read-only/unsupported identity, route, generation, configuration state, mutation state, scheduler completion, response family, stale/mismatched input, malformed/maximum payload, cache outcome, nested effect bound, counter rollover, cancellation, reconnect, and reset
- **THEN** the composition SHALL remain bounded, deterministic, panic-free for public input, fail closed before unsafe work, release every terminated lease, and publish no stale or private data

#### Scenario: Prove safe dependencies and production isolation

- **WHEN** dependency, copied-logic, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the coordinator SHALL directly use only named safe Rust owners/codecs, neither host SHALL select or package it, and no new FFI/JNI, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the composition is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native application owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, controls, mutation exclusion, configuration schema/current/default/write/readback, shutdown, telemetry, BMS, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
