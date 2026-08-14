## ADDED Requirements

### Requirement: Safe Rust session-history reduction kernel

The repository SHALL provide one safe bounded Rust candidate for historical series reduction while Apple and Android retain their selected native reducers. The candidate SHALL consume ordered platform-neutral rows incrementally and own exact M4 role selection, null/gap continuity generations, and deterministic bounded column publication.

#### Scenario: Configure bounded reduction

- **WHEN** a caller supplies 1–16 unique bounded field names, an inclusive nonnegative elapsed range, 1–1,024 buckets, a valid explicit or automatic gap policy, positive persistence cadence, and bounded retained resolution
- **THEN** the candidate SHALL allocate only field/bucket-bounded state and preserve the supplied field order

#### Scenario: Select M4 roles

- **WHEN** ordered finite observations populate one or more buckets
- **THEN** each field/bucket SHALL retain the chronologically first and last rows plus the minimum and maximum finite values, resolve equal roles by earliest `(elapsedMs, sequence)`, deduplicate multi-role source rows, and publish candidates in chronological order

#### Scenario: Preserve discontinuities

- **WHEN** a value is absent, an elapsed delta exceeds the explicit threshold, or an automatic phase/cadence/resolution threshold is crossed
- **THEN** the affected continuity generation SHALL advance exactly and later values SHALL NOT be connected across that boundary

#### Scenario: Reject invalid ordered input

- **WHEN** configuration or input has duplicate/oversized fields, invalid ranges, invalid counts, non-finite present values, mismatched value arity, out-of-range rows, descending or duplicate keys, invalid cadence/thresholds, arithmetic overflow, generation exhaustion, or more than 65,536 selected observations
- **THEN** the candidate SHALL return one stable payload-free error without panicking, partially publishing a result, or retaining unbounded input

### Requirement: Safe Rust session prune-selection kernel

The Rust session-history candidate SHALL select retained real source rows from an ordered stream using elapsed-zero-aligned buckets, protected semantic evidence, deterministic extrema coverage, and an explicit cap of eight ordinary rows per bucket. Its live memory SHALL remain bounded by field count and the current bucket rather than ride length.

#### Scenario: Retain protected evidence

- **WHEN** a session stream contains endpoints, first non-null field sources, both sides of riding-phase transitions, caller-marked event sources, caller-marked summary sources, or other protected rows
- **THEN** every such source row SHALL be retained independently of the ordinary eight-row bucket cap

#### Scenario: Cover ordinary bucket roles

- **WHEN** an elapsed bucket contains unprotected first/last and ordered per-field minimum/maximum candidates
- **THEN** the candidate SHALL select deterministic maximum-coverage rows with chronological tie-breaking, retain no more than eight ordinary rows, and report any roles left uncovered by that cap

#### Scenario: Finalize incrementally

- **WHEN** ordered input advances into another elapsed-zero-aligned bucket or the caller finishes the stream
- **THEN** the candidate SHALL return the finalized prior selection in chronological order and release its raw bucket rows before accepting unbounded later history

#### Scenario: Preserve aligned coarser behavior

- **WHEN** previously retained source rows are selected again at a strictly coarser aligned integer-multiple resolution
- **THEN** protected evidence SHALL remain present and the deterministic result SHALL introduce no row absent from the supplied retained source

#### Scenario: Reject invalid prune input

- **WHEN** field, resolution, ordering, sequence, elapsed, value, bucket-width, counter, or role bounds are invalid or exhausted
- **THEN** the candidate SHALL fail closed with a stable payload-free error and SHALL publish no partial current bucket

### Requirement: Session-history kernel remains below persistence and runtime execution

The Rust session-history candidate SHALL expose only safe Rust configuration, row, column, selection, issue, and ordered-finalization values. It MUST NOT own or call SQLite, SQL, a persistence port, cursor JSON/base64, a local server, frontend code, BLE, callbacks, foreign handles, bridge records, threads, filesystems, sockets, async runtimes, or OS frameworks.

#### Scenario: Compose future SQLite directly

- **WHEN** a later platform-neutral Rust session store reads ordered SQLite rows or performs prune selection
- **THEN** it SHALL call this safe kernel directly and persist returned facts without a session-history Apple FFI or Android JNI boundary

#### Scenario: Preserve current production ownership

- **WHEN** the candidate is built and tested before runtime cutover
- **THEN** selected native reducers, stores, cursor encoders, loopback handlers, durable schemas, and application publication SHALL remain unchanged
