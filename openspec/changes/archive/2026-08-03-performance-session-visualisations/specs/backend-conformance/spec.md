## MODIFIED Requirements

### Requirement: Complete contract coverage

The canonical suite SHALL cover every mandatory WebSocket control method including standalone `session.list`, pending-delete cancel, and prune preview/start/cancel, session-data HTTP request kind, result shape, state event, stable error, authentication rule, ordering invariant, lifecycle behavior, persistence behavior, and resource bound visible to the frontend. A required scenario MUST create its prerequisite device, Board Profile, session, history revision, snapshot, cursor, prune job, or session-list page and MUST fail rather than silently skip its assertions when that state is absent. Historical assertions SHALL use `session.get` metadata plus `series`, `samples`, and `events`; `session.getDiagnostics`, embedded `SessionDetail`, and session-group assertions SHALL NOT remain.

#### Scenario: Contract operation added

- **WHEN** a new frontend-visible control method or session-data request kind is introduced
- **THEN** the change is incomplete until canonical success and relevant authentication, failure, cancellation, and bound scenarios cover it

#### Scenario: Backend drift

- **WHEN** a backend returns a structurally or behaviorally incompatible WebSocket or HTTP result
- **THEN** canonical conformance fails with the first actionable mismatch

#### Scenario: Missing scenario resource

- **WHEN** a success scenario requires a device, Board Profile, session, snapshot, cursor, or session-list page that setup did not create
- **THEN** canonical conformance fails with an actionable setup error instead of omitting the operation

#### Scenario: Standalone destructive product operations

- **WHEN** canonical conformance creates disposable standalone sessions
- **THEN** it exercises single/batch pruning and query/deletion coordination without creating or unlinking a session group

### Requirement: Detailed persistence and contract coverage

Canonical conformance SHALL verify that both backends commit, recover, publish, serialize, range-query, page, diagnose through exact retained samples, prune, and export every typed detailed field, observed discrete transition, and representative bounded extension with stable names, units, nullability, continuity, revision, and privacy. It SHALL cover dedicated stable columns, write-time field presence, authoritative sample/event and recovery-summary markers, recorded/stored counts, source evidence, catalog reconciliation, storage failure, frontend recreation, reconnect, selected-controller change, active snapshot growth, session interruption, decreasing device counters, and unknown future enum values.

#### Scenario: End-to-end detailed sample

- **WHEN** the simulator supplies a sample containing representative fields from every semantic group
- **THEN** live state, published metadata, series, exact samples, transition pages, and export agree on committed sequences and values on Apple and Android

#### Scenario: One controller per session

- **WHEN** a product selects a different discovered controller after recording detailed telemetry
- **THEN** conformance observes the old session end, route-scoped telemetry clear, and the new controller contribute only to a new session and snapshot

#### Scenario: Privacy and composed bounds

- **WHEN** detailed telemetry, scenario aliases, learned identities, oversized fields, extension limits, series field/bucket products, sample pages, event pages, and decoded HTTP bodies are exercised
- **THEN** conformance observes the 65,536-byte WebSocket, 64-field layout, 64-byte key, 4,096-byte aggregate-name, 32-extension, 16,384-byte request/extension JSON, 16-request-field, 1,024-bucket, 65,536-observation, 2,000-sample-page, 500-event-page, and 4,194,304-decoded-byte bounds with canonical controller UUIDs where identity is present and no launch token or platform Bluetooth identifier in state, history, logs, or errors

#### Scenario: Interrupted publication

- **WHEN** a test seam stops the process after the session marker commits but before catalog publication
- **THEN** restart reconciles the catalog and returns matching sample/event boundaries, counts, elapsed coverage, fields, riding/distance/maxima summaries, and query results before exposing the recovered session

#### Scenario: In-process publication failure

- **WHEN** a test seam fails the catalog update after the session transaction commits without stopping the process
- **THEN** no newer state is published, another append or metadata read first attempts reconciliation, and an unreconciled session reports a stable storage failure

