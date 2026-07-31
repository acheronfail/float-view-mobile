## MODIFIED Requirements

### Requirement: Authenticated loopback transport
The backend SHALL bind only to loopback and expose `/`, `/api/bootstrap`, `/api/ws`, and the session-data URL identified by bootstrap. Bootstrap SHALL provide the WebSocket URL, session-data URL, and one random per-launch token. The WebSocket SHALL validate the token and expected Origin. Every session-data request SHALL be an Origin-bearing POST, SHALL validate the same token from a request header and the expected Origin before reading the bounded body, and SHALL return `Cache-Control: no-store` without placing the token in a URL, response, error, or log.

#### Scenario: Valid frontend connection
- **WHEN** the bundled frontend uses the current bootstrap token and application Origin
- **THEN** the backend accepts its WebSocket and bounded session-data requests

#### Scenario: Invalid authentication
- **WHEN** a WebSocket or session-data request omits or supplies an incorrect token or Origin
- **THEN** the backend rejects it without querying a session or disclosing whether that session exists

#### Scenario: Recreated frontend
- **WHEN** the frontend is recreated during the same native launch
- **THEN** bootstrap provides the current authenticated control-plane and session-data endpoints without exposing durable state through an unauthenticated route

### Requirement: Current frontend product operations
The contract SHALL support revision-checked saved-device custom-name updates, persisted settings, local versioned Board Profiles, small session metadata retrieval, authenticated bounded historical-data queries, native export presentation, deletion, and session-group unlinking with platform-neutral semantics. `session.get` SHALL return the requested summary, available numeric fields, event count, and committed sample and event sequence boundaries, but SHALL NOT contain historical numeric points or transition-event collections. Historical overview series, exact sample windows, and event pages SHALL use the session-data endpoint and the snapshot boundaries returned by `session.get`.

#### Scenario: Saved board rename
- **WHEN** the frontend supplies a valid custom display name or clears the override at the current saved-device revision
- **THEN** the backend persists it and returns saved-device state with a newer revision and newly resolved display name

#### Scenario: Session review metadata
- **WHEN** the frontend requests `session.get`
- **THEN** the backend returns bounded metadata and committed snapshot boundaries in a message no larger than 65,536 UTF-8 bytes without embedding historical series or events

#### Scenario: Historical data review
- **WHEN** the frontend submits an authenticated series, exact-sample, or event query for the returned snapshot
- **THEN** the backend returns the corresponding bounded committed historical data with the same platform-neutral names, units, nullability, and sequence ordering

#### Scenario: Diagnostic review
- **WHEN** the frontend requests an exact bounded window around a valid elapsed time
- **THEN** the session-data endpoint returns lossless committed samples without creating a separate logging path

#### Scenario: Native export
- **WHEN** the frontend requests a supported session export
- **THEN** the backend creates the export and invokes the host platform save or share presentation without frontend platform branching

## ADDED Requirements

### Requirement: Bounded compressed session-data protocol
The hand-written contract SHALL define one discriminated POST request protocol for `series`, `samples`, and `events` reads. Every request SHALL identify one session and explicit committed sequence boundary, and SHALL contain only bounded elapsed ranges, allowlisted fields, bucket or page limits, and cursors applicable to its kind. Request bodies SHALL be at most 16,384 UTF-8 bytes. Successful bodies SHALL be columnar JSON, SHALL advertise standard HTTP gzip content encoding when the client accepts it, and SHALL be at most 4,194,304 bytes after decompression. The backend SHALL reject invalid, stale, excessive, unsupported, or cross-session parameters with bounded HTTP errors and SHALL bound concurrent work, temporary buffers, query duration, and output collections.

#### Scenario: Compressed range query
- **WHEN** an authenticated client requests valid fields and an elapsed range with gzip accepted
- **THEN** the backend returns valid gzip-encoded columnar JSON whose decoded result stays within every declared field, point, and byte bound

