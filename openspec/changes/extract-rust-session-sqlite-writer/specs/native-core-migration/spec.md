## ADDED Requirements

### Requirement: Rust canonical session SQLite writer

The repository SHALL provide one bounded Rust candidate that creates and reopens a canonical per-session SQLite file and atomically persists normalized telemetry, derived field/event evidence, commit summaries, and location observations while selected Apple and Android stores remain unchanged.

#### Scenario: Create an exact canonical session file

- **WHEN** a caller supplies a new path, positive bounded idle/riding persistence cadences, and a commit timestamp
- **THEN** the writer SHALL initialize the byte-exact `backends/sql/session.sql` schema, one metadata row, and one zero-count commit marker in a private temporary file and atomically publish only a complete database without overwriting an existing target

#### Scenario: Reopen a compatible database

- **WHEN** a completed canonical session file passes integrity and required schema/singleton/cadence checks
- **THEN** the writer SHALL reopen it and continue durable sample, event, field, location, and marker sequences from stored state

#### Scenario: Reject an incompatible database

- **WHEN** a target is missing required schema objects or singleton rows, is corrupt, has invalid metadata, or fails canonical compatibility checks
- **THEN** the writer SHALL return a stable payload-free incompatibility or corruption error without repairing, migrating, truncating, or partially mutating the file

### Requirement: Atomic normalized telemetry persistence

The Rust writer SHALL accept bounded ordered `refloat-telemetry-service::NormalizedSample` batches directly and persist the canonical sample columns, bounded deterministic extension JSON, first-source numeric fields, transition events, and exact commit-marker aggregates in one immediate transaction.

#### Scenario: Persist a new ordered batch

- **WHEN** 1–1,024 valid strictly ordered samples contain bounded finite stable and extension values
- **THEN** the writer SHALL insert every new sequence, preserve all canonical nullable values, discover each stable/extension field once, emit only changed transition values, and atomically update recorded/stored/event counts, committed sequences/time, riding elapsed, distance, extrema values and earliest source sequences, plus last speed/state

#### Scenario: Replay already stored sequences

- **WHEN** an otherwise valid ordered batch includes sample sequences already present in the database
- **THEN** existing sample bytes, first-field sources, and transition events SHALL remain unchanged, no duplicate stored row SHALL appear, recorded count SHALL include every submitted sample, and the marker SHALL remain a deterministic projection of durable rows plus submitted-count semantics

#### Scenario: Continue across batches and reopen

- **WHEN** ordered samples are appended in multiple calls separated by checkpoint, close, or reopen
- **THEN** transition comparison, event sequence, first-field discovery, summaries, extrema ties, counts, and committed source sequences SHALL match one continuous durable session

#### Scenario: Roll back a failed sample transaction

- **WHEN** any input bound, conversion, JSON bound, SQLite statement, constraint, busy/full/I/O condition, aggregate computation, or marker update fails
- **THEN** the call SHALL expose one stable payload-free error and SHALL commit none of that batch's samples, fields, events, counts, or marker changes

### Requirement: Atomic canonical location persistence

The Rust writer SHALL accept bounded already-observed location facts independently of location acquisition and persist canonical validated/deduplicated rows plus marker state in one immediate transaction.

#### Scenario: Persist ordered locations

- **WHEN** 1–1,024 observations contain durable nonnegative time/elapsed values, finite in-range coordinates, and absent or finite nonnegative accuracy
- **THEN** the writer SHALL order them deterministically, ignore existing elapsed/latitude/longitude duplicates, assign consecutive location sequences, and atomically update committed location sequence/elapsed, recorded/stored counts, and available status

#### Scenario: Preserve an all-duplicate marker

- **WHEN** every valid submitted observation already exists
- **THEN** no new row SHALL be stored and every location marker field SHALL remain unchanged

#### Scenario: Roll back a failed location transaction

- **WHEN** any observation bound, conversion, SQLite statement, constraint, busy/full/I/O condition, or marker update fails
- **THEN** the call SHALL expose one stable payload-free error and SHALL commit none of that batch's location rows or marker changes

### Requirement: Session writer remains below catalog and runtime execution

The Rust session writer SHALL own one connection and expose only safe owned creation, open, append, marker, checkpoint, and close values. It MUST NOT own catalog SQLite, settings, profiles, history reads/pruning, imports/exports, deletion/retention, recovery policy, local serving, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose future persistence directly

- **WHEN** a later Rust catalog/application runtime creates or records a session
- **THEN** it SHALL call the safe writer directly without a session-writer Apple FFI or Android JNI boundary

#### Scenario: Preserve current production ownership

- **WHEN** the writer candidate is built and tested before runtime cutover
- **THEN** selected native stores, catalog reconciliation, session readers/reducers, recovery, files, loopback handlers, and application publication SHALL remain unchanged

#### Scenario: Bound dependencies and diagnostics

- **WHEN** maintainers inspect the locked graph or any valid/adversarial value reaches errors and debug formatting
- **THEN** dependencies SHALL remain limited to the named safe Rust telemetry owner, maintained bundled SQLite/JSON support and their build graph, and output SHALL reveal no path, SQL, telemetry, coordinate, extension, or SQLite-message payload
