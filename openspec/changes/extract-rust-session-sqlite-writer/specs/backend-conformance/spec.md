## ADDED Requirements

### Requirement: Rust session SQLite writer candidate evidence

The Rust session-writer extraction SHALL be backed by exact canonical-schema and database-state fixtures, exhaustive transaction/adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later catalog/runtime selection.

#### Scenario: Prove exact schema creation

- **WHEN** focused tests create a session database from the compiled schema
- **THEN** the schema input SHALL be byte-identical to `backends/sql/session.sql` and required pragmas, tables, indexes, constraints, metadata, singleton marker, and initial values SHALL match canonical expected evidence

#### Scenario: Replay exact sample state vectors

- **WHEN** focused tests replay committed direct/dynamic/legacy, nullable, extension, transition, duplicate, extrema-tie, multi-batch, and reopen sample vectors
- **THEN** sample rows, extension JSON, numeric-field sources, event rows, recorded/stored/event counts, committed sequences/time, riding elapsed, distance, extrema/source sequences, and last state SHALL match immutable expected database projections

#### Scenario: Replay exact location state vectors

- **WHEN** focused tests replay unordered, duplicate, boundary-coordinate, nullable-accuracy, multi-batch, and reopen location vectors
- **THEN** stored order/sequences and every committed/recorded/stored/status marker field SHALL match immutable expected database projections

#### Scenario: Exercise transaction and adversarial bounds

- **WHEN** tests cover every empty/minimum/maximum batch, integer conversion, order, enum, finite numeric, extension count/name/JSON, timestamp/elapsed, coordinate/accuracy, target existence, schema/singleton/cadence, corrupt database, busy/locked, injected mid-transaction failure, reopen, and diagnostic boundary
- **THEN** the writer SHALL remain bounded and deterministic, commit no partial batch, preserve prior durable state, and expose only stable payload-free failures

#### Scenario: Prove dependency and production isolation

- **WHEN** lockfile, dependency, copied-source, schema-identity, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, catalog, server, generated-binding, and native-file audits run
- **THEN** the candidate SHALL own the SQLite write behavior through only its allowed graph, neither host SHALL select or package it, and no writer FFI/JNI, runtime fallback, copied schema, or production source change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests/audits, warnings-denied offline locked workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native stores still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification
