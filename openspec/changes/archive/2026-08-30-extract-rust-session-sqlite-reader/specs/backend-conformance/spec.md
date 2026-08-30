## ADDED Requirements

### Requirement: Rust session SQLite reader candidate evidence

The Rust session-reader extraction SHALL be backed by exact canonical database/query fixtures, exhaustive read/adversarial/concurrency behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later catalog/runtime selection.

#### Scenario: Preserve writer behavior through consolidation

- **WHEN** the writer-only package is renamed into the consolidated session SQLite package
- **THEN** every existing exact schema, creation, reopen, sample/location transaction, rollback, marker, bound, and diagnostic fixture SHALL pass unchanged and no superseded crate, fixture, recipe, audit, or bridge path SHALL remain

#### Scenario: Replay exact reader query vectors

- **WHEN** focused tests read committed metadata/marker/field facts and published stable/extension series, sample, event, and location vectors across nulls, ties, gaps, pages, continuation keys, active WAL, and reopen
- **THEN** every ordered key, nullable value, descriptor, generation, details payload, location gap/boundary, source-row count, and has-more result SHALL match immutable expected projections

#### Scenario: Exercise read failure and resource bounds

- **WHEN** tests cover every field/range/bucket/source/page/integer bound, stale/future/revision boundary, corrupt/incompatible schema, malformed/nonfinite durable value, unsupported field, invalid continuation, busy/locked state, cancellation, and diagnostic boundary
- **THEN** the reader SHALL remain bounded and deterministic, return no partial result, release every transaction/resource, remain usable after interruption, and expose only stable payload-free failures

#### Scenario: Prove WAL writer compatibility

- **WHEN** adversarial tests hold a published read snapshot while `SessionWriter` commits a newer sample/location batch
- **THEN** writer progress SHALL remain independent, the reader SHALL observe only its requested boundary, a later transaction SHALL observe the newer marker, and no read shall mutate or checkpoint the database

#### Scenario: Prove dependency and production isolation

- **WHEN** lockfile, dependency, copied-source, schema-identity, arbitrary-SQL, public-SQLite-type, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, server, generated-binding, and native-file audits run
- **THEN** one consolidated candidate SHALL own per-session SQLite write/read behavior through only its allowed graph, neither host SHALL select or package it, and no session SQLite FFI/JNI, runtime fallback, copied schema, or production source change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests/audits, warnings-denied offline locked workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native stores and readers still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification
