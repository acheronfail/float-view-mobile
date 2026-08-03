## MODIFIED Requirements

### Requirement: Equivalent native detailed persistence

The Apple and Android current schemas SHALL durably store every stable numeric telemetry value in the matching dedicated nullable sample column, unknown negotiated numeric values in bounded `extension_json`, observed field presence in a bounded per-session table, and discrete transitions in `session_event` with an independent monotonic event sequence, source sample sequence, observation time, elapsed time, and single-session ownership. Multiple transitions from one sample SHALL be inserted in stable catalog order. Session metadata SHALL capture its positive bounded riding and idle persistence intervals. The per-session commit marker SHALL atomically record committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, commit time, and the bounded riding/distance/maximum/source-sequence/last-value projection required to recover catalog summaries with those rows. The schema SHALL initialize incremental auto-vacuum and bounded prune metadata/job state. Denormalized catalog summaries SHALL converge before a boundary is published and SHALL be reconciled from the marker after interruption or an in-process catalog failure before that session is read or appended again. Incompatible pre-release databases SHALL be deleted and recreated as the canonical first production schema rather than migrated.

#### Scenario: Equivalent stored sample

- **WHEN** both native backends accept equivalent normalized complete-catalog telemetry
- **THEN** dedicated columns, field presence, session-data retrieval, and export produce semantically equivalent numeric records and transitions despite native implementation differences

#### Scenario: Atomic session marker

- **WHEN** a batch introduces samples, multiple transition events, and a late extension field
- **THEN** those rows and the corresponding sample/event sequences, source evidence, recorded/stored counts, history revision, elapsed boundary, field presence, and recovery summary projection become durable in one session-database transaction

#### Scenario: Old disposable schema

- **WHEN** development storage contains the previous `detail_json` session layout or another incompatible version
- **THEN** normal test-stage recreation installs the complete optimized schema without migration, legacy reads, partial columns, or backfill

## ADDED Requirements

### Requirement: Native standalone session catalog and grouping removal

Apple and Android SHALL expose the same bounded `session.list` control operation over their existing native session stores. The query SHALL use the catalog's `(started_at_ms, session_id)` keyset order, return at most the requested page size, encode the query shape/catalog revision/last-key context in its opaque cursor, and avoid offset scans. Complete state snapshot construction SHALL not query or serialize session summaries during telemetry-driven publication; it SHALL expose only the bounded session-catalog revision marker. The canonical schema, settings store, application service, loopback handlers, and mock SHALL contain no session-group tables, group-member tables, automatic-linking fields, group mutation ports, or linked-ride payloads. Since storage remains disposable test-stage data, incompatible group-bearing schemas SHALL be recreated without migration.

#### Scenario: Indexed session page

- **WHEN** both hosts serve successive valid `session.list` requests over a catalog larger than one page
- **THEN** both use the checked started-time/session-ID keyset order, return identical standalone summaries and continuation behavior, and do not use offset pagination

#### Scenario: Catalog query absent from telemetry snapshot

- **WHEN** a persisted telemetry sample invalidates application state
- **THEN** neither host queries or serializes a session-summary or group collection for that state event, and the unchanged session-catalog revision remains available

#### Scenario: Grouping removed from native surfaces

- **WHEN** a backend is built from the canonical disposable schema and exposes settings or control methods
- **THEN** group tables, linked-ride records, automatic-linking options, and group/unlink operations are absent on Apple and Android

#### Scenario: Cursor context mismatch

- **WHEN** a session-list cursor is malformed, rebound to another query shape, or issued before a newer catalog revision
- **THEN** both hosts reject it with the same bounded error and do not return a mixed page

### Requirement: Mirrored native session-data adapters

Apple and Android SHALL implement the same authenticated session-data request kinds with small backend-owned adapters over their production SQLite session stores. Each adapter SHALL validate the shared hand-written contract, acquire the requested published history revision and boundary, select only validated columns/ranges plus internal ride phase, stream ordered rows through an equivalent bounded configurable-gap first/minimum/maximum/last reducer, serialize the same explicit columnar response, and apply standard gzip encoding using platform facilities. Swift and Kotlin implementation code SHALL remain backend-owned; shared behavior SHALL be enforced through the contract, canonical SQL, immutable vectors, mock, and conformance rather than a generated or cross-language runtime.

#### Scenario: Equivalent historical query

- **WHEN** Apple and Android query equivalent stored rows with the same series, samples, or events request
- **THEN** they return semantically identical snapshots, columnar values, extrema, continuity segments, ordering, and continuation behavior

#### Scenario: Platform compression

- **WHEN** the frontend accepts gzip from either host
- **THEN** each host uses maintained platform facilities and returns the same decoded contract without a shared runtime dependency

#### Scenario: Backend ownership

- **WHEN** the WebView is destroyed, recreated, or aborts a query
- **THEN** native storage, active logging, snapshot publication, cancellation, recovery, and resource cleanup remain owned by the host backend

### Requirement: Historical reads do not obstruct persistence

Each host SHALL resolve session metadata under only a short store synchronization boundary, then use a dedicated read-only/query-only connection, one per-session read lease, and an explicit bounded WAL read transaction for the scan. Historical work SHALL execute away from the loopback and session-writer executors, SHALL allow no more than two concurrent reads per backend, SHALL remain cancellable throughout query/reduction/serialization, and SHALL release cursors, transactions, connections, reducers, and compression buffers after success, cancellation, disconnect, or serialization failure. Apple SHALL provide connection-local SQLite interruption/progress cancellation and Android SHALL provide database cancellation tied to coroutine/request lifetime. Releasing cancelled work within 250 milliseconds and ordinary completion within two seconds on the reference devices SHALL be recorded performance targets, not correctness deadlines.