## ADDED Requirements

### Requirement: Standalone session catalog conformance

The canonical suite SHALL exercise `session.list` against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify bounded first pages, deterministic `(startedAt, sessionId)` ordering, duplicate-free keyset continuation beyond 50 sessions, catalog-revision cursor conflicts, bounded complete state snapshots without recent-session or group collections, and unchanged catalog revision across telemetry-only state events. It SHALL also verify that automatic-linking settings and session-group methods, payloads, tables, and fixtures are absent.

#### Scenario: Page continuation beyond fifty sessions

- **WHEN** a disposable catalog contains more than one bounded session-list page
- **THEN** every target returns identical standalone summaries in deterministic order and reaches all rows through continuation without offset pagination or a total-history cap of 50

#### Scenario: Stale catalog cursor

- **WHEN** a catalog-visible session mutation occurs after a cursor is issued
- **THEN** every target rejects continuation with the same bounded conflict and the next first-page request succeeds

#### Scenario: Telemetry-only state change

- **WHEN** a persisted telemetry sample publishes a newer complete state snapshot without changing catalog-visible metadata
- **THEN** the snapshot remains bounded, contains no session collection or group payload, and leaves `sessionCatalogRevision` unchanged

#### Scenario: Grouping surfaces are removed

- **WHEN** conformance inspects settings, control methods, state, storage recreation, and mock fixtures
- **THEN** no automatic-linking field, session-group method, linked-ride payload, group table, or group fixture is exposed

### Requirement: Canonical session-data conformance

The canonical black-box suite SHALL exercise the same session-data HTTP requests against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify bootstrap discovery and development URL resolution, token and exact-Origin authentication before body/session processing, no-store responses, gzip decoding, explicit columnar shapes and parallel-array lengths, field/range minimization, published boundaries, stable error mappings, cursors, request/decoded-response bounds, cancellation cleanup, and the continued 65,536-byte WebSocket limit without duplicating assertions in target adapters.

#### Scenario: Cross-platform data response

- **WHEN** canonical conformance sends one valid series, samples, nearest, or events request to any supported target
- **THEN** the decoded response satisfies the same hand-written contract and immutable expected result

#### Scenario: Unauthenticated HTTP query

- **WHEN** a query has an invalid token or Origin and names an existing or missing session
- **THEN** every target returns indistinguishable rejection before body parsing/session lookup and exposes no private identifier or token

#### Scenario: Bounds and cleanup

- **WHEN** conformance exercises oversized, malformed, cancelled, concurrent, invalid-range, invalid-field, future-boundary, and cross-request-cursor cases
- **THEN** every target remains within resource/shape limits, stays cancellable without a hard correctness deadline, releases cancelled work, and remains responsive to a later valid request and durable append

#### Scenario: Decoded stream overflow

- **WHEN** the canonical client receives a gzip body that expands beyond the decoded bound
- **THEN** it aborts before JSON parsing or complete retention and reports a bounded invalid-backend failure

### Requirement: Immutable historical aggregation and pagination vectors

Conformance SHALL own immutable platform-neutral vectors containing ordered raw samples, stable and extension fields, null and time-gap continuity cases, ride phases, session-captured persistence intervals, pruning resolution, published revisions/boundaries, bounded requests, exact expected M4 segments, nearest candidates, and keyset pages. The frontend mock, Swift reducer/reader, Kotlin reducer/reader, and canonical suite SHALL consume those vectors. Vectors SHALL cover checked bucket boundaries, extrema order and deduplication, duplicate elapsed values, sequence ties, nulls at and across bucket edges, alternating gaps, configured and derived time gaps, snapshot/revision exclusion, late fields, exact-nearest ties, dense sample windows, event pages, and invalid cursor replay.

#### Scenario: Extrema order

