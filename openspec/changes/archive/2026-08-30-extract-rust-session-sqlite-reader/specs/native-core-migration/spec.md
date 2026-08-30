## ADDED Requirements

### Requirement: Read canonical session SQLite snapshots

The Rust session SQLite package SHALL provide an independent read-only `SessionReader` that validates one caller-selected canonical session database and returns only safe owned metadata, commit boundary, and numeric-field facts without mutating or discovering files.

#### Scenario: Open a compatible canonical database

- **WHEN** a session database passes integrity, required schema/index/singleton, positive cadence, marker, and field-evidence checks
- **THEN** the reader SHALL open it read-only and expose exact metadata, retained-resolution, commit-marker, and ordered stable/extension field facts

#### Scenario: Reject incompatible or malformed durable state

- **WHEN** a file is absent, corrupt, incompatible, missing required objects or singletons, contains invalid cadence/marker/field data, or cannot be opened read-only
- **THEN** the reader SHALL return a stable payload-free error without repairing, migrating, checkpointing, truncating, or otherwise mutating the file

#### Scenario: Validate a published boundary inside the read snapshot

- **WHEN** a query supplies a published history revision and sample/event/location sequences
- **THEN** the reader SHALL validate them against the commit marker inside the same SQLite read transaction, require an equal history revision and nonfuture sequences, and cap every scan at those sequences

### Requirement: Bounded canonical sample and series reads

The Rust session reader SHALL resolve 1–16 unique bounded requested numeric fields through one closed stable-field catalogue or validated extension-field evidence and SHALL return deterministic ordered sample pages or `session-history-kernel` series reductions from the canonical sample table.

#### Scenario: Reduce a published series window

- **WHEN** a valid nonnegative elapsed range, 1–1,024 buckets, canonical cadence/gap policy, retained resolution, published boundary, and at most 200,000 ordered source rows are supplied
- **THEN** the reader SHALL stream finite stable/extension values and exact ride phases into `SeriesAccumulator` and return the kernel's bounded columnar first/minimum/maximum/last observations, sequences, and gap generations without retaining the source window

#### Scenario: Page lossless sample rows

- **WHEN** a valid range, 1–2,000 row limit, requested fields, published boundary, and optional prior `(elapsed, sequence)` key are supplied
- **THEN** the reader SHALL return the next rows ordered by `(elapsed, sequence)`, exact nullable finite values, and an owned continuation key only when another row exists

#### Scenario: Reject an unsafe field or query bound

- **WHEN** fields are empty, duplicated, too many, oversized, unsupported, missing extension evidence, or resolve malformed/nonfinite stored values, or a range/bucket/source/page/integer bound is invalid
- **THEN** the reader SHALL fail closed without returning a partial page or reduction and SHALL interpolate no caller-controlled SQL identifier or JSON path

### Requirement: Bounded canonical event and location reads

The Rust session reader SHALL expose deterministic published event and location pages from canonical durable rows while leaving opaque cursor and wire-format ownership above the persistence package.

#### Scenario: Page supported transition events

- **WHEN** a valid range, 1–500 row limit, published event boundary, and optional prior `(elapsed, sequence)` key are supplied
- **THEN** the reader SHALL return only the closed supported transition kinds in `(elapsed, sequence)` order with exact details JSON and nullable source-sample sequence plus an owned continuation key when more rows exist

#### Scenario: Page recorded locations

- **WHEN** a valid range, 1–2,000 row limit, published location boundary, and optional prior location sequence are supplied
- **THEN** the reader SHALL return consecutive ordered finite coordinates, nullable nonnegative accuracy, observation time, deterministic greater-than-ten-second `noFix` gaps, the published boundary, and a continuation sequence only when more rows exist

#### Scenario: Reject malformed event or location state

- **WHEN** durable event JSON/kind/source facts or location sequence/time/coordinate/accuracy facts violate canonical bounds or ordering
- **THEN** the query SHALL return a stable payload-free failure and no partial page

### Requirement: Session SQLite reads remain interruptible and writer-compatible

The Rust reader SHALL hold no thread or async runtime but SHALL expose a safe cloneable interrupter capable of cancelling an executing SQLite scan from another runtime-owned thread. Read transactions SHALL use WAL-compatible snapshot semantics and release every statement and transaction after success, failure, or cancellation.

#### Scenario: Interrupt a long historical scan

- **WHEN** another thread interrupts an active bounded scan
- **THEN** the scan SHALL return the stable cancelled error, release its transaction and statements, reveal no query or data payload, and permit a later query on the reader

#### Scenario: Read while the writer advances

- **WHEN** `SessionWriter` commits a newer batch while a reader holds an older valid published snapshot
- **THEN** the writer SHALL complete without waiting for reduction, the reader SHALL observe only its transaction and published sequence boundaries, and a later read SHALL observe the newer marker

## RENAMED Requirements

- FROM: `### Requirement: Session writer remains below catalog and runtime execution`
- TO: `### Requirement: Session SQLite remains below catalog and runtime execution`

## MODIFIED Requirements

### Requirement: Session SQLite remains below catalog and runtime execution

The Rust session SQLite package SHALL own independent writer and read-only connections and expose only safe owned creation, open, append, marker, checkpoint, close, metadata, field, bounded query, reduction, page-key, and interruption values. It MUST NOT own catalog SQLite, settings, profiles, opaque cursors, history prune mutation, imports/exports, deletion/retention, recovery policy, local serving, gzip/JSON envelopes, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose future persistence directly

- **WHEN** a later Rust catalog/application runtime creates, records, or queries a session
- **THEN** it SHALL call the safe writer, reader, and history-kernel APIs directly without session SQLite Apple FFI or Android JNI boundaries

#### Scenario: Preserve current production ownership

- **WHEN** the consolidated candidate is built and tested before runtime cutover
- **THEN** selected native stores, catalog reconciliation, session readers/reducers, cursor encoders, recovery, files, loopback handlers, and application publication SHALL remain unchanged

#### Scenario: Bound dependencies and diagnostics

- **WHEN** maintainers inspect the locked graph or any valid/adversarial value reaches errors and debug formatting
- **THEN** dependencies SHALL remain limited to the named safe Rust telemetry/history owners, maintained bundled SQLite/JSON support and their build graph, and output SHALL reveal no path, SQL, telemetry, coordinate, extension, event, JSON, or SQLite-message payload