#### Scenario: Review an active long session

- **WHEN** a historical overview scans an active 150,000-sample session while another sample becomes due for persistence
- **THEN** the durable append remains within its existing bound and the query observes only its requested published snapshot

#### Scenario: Superseded zoom request

- **WHEN** repeated pan or pinch interactions cancel an older native query
- **THEN** the host requests immediate interruption, exposes no stale result, eventually releases every resource without accumulating jobs or silently pinning WAL, and records whether the cancellation-release target was met

#### Scenario: Loopback remains responsive

- **WHEN** a long query, gzip encoding, or client backpressure is active
- **THEN** control-plane requests and authoritative state publication do not wait for that historical work's executor

### Requirement: Historical readers coordinate with recovery, export, pruning, and deletion

Opening a historical request SHALL acquire a lease only while the catalog row is readable and not pending maintenance or deletion. Deletion and pruning SHALL prevent new leases before requesting cancellation from existing leases and SHALL mutate or remove SQLite files only after every reader releases them. If release is slow, the operation SHALL remain visibly pending and cancellable instead of deleting beneath an open connection or relying on a correctness timeout. Pending deletion cancellation SHALL restore readability only when it wins before irreversible file removal; otherwise deletion SHALL report its stable completed result. Export, compaction, and recovery SHALL use the same file-lifecycle boundary wherever their snapshot or file operations could race a reader. Startup SHALL perform bounded integrity checks and reconcile catalog counts/boundaries/revisions from the authoritative marker before any recovered session can be queried.

#### Scenario: Delete during a historical read

- **WHEN** deletion begins while a completed-session query holds a read lease
- **THEN** no new query opens, cancellation is requested from the existing query, and files are removed only after its connection closes even when release exceeds the performance target

#### Scenario: Interrupted catalog update

- **WHEN** startup finds a valid session marker newer than the denormalized catalog
- **THEN** recovery repairs the catalog before metadata or history exposes the session

#### Scenario: Catalog failure without process exit

- **WHEN** the session marker commits and the following catalog update fails
- **THEN** that session remains unpublished and dirty until marker reconciliation succeeds, and another append does not proceed against divergent metadata

#### Scenario: Failed reader cleanup

- **WHEN** a query fails after opening its WAL snapshot but before completing gzip output
- **THEN** a later append, export, deletion, and historical query all remain usable without leaked leases or SQLite resources

### Requirement: Indexed field-minimized historical access

Each host SHALL map every stable contract field to its dedicated SQL column and SHALL parse `extension_json` at most once per row only when requested extensions are present. Series scans, keyset sample/event pages, and exact predecessor/successor seeks SHALL constrain the published sequence and elapsed range, preserve `(elapsedMs, sequence)` order, and use representative checked query plans without an avoidable full-table sort. Metadata SHALL obtain available fields from bounded field-presence rows rather than scanning samples.

#### Scenario: Stable field query

- **WHEN** a series request asks for two stable fields from a complete-catalog session
- **THEN** each host selects those two numeric columns plus ordering fields without decoding stable per-row JSON or unrequested telemetry

#### Scenario: Extension query

- **WHEN** a request asks for several valid extension fields
- **THEN** each selected row's bounded extension object is decoded no more than once and only the requested values reach the reducer

#### Scenario: Indexed exact lookup

- **WHEN** a nearest-sample tooltip request targets the middle of a long session
- **THEN** the host compares indexed exact, predecessor, and successor candidates rather than sorting the full range by absolute distance

### Requirement: Mirrored native ride pruning

Each host SHALL implement the same cancellable one-ride and all-completed-rides prune operations over an exclusive per-session maintenance lease. The selector SHALL retain field-presence and semantic source samples plus no more than eight ordinary real source rows per elapsed-zero-aligned bucket using the shared deterministic coverage rule. It SHALL stream one bucket with bounded native memory and place retained sequences in an indexed file-backed SQLite keep-set with bounded cleanup rather than an in-memory ride-sized collection. A per-ride logical prune SHALL be transactional, increment history revision, preserve original available fields/summaries/events/recorded count, update stored count and policy, and invalidate older readers and exports. Batch progress SHALL use a single bounded persisted job, a captured upper session key, and stable keyset traversal over rides completed at job start; it SHALL resume safely after process interruption and SHALL be per-ride atomic. A cancellable WAL checkpoint and incremental compaction SHALL be resumable and SHALL NOT make a committed logical prune appear failed solely because all free pages have not yet been reclaimed.

#### Scenario: Equivalent retained rows

- **WHEN** Apple and Android prune equivalent complete-catalog fixtures with the same target resolution
- **THEN** they retain identical source sequences, protected evidence, counts, policy, and next history revision

#### Scenario: Cancel during one ride

- **WHEN** cancellation is observed before a ride's logical prune commits
- **THEN** its transaction rolls back, all maintenance resources eventually release, and the prior history revision remains queryable

#### Scenario: Resume interrupted batch

- **WHEN** the process exits after one batch ride commits and before the next ride completes
- **THEN** startup uses marker revision and persisted keyset progress to avoid double-pruning, then resumes or terminates the bounded job deterministically

#### Scenario: Interrupted compaction

- **WHEN** logical pruning commits but incremental vacuum is cancelled or interrupted
- **THEN** retained history and its new revision remain valid while bounded state records reclamation pending for a later compaction attempt

#### Scenario: Pathological prune memory

- **WHEN** a ride contains far more raw rows than the target keep-set
- **THEN** native selector memory remains bounded by one bucket and field limits, temporary SQLite storage is cleaned after success/cancellation/failure, and no raw-sequence collection is retained in memory