- **WHEN** minimum and maximum observations occur in either order inside a bucket
- **THEN** every implementation emits them once in original `(elapsedMs, sequence)` order with the same expected segments

#### Scenario: Bounded alternating gaps

- **WHEN** valid and null observations alternate within one bucket and across the next bucket boundary
- **THEN** every implementation remains within four observations per bucket and emits no segment that bridges a null generation

#### Scenario: Configured and automatic time gaps

- **WHEN** the same rows are reduced with an explicit seconds threshold and with automatic phase/resolution policy
- **THEN** every implementation emits the exact expected continuity generations for each captured policy

#### Scenario: Snapshot exclusion

- **WHEN** fixture rows, fields, or events exist beyond the request's published boundaries
- **THEN** every implementation excludes them from metadata, series, samples, nearest, and event results

#### Scenario: Page continuation

- **WHEN** a fixture window exceeds one sample or event page
- **THEN** every implementation returns identical chronological, duplicate-free pages and rejects a cursor rebound to another request

### Requirement: Canonical SQLite storage and reader verification

Native verification SHALL create the shared current schema verbatim and SHALL inspect representative query plans and lifecycle behavior for both hosts. It SHALL verify dedicated stable-column persistence, bounded extension decoding, field-presence discovery after late samples, marker/catalog publication order, in-process and restart reconciliation, history/prune metadata, incremental auto-vacuum, read-only WAL snapshots, writer non-obstruction, cancellability, two-read concurrency, cancellation-release and ordinary-completion target measurements, and reader coordination with deletion/export/pruning/recovery. Series and exact query plans MUST use an elapsed index without avoidable full-range distance sorting or temporary ordering.

#### Scenario: Requested stable columns only

- **WHEN** a long complete-catalog session is queried for two stable fields
- **THEN** the checked plan and result use only the ordering/snapshot columns and those dedicated numeric columns without decoding stable `detail_json`

#### Scenario: Active writer and cancelled reader

- **WHEN** a long historical reader is cancelled while an active session append becomes due
- **THEN** the append completes within its existing persistence bound and all reader transaction, connection, lease, reducer, and compression resources are released

#### Scenario: Delete with an open lease

- **WHEN** deletion races a completed-session query
- **THEN** no session files are removed beneath an open connection and no new reader starts after pending deletion

#### Scenario: Slow cancellation

- **WHEN** a loaded reference device exceeds the cancellation-release or ordinary-completion target
- **THEN** verification records stage/resource diagnostics while requiring the operation to remain cancellable and preventing stale publication or unsafe file mutation

### Requirement: Cross-input chart and tooltip conformance

Frontend verification SHALL exercise the shared chart viewport and one-active-field exact tooltip model with mouse, touch, pen, keyboard, and assistive-technology inputs. It SHALL cover hover, tap-to-pin, synchronized group crosshair, crosshair-handle drag, background pan threshold, two-touch pinch where only one contact is primary, vertical page scrolling, pointer cancellation/lost capture, visible controls, range-slider value text, pending/exact/unavailable tooltip content, edge positioning, Escape/close dismissal, stale exact responses, and guarded activation without double action.

#### Scenario: Desktop hover and mobile touch equivalence

- **WHEN** mouse hover and touch tap target the same elapsed position in equivalent chart state
- **THEN** both paths resolve the same exact retained source sample and present equivalent elapsed, label, value, unit, and unavailable semantics

#### Scenario: Hover request coalescing

- **WHEN** rapid mouse or pen movement advances while one exact request is in flight
- **THEN** at most one request plus one latest target is retained and an older response never replaces the newest tooltip

#### Scenario: Gesture arbitration

- **WHEN** tests distinguish plot tap, background horizontal drag, crosshair-handle drag, two-touch pinch, non-primary one-pointer input, and browser-owned vertical scroll
- **THEN** each input produces only its specified selection, pan, inspection, zoom, no-op, or cancellation behavior

