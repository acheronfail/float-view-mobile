## ADDED Requirements

### Requirement: Rust session SQLite pruning candidate evidence

The Rust session-pruning extraction SHALL be backed by exact canonical-database fixtures, exhaustive transaction and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later catalog/runtime selection.

#### Scenario: Replay exact preview and commit vectors

- **WHEN** focused tests replay empty, dense, endpoint, field/event/summary-source, phase-transition, extrema-tie, location-neighborhood, initial-prune, repeated-coarser, and no-op vectors
- **THEN** selected sequences, preview summaries, retained rows/evidence, deleted locations, metadata, recorded/stored counts, commit markers, history revisions, and reopen state SHALL match immutable expected projections

#### Scenario: Exercise stale, transactional, and resource bounds

- **WHEN** tests cover every resolution/alignment/source-count/integer/order/data bound, stale marker/resolution/identity, foreign keys, active WAL, busy/locked state, cancellation, corrupt/incompatible schema, and injected failure point
- **THEN** pruning SHALL remain bounded and deterministic, commit no partial mutation, preserve prior durable state, release every transaction/resource, and expose only stable payload-free failures

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, copied-source, schema-identity, arbitrary-SQL, public-SQLite-type, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, catalog, server, generated-binding, and native-file audits run
- **THEN** `session-sqlite` SHALL call `session-history-kernel` directly through only its allowed graph, neither host SHALL select or package pruning, and no pruning FFI/JNI, runtime fallback, copied schema, production source, or canonical SQL change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused locked tests/audits, warnings-denied workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native pruning owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification
