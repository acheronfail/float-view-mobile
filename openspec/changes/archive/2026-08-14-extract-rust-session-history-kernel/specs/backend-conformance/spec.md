## ADDED Requirements

### Requirement: Rust session-history kernel candidate evidence

The Rust session-history extraction SHALL be backed by shared immutable expected vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later persistence/runtime selection.

#### Scenario: Replay shared series vectors

- **WHEN** focused Rust tests replay committed edge, tie, gap, late-field, null, phase-cadence, retained-resolution, bucket-count, dense, and ordered-finalization vectors
- **THEN** M4 source sequences, finite values, continuity generations, field order, role deduplication, and bounds SHALL match the canonical expected evidence

#### Scenario: Replay shared prune vectors

- **WHEN** focused Rust tests replay committed endpoints, field starts, phase boundaries, event/summary protection, aligned buckets, extrema ties, multi-role coverage, eight-row-cap, and repeated-coarser vectors
- **THEN** retained sequences, protected/ordinary classification, uncovered roles, chronological finalization, and bounds SHALL match the canonical expected evidence

#### Scenario: Exercise complete adversarial bounds

- **WHEN** tests cover every empty/minimum/maximum field, bucket, row, resolution, threshold, cadence, retained-resolution, elapsed, sequence, numeric, ordering, arity, generation, role, output, and arithmetic boundary plus malformed fixture input
- **THEN** the candidate SHALL remain deterministic, bounded, standard-library-only, panic-free for public input, and free of partial or private-data diagnostics

#### Scenario: Prove direct future ownership and production isolation

- **WHEN** dependency, copied-source, source-selection, bridge-catalogue, symbol, package, callback, SQLite, persistence, network, generated-artifact, and native-file audits run
- **THEN** the candidate SHALL own the safe algorithms without importing a native/frontend implementation, neither host SHALL select or package it, and no FFI/JNI, generated binding, database/runtime dependency, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native session owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session creation/history/pruning, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