### Requirement: Representative long-session performance gate

Verification SHALL include a deterministic 150,000-sample complete-catalog session, a larger pathological resource-bound session, and physical checkpoints on an iPhone 12 and Samsung Galaxy S9. Each checkpoint SHALL record exact OS/WebView versions, cache state, native query/reduction/gzip time, cancellation-release time, response bytes, frontend decode/path/render time, SVG path/observation counts, and interaction long tasks. With an empty session-feature cache, metadata to first useful visible-group SVG SHALL target one second in each of at least three recorded runs. An uncached settled zoom or pan SHALL target 250 milliseconds. Current bounded paths SHALL provide gesture feedback before that settled fetch, active persistence cadence SHALL remain within its existing bound, and every measured operation SHALL remain cancellable. A target miss SHALL produce retained stage diagnostics rather than a correctness timeout.

#### Scenario: Mobile overview budget

- **WHEN** each reference device opens the deterministic long session with an empty feature cache
- **THEN** it reports whether a useful first visible SVG met the one-second target plus bounded response, cache, path, observation, and stage measurements

#### Scenario: Mobile zoom budget

- **WHEN** the target settles an uncached zoom or pan over that session
- **THEN** verification reports whether the newest requested range met the 250-millisecond target and requires that superseded responses never subsequently replace it

#### Scenario: Immediate gesture feedback

- **WHEN** the user continuously pans or pinches before a new range response is available
- **THEN** the existing bounded SVG transforms on animation frames and diagnostics expose any interaction-blocking long task

#### Scenario: Native scan target miss

- **WHEN** the dedicated-column native scan misses its allocated initial-overview budget before HTTP or UI implementation proceeds
- **THEN** the miss and stage profile are retained for review, repeated material misses trigger a persisted-aggregate design decision, and the individual run is not failed solely by elapsed time

### Requirement: Canonical pruning conformance

Conformance SHALL apply immutable pruning vectors to the mock, Swift, Kotlin, and decoded black-box targets. Vectors SHALL define elapsed-zero bucket alignment, first/last rows, first non-null field sources, per-field minimum/maximum roles, greedy maximum-coverage selection, stable field and extension ordering, nulls, multi-role rows, phase-boundary pairs, event sources, earliest-tied summary sources, the eight ordinary-row cap, aligned coarser re-pruning, recorded/stored counts, and expected history revisions. Canonical scenarios SHALL cover preview, one ride, all rides with a captured upper session key, rides completed after batch start, ineligible skips, cancellation before commit, cancellation between rides, crash recovery, stale snapshots/cursors, exports, bounded selector/temp-file resources, WAL checkpoint, and interrupted incremental compaction.

#### Scenario: Deterministic capped selection

- **WHEN** a bucket offers more than eight ordinary source candidates across many numeric fields
- **THEN** every implementation retains the same at-most-eight rows after protected evidence and reports the same uncovered roles

#### Scenario: Atomic cancellation and batch progress

- **WHEN** cancellation or process interruption occurs at each transaction and between-ride seam
- **THEN** every ride is either wholly old or wholly revised, completed batch work is not repeated, and bounded progress reaches the same recoverable state

#### Scenario: Logical prune before compaction

- **WHEN** pruning commits and incremental vacuum is interrupted
- **THEN** exact retained history, revision, events, summaries, and export disclosure remain correct while reclaimable pages stay resumable

#### Scenario: Bounded pruning resources

- **WHEN** preview and pruning process a pathological raw ride and are cancelled at selection, checkpoint, and vacuum seams
- **THEN** measured native memory remains independent of raw sequence count, temporary keep storage is removed, and actual file reclamation remains distinguishable from its preview estimate

#### Scenario: Stale consumer after prune

- **WHEN** a frontend uses a pre-prune snapshot, cursor, cache response, or export read after the history revision advances
- **THEN** every target rejects or discards it and requires refreshed metadata without mixing removed and retained rows