#### Scenario: Oversized request
- **WHEN** a request body, field collection, elapsed range, bucket count, page limit, or cursor exceeds its contract bound
- **THEN** the backend rejects it before an unbounded query or response allocation

#### Scenario: Client cancellation
- **WHEN** the frontend aborts a superseded overview or zoom request
- **THEN** the backend stops or discards its work within a bounded interval and does not delay durable session persistence

#### Scenario: WebSocket remains small
- **WHEN** a long session contains telemetry or events that would exceed one control-plane message
- **THEN** WebSocket messages remain bounded to 65,536 UTF-8 bytes and bulk history is available only through the session-data protocol

### Requirement: Resolution-aware historical series
A `series` query SHALL select one bounded committed snapshot, elapsed range, field list, and target bucket count. For each requested numeric field the backend SHALL read samples in `(elapsedMs, sequence)` order and return ordered first, minimum, maximum, and last non-null observations per non-empty bucket, deduplicating observations that occupy more than one role. The result SHALL retain each selected observation's real elapsed time, split segments across null or missing runs, contain at most four observations per bucket per field, preserve finite extrema and endpoints without averaging or fabricating values, and omit unrequested telemetry.

#### Scenario: Four-hour overview
- **WHEN** a 150,000-sample session is queried across its full elapsed range for a 400-pixel chart
- **THEN** each requested field returns at most 1,600 ordered observations regardless of the raw sample count

#### Scenario: Narrow spike and voltage sag
- **WHEN** a bucket contains a short positive spike and a short lower value between its first and last observations
- **THEN** both extrema retain their original elapsed times in source order rather than being replaced by an average

#### Scenario: Missing interval
- **WHEN** a requested field is null or absent across part of the range
- **THEN** the response represents separate segments and the consumer is not instructed to draw a continuous fabricated line across the gap

#### Scenario: Field minimization
- **WHEN** a query requests two fields from a session containing the complete stable catalog and extensions
- **THEN** the response contains only those two fields and no full unrequested telemetry records

### Requirement: Exact samples and pageable transition events
A `samples` query SHALL return at most 2,000 lossless committed samples for requested fields within a bounded elapsed window, ordered by `(elapsedMs, sequence)`. An exact-nearest query SHALL deterministically select the nearest committed sample to an elapsed time, preferring the earlier elapsed time and then lower sequence on a tie. An `events` query SHALL return chronological committed transition events for an elapsed range in pages of at most 500, with an opaque bounded cursor and explicit continuation state, so every event in a long session remains reachable without one oversized newest-only response.

#### Scenario: Crosshair exact value
- **WHEN** the frontend requests the nearest committed sample to its crosshair elapsed time
- **THEN** the backend returns the deterministic exact stored values rather than values interpolated from the overview envelope

#### Scenario: Lossless diagnostic window
- **WHEN** a valid samples request selects a bounded range containing no more than 2,000 committed samples
- **THEN** the backend returns those requested values losslessly in stable order

#### Scenario: More than two thousand events
- **WHEN** a session contains transitions before and after the first 2,000 event-page results
- **THEN** chronological cursor pagination makes every committed transition reachable while each response remains bounded

### Requirement: Stable active-session snapshots
Session metadata and every session-data response SHALL identify the committed sample sequence and committed event sequence through which it was read. A query for an active session SHALL never include later partially committed data or mix snapshot boundaries, and native persistence SHALL remain authoritative while the frontend is absent. A later metadata request MAY expose newer boundaries without invalidating an already returned snapshot.

#### Scenario: Query while persistence continues
- **WHEN** an active session commits additional samples while the frontend queries an earlier returned boundary
- **THEN** every result is internally consistent through that earlier boundary and logging continues without waiting for the WebView

#### Scenario: Refresh growing history
- **WHEN** a later metadata request reports newer committed boundaries
- **THEN** the same data protocol can query the expanded active history without changing its request or response semantics

