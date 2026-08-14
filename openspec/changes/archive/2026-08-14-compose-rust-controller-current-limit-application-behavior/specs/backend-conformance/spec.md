## ADDED Requirements

### Requirement: Rust controller current-limit composition evidence

The Rust controller current-limit application composition SHALL be backed by shared source-derived activation/completion vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later runtime selection.

#### Scenario: Replay shared current-limit composition vectors

- **WHEN** focused Rust tests replay committed direct/CAN activation order, exact command-14 routing, valid/optional-cell responses, standalone-BMS exclusion, malformed/timeout/queue/cancellation, reconnect, and reset vectors
- **THEN** scheduler facts, decoded values, snapshots, publications, diagnostics, and effect order SHALL match the selected Swift and Kotlin application behavior

#### Scenario: Exercise the complete current-limit safety matrix

- **WHEN** tests cover supported controller and standalone-BMS identities, every route, valid/unknown/truncated signatures, absent/zero/positive cell counts, finite/nonzero/invalid auto-floats, every scheduler completion, stale/mismatched/repeated input, maximum payload, generation replacement, reconnect, and reset
- **THEN** the composition SHALL remain bounded, deterministic, panic-free for public input, non-fatal to readiness, and free of stale, partial, or private-data publication

#### Scenario: Prove direct dependency and production isolation

- **WHEN** dependency, copied-logic, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the coordinator SHALL directly use the safe controller-data crate, neither host SHALL select or package the coordinator, and no new FFI/JNI, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the composition is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native application owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN current-limit publication, discovery, configuration, controls, shutdown, telemetry, BMS, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
