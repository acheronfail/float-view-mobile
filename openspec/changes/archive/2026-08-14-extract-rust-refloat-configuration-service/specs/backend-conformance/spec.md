## ADDED Requirements

### Requirement: Rust Refloat configuration service candidate evidence

The Rust Refloat configuration service candidate SHALL NOT be treated as a production owner until locked formatting, warnings-denied linting, dependency and workspace audits, source-derived catalogue/fixture parity, exhaustive native-transition and adversarial tests, the complete `just verify` software gate, and Android/iOS/macOS ESP32 regression pass. Candidate evidence SHALL cover cache hit/miss/corruption, every audited release, legacy patch resolution, direct/CAN opaque routes, schema paging bounds, generation and operation identity, current/default revisions, read-only and flywheel policy, write acknowledgement/readback, normalization, conflicts, failures, aborts, reset/reconnect, counter boundaries, deterministic snapshots/effects, and payload-safe diagnostics.

#### Scenario: Replay every selected native transition

- **WHEN** focused Rust tests replay the complete Swift and Kotlin service vectors using immutable release fixtures
- **THEN** every snapshot, request effect, cache decision, payload, revision, operation, error, recovery, and normalization result SHALL match the selected native behavior

#### Scenario: Exercise adversarial lifecycle sequences

- **WHEN** focused tests permute stale generations, operations, routes, indexes, commands, chunks, offsets, lengths, responses, acknowledgements, failures, flywheel transitions, resets, and counter boundaries
- **THEN** the candidate SHALL remain bounded and deterministic, fail closed with stable payload-free errors, and publish no partial state or effect

#### Scenario: Verify locked isolation

- **WHEN** workspace, dependency, source-selection, bridge-catalogue, symbol, generated-artifact, and package audits run
- **THEN** the candidate SHALL depend only on the existing safe codec, contain no unsafe or host integration, and leave every production artifact and selected owner unchanged

#### Scenario: Pass complete software regression

- **WHEN** `just verify` runs with the isolated candidate present
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass without a contract or persistent-format change

#### Scenario: Verify all-host physical regression

- **WHEN** production Android, iOS, and macOS execute their complete applicable ESP32 physical matrices
- **THEN** each host SHALL preserve BLE, direct/CAN configuration schema/current/default/write/readback, telemetry, BMS, controls, reconnect, sessions, persistence, files, theme, cleanup, and mobile background logging behavior through its unchanged selected native service
- **AND** only bounded mobile location-sample absence MAY be deferred for manual verification
- **AND** no non-location, application, protocol, persistence, background, or cleanup failure MAY be deferred

#### Scenario: Defer host cutover evidence

- **WHEN** candidate parity and regression checks pass
- **THEN** the change SHALL record extraction readiness only and SHALL require separate Apple and Android changes to prove adapter mapping, production selection, physical Rust execution, and native-service deletion
