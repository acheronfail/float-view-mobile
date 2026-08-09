## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `savedDevice.update`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, and `events`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Authenticated loopback transport

The backend SHALL bind only to loopback and expose `/`, `/api/bootstrap`, `/api/ws`, and the session-data URL identified by bootstrap. Bootstrap SHALL provide the WebSocket URL, session-data URL, and one random per-launch token. The WebSocket SHALL validate the token and expected Origin. Every session-data request SHALL be an Origin-bearing POST, SHALL validate the same token from `X-Float-View-Token` and the exact expected Origin before reading the bounded body or resolving a session, and SHALL return `Cache-Control: no-store` without placing the token in a URL, response, error, or log. Browser development SHALL resolve both data transports through the authenticated development origin rather than introducing cross-origin behavior.

#### Scenario: Valid frontend connection

- **WHEN** the bundled frontend uses the current bootstrap token and application Origin
- **THEN** the backend accepts its WebSocket and bounded session-data requests

#### Scenario: Invalid authentication

- **WHEN** a WebSocket or session-data request omits or supplies an incorrect token or Origin
- **THEN** the backend rejects it before body parsing or session lookup and discloses neither session existence nor private data

#### Scenario: Recreated frontend

- **WHEN** the frontend is recreated during the same native launch
- **THEN** bootstrap provides the current authenticated control-plane and session-data endpoints without exposing durable state through an unauthenticated route

### Requirement: Bounded request protocol

WebSocket control requests SHALL use `{ id, method, params }`; successes SHALL use `{ id, result }`; and failures SHALL use `{ id, error }`. Session-data POST bodies and results SHALL use the bounded discriminated shapes defined for `series`, `samples`, and `events`, with bounded errors using the same frontend-visible error vocabulary. Every accepted WebSocket request ID and every accepted HTTP request MUST receive exactly one terminal result unless its client connection is cancelled, and IDs, messages, bodies, decoded responses, queues, timeouts, retries, cursors, and collections MUST be bounded.

#### Scenario: Successful request

- **WHEN** a valid control or session-data request is processed
- **THEN** exactly one corresponding bounded result is returned

#### Scenario: Failed request

- **WHEN** a request cannot be completed
- **THEN** exactly one structured bounded error is returned without partial success data

#### Scenario: Oversized message

- **WHEN** a client exceeds the documented WebSocket, HTTP body, decoded response, field, point, page, or cursor bound
- **THEN** the backend rejects the request or connection before unbounded allocation or storage work

### Requirement: Current frontend product operations

The contract SHALL support revision-checked saved-device custom-name updates, persisted settings including nullable `historyGapThresholdSeconds`, local versioned Board Profiles, bounded cursor-paginated standalone session summaries, small session metadata retrieval, authenticated bounded historical-data queries, native export presentation, irreversible ride pruning, and deletion with platform-neutral semantics. Session grouping, automatic linking, linked-ride state, and group mutation operations SHALL NOT be part of the contract. A configured history-gap threshold SHALL be finite and from 0.1 through 3,600 seconds; null or absence SHALL select the derived default. `session.get` SHALL return the requested summary, ordered available numeric-field descriptors, event count, recorded/stored sample counts, prune metadata, and one published snapshot containing history revision, committed sample sequence, committed event sequence, and committed elapsed boundary, but SHALL NOT contain historical numeric points or transition collections. Historical overview series, exact or retained sample pages, and event pages SHALL use the session-data endpoint and a snapshot returned by `session.get`.

#### Scenario: Saved board rename

- **WHEN** the frontend supplies a valid custom display name or clears the override at the current saved-device revision
- **THEN** the backend persists it and returns saved-device state with a newer revision and newly resolved display name

#### Scenario: Session review metadata

- **WHEN** the frontend requests `session.get`
- **THEN** the backend returns bounded metadata and one published snapshot in a message no larger than 65,536 UTF-8 bytes without embedding historical series or events

#### Scenario: Session catalog page

- **WHEN** the frontend requests a valid standalone session-list page with a bounded limit and optional cursor
- **THEN** the backend returns summaries ordered by `startedAt` descending with `sessionId` descending tie-breaking, plus an opaque continuation cursor when more rows remain

#### Scenario: Grouping is absent

- **WHEN** the frontend requests state, settings, or supported control methods
- **THEN** no session-group collection, automatic-linking option, linked-ride operation, or group persistence result is exposed

#### Scenario: Historical data review

- **WHEN** the frontend submits an authenticated series, samples, or events request for the returned snapshot
- **THEN** the backend returns the corresponding bounded committed history with the same platform-neutral names, units, nullability, and sequence ordering

#### Scenario: Configure history gaps

- **WHEN** the frontend persists a valid history-gap threshold in seconds or clears it to null
- **THEN** complete application state reports that value and subsequent series requests use the configured threshold or the defined derived default without platform branching

#### Scenario: Diagnostic review

- **WHEN** the frontend submits a samples request for an exact bounded window around a valid elapsed time
- **THEN** the session-data endpoint returns exact pageable retained source samples without creating a separate logging or WebSocket method, equal to the original lossless stream only when the ride has not been pruned

#### Scenario: Native export

- **WHEN** the frontend requests a supported session export
- **THEN** the backend creates the export and invokes the host platform save or share presentation without frontend platform branching, disclosing any pruning policy and recorded-versus-stored counts

#### Scenario: Cancel pending deletion

- **WHEN** the user cancels deletion while it is still waiting for reader leases and before file removal begins
- **THEN** pending deletion clears and the unchanged session becomes readable again; if removal already won, the backend reports the stable completed-deletion result

### Requirement: Disposable current storage schema

The native backends SHALL execute the shared canonical first-production SQLite schemas, retain WAL recovery and integrity handling, and recreate all incompatible test-stage storage on schema-version mismatch rather than running migrations. The session schema SHALL use dedicated nullable columns for every stable numeric telemetry field, bounded JSON only for negotiated numeric extensions, bounded write-time numeric-field presence metadata, chronological transition rows linked to source samples, elapsed indexes, incremental auto-vacuum, session-captured riding/idle persistence intervals, history/prune metadata, and one authoritative commit marker containing committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, and the bounded riding/distance/maximum/source-sequence/last-value projection needed to recover catalog summaries. The catalog SHALL contain bounded persisted prune-job progress. Denormalized catalog metadata SHALL be reconciled from the authoritative session database after interruption or an in-process catalog failure before that session is published or appended again.

#### Scenario: Schema mismatch

- **WHEN** the canonical storage version differs from persisted pre-release test data
- **THEN** the backend deletes its logging directory and recreates the complete current schema without migration, backfill, compatibility columns, or partial fallback

#### Scenario: Interrupted session recovery

- **WHEN** the backend starts with an interrupted active session whose session marker is newer than its catalog row
- **THEN** it performs bounded integrity recovery, reconciles published boundaries and counts from the session database, and only then exposes the recovered result

#### Scenario: Catalog update fails after session commit

- **WHEN** a session transaction commits but its denormalized catalog update fails in the running process
- **THEN** the backend marks that session dirty, publishes no newer boundary, and reconciles from the marker before accepting another append or metadata read

#### Scenario: Late extension field

- **WHEN** a valid extension field first appears after the first 2,000 committed samples
- **THEN** snapshot metadata includes it through write-time field presence without scanning or sampling the ride

### Requirement: Live and historical telemetry symmetry

Every typed detailed numeric field accepted into a durable session SHALL be retrievable through session-data series and samples under the same stable name and unit used by the live sample. Before pruning, exports SHALL include the same semantic fields and bounded extensions losslessly. After pruning, exports SHALL include every retained real source row plus unchanged events and summaries and SHALL state that removed samples cannot be reconstructed. `session.get` SHALL contain metadata only. Transition events SHALL be available through chronological range pages; each event SHALL contain committed sequence, source sample sequence, elapsed time, a kind from the detailed semantic categories, and a stable string value. Boolean transitions SHALL use `active` and `cleared`, nullable reason or fault clearing SHALL use `none`, and events SHALL be appended only when an observed discrete value changes. Series reduction MUST NOT average values, fabricate device history from before Float View observed the controller, or join across an unavailable or time-gap boundary.

#### Scenario: Review a simulated warning

- **WHEN** a duty-pushback sample is persisted and the session is later opened
- **THEN** historical series, exact samples, transitions, and export identify the same duty value, setpoint-adjustment state, and alert reason shown live

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated after detailed samples were committed
- **THEN** state, metadata, and authenticated session-data requests recover authoritative native values without depending on browser memory

#### Scenario: Review a discrete transition

- **WHEN** wheelslip becomes active and later clears during a session
- **THEN** chronological event pages contain one `wheelslip=active` event and one `wheelslip=cleared` event at their committed elapsed positions

#### Scenario: More than two thousand events

- **WHEN** a session contains transitions before and after its first 2,000 events
- **THEN** keyset pagination makes every committed transition reachable while no response contains an unbounded event collection

### Requirement: Bounded telemetry serialization

Detailed live samples, extension keys and counts, metadata, session-list pages/cursors, session-data requests/results, retained sample pages, event pages, prune previews/jobs, and exports SHALL retain explicit bounds. Authenticated WebSocket messages SHALL remain at most 65,536 bytes. A negotiated field name SHALL be at most 64 UTF-8 bytes, an extension map SHALL contain at most 32 finite values, and serialized extension JSON SHALL be at most 16,384 UTF-8 bytes. A session-list page SHALL contain 1 through 100 summaries and an opaque cursor SHALL be at most 1,024 UTF-8 bytes. A session-data request body SHALL be at most 16,384 UTF-8 bytes; a decoded success body SHALL be at most 4,194,304 bytes; series and sample requests SHALL contain 1 through 16 unique fields; series SHALL contain 1 through 1,024 buckets and at most 65,536 total observations; sample pages SHALL contain at most 2,000 rows; and event pages SHALL contain at most 500 rows. Sequences, revisions, counts, and elapsed values SHALL be non-negative JavaScript-safe integers. Prune target resolution SHALL be a whole 1 through 300 seconds, and complete application state SHALL expose at most one bounded prune operation without embedding a ride-identifier collection. Canonical VESC controller UUIDs MAY appear in bounded state, history, extensions, errors, and default logs. Launch tokens and platform Bluetooth identifiers MUST NOT appear there.

#### Scenario: Oversized extension data

- **WHEN** native telemetry contains extension data beyond the field-count, name, finite-value, or encoded-byte bound
- **THEN** the backend applies the specified deterministic rejection or omission rule and emits valid bounded contract state

#### Scenario: Composed series bound

- **WHEN** a requested field and bucket combination could exceed 65,536 observations or 4,194,304 decoded bytes
- **THEN** the backend rejects it before scanning or allocating a response even when each individual field and bucket value is otherwise valid

#### Scenario: Identifier consistency inspection

- **WHEN** live state, metadata, session-data responses, exports, errors, and logs are inspected
- **THEN** learned or simulated controller UUIDs use the canonical lowercase-hex representation and no launch token or platform Bluetooth identifier is present

### Requirement: Authoritative application state

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, a separate monotonic `sessionCatalogRevision`, Bluetooth state, discovered and saved devices with resolved identity, active connection with saved-device identity and canonical controller UUIDs where meaningful, connection state, detected direct-or-CAN controller route, bounded controller candidates when selection is required, bounded connection issue when failed, firmware, active-session summary, latest successfully persisted sample when known, and settings. Recent session summaries and session groups SHALL NOT be included; standalone session summaries SHALL be retrieved through `session.list`. Historical series and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all bounded state needed to render the main application and a catalog revision with no session-summary or session-group collection

#### Scenario: Catalog revision is independent

- **WHEN** a persisted telemetry sample publishes a newer complete state snapshot without changing session catalog membership or summary visibility
- **THEN** the global state revision advances while `sessionCatalogRevision` remains unchanged

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while the board is reconnecting
- **THEN** initial authoritative state contains the current connection state, detected route when known, pending candidates or connection issue when applicable, latest sample when one exists, and the current session catalog revision

### Requirement: Complete persisted settings shape

The mandatory `Settings` value SHALL contain `revision`, `idleProbeIntervalMs`, `idlePersistIntervalMs`, `ridingPersistIntervalMs`, `postRideTailMs`, `reconnectDurationMs`, `minimumSessionDistance`, `themePreference`, and `unitPreference`. `minimumSessionDistance` SHALL be expressed in meters and default to `100`; `themePreference` SHALL accept exactly `system`, `light`, or `dark`; `unitPreference` SHALL accept exactly `metric` or `imperial` and default to `metric`; `automaticSessionLinkingEnabled`, `sessionLinkWindowMs`, and `rawPacketCaptureEnabled` MUST NOT remain in the contract or persisted settings payload.

#### Scenario: Default settings

- **WHEN** a backend creates current default settings
- **THEN** it returns the existing timing defaults with `minimumSessionDistance` equal to `100`, `themePreference` equal to `system`, `unitPreference` equal to `metric`, and no linking or raw-packet fields

#### Scenario: Settings round trip

- **WHEN** the frontend updates a valid appearance, units, timing, or distance field at the current revision
- **THEN** every supported backend persists and returns the same complete settings shape at a newer revision

#### Scenario: Removed linking option

- **WHEN** a settings patch contains `automaticSessionLinkingEnabled` or `sessionLinkWindowMs`
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Unknown settings field

- **WHEN** a settings patch contains `rawPacketCaptureEnabled` or another field outside the complete shape
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

## ADDED Requirements

### Requirement: Bounded pageable standalone session catalog

The WebSocket control contract SHALL expose `session.list` for standalone session summaries outside complete `state.changed` snapshots. The request SHALL accept a page size from 1 through 100 and an optional opaque cursor; the initial product query SHALL order by `startedAt` descending and `sessionId` descending as a deterministic tie-break. The result SHALL contain at most the requested number of summaries, the catalog revision observed by the query, and either a bounded continuation cursor or null. Cursors SHALL be versioned and bound to the query shape, catalog revision, and last `(startedAt, sessionId)` key; a cursor for another query, revision, or malformed context SHALL fail with bounded conflict or invalid-request semantics. The list SHALL contain no grouping metadata or linked-session state.

#### Scenario: First standalone page

- **WHEN** the frontend requests the first page with a valid limit
- **THEN** the backend returns the newest standalone session summaries in deterministic order and no more than the requested count

#### Scenario: Continue beyond the page bound

- **WHEN** more sessions exist after the first page
- **THEN** the result supplies a cursor and later pages reach every remaining summary without offset scanning, duplication, or a total-history limit of 50

#### Scenario: Cursor revision conflict

- **WHEN** a session is created, completed, deleted, or otherwise changes catalog visibility after a cursor is issued
- **THEN** the next request fails with a bounded conflict and the frontend must restart from the first page

#### Scenario: Telemetry does not reload the catalog

- **WHEN** a persisted telemetry sample publishes a newer `state.changed` snapshot without changing catalog-visible session metadata
- **THEN** the snapshot remains bounded and the frontend does not need to refetch or re-group the loaded session pages

#### Scenario: Grouping removal

- **WHEN** any supported backend exposes state, settings, or control methods
- **THEN** no `SessionGroup`, `sessionGroup.unlink`, automatic-linking field, linked-ride collection, or group member result is present

### Requirement: Bounded compressed session-data protocol

The hand-written contract SHALL define explicit columnar JSON request/result unions for `series`, `samples`, and `events`. Every request SHALL identify one session, one previously published snapshot including history revision, one bounded inclusive elapsed range, and only the fields, gap policy, bucket/page limits, mode, and cursor applicable to its kind. Successful results SHALL echo the session, snapshot, revision, range, and applicable gap policy; SHALL use parallel arrays with validated equal lengths; SHALL advertise standard HTTP gzip content encoding when accepted; and SHALL remain inside the composed observation, collection, and decoded-byte bounds. The frontend SHALL count the decoded response stream before parsing and SHALL reject malformed, mismatched, excessive, unsupported, or unrequested data.

#### Scenario: Compressed range query

- **WHEN** an authenticated client requests valid fields and an elapsed range with gzip accepted
- **THEN** the backend returns gzip-encoded columnar JSON whose decoded result satisfies every declared field, array, observation, and byte bound

#### Scenario: Oversized decoded stream

- **WHEN** a response stream expands beyond 4,194,304 decoded bytes or declares inconsistent parallel arrays
- **THEN** the frontend aborts and reports a bounded invalid-backend response without parsing or retaining the complete body

#### Scenario: Client cancellation

- **WHEN** the frontend aborts a superseded overview, zoom, sample, or event request
- **THEN** the backend requests native interruption, eventually releases all work and leases, and remains responsive to durable persistence and a later valid request without relying on a correctness timeout

### Requirement: Resolution-aware historical series

A `series` query SHALL select one published sample boundary and history revision, inclusive elapsed range, ordered unique field list, target bucket count, and the nullable persisted history-gap threshold captured by the frontend. For each field the backend SHALL read samples in `(elapsedMs, sequence)` order and select first, minimum, maximum, and last non-null observations per non-empty bucket, deduplicating observations that occupy multiple roles and returning them in source order. Bucket arithmetic SHALL assign each endpoint deterministically without overflow. A null or absent raw observation SHALL advance that field's continuity generation. An elapsed delta greater than a configured threshold SHALL advance every field; when unset, the edge threshold SHALL be `max(1 second, 3 × session-captured expected persistence interval for the preceding ride phase, 3 × retained history resolution)`, with zero retained-resolution contribution for unpruned history. The backend SHALL read phase internally without returning it unless requested. Returned segments SHALL join selected observations only within one unchanged generation. The result SHALL preserve finite extrema and endpoints, contain at most four observations per bucket per field, and omit every unrequested telemetry value.

#### Scenario: Four-hour overview

- **WHEN** a 150,000-sample session is queried across its full elapsed range for a 400-pixel chart
- **THEN** each requested field returns at most 1,600 ordered observations regardless of raw sample count

#### Scenario: Narrow spike and voltage sag

- **WHEN** a bucket contains a short positive spike and a short lower value between its first and last observations
- **THEN** both extrema retain their original elapsed time and sequence position rather than being replaced by an average

#### Scenario: Alternating missing observations

- **WHEN** valid and null values alternate repeatedly within one bucket
- **THEN** the response remains within four selected observations and no returned segment instructs the chart to bridge a raw null run

#### Scenario: Configured recording gap

- **WHEN** adjacent non-null rows are farther apart than the configured threshold in seconds
- **THEN** every requested field starts a new continuity segment at the later row

#### Scenario: Derived recording gap

- **WHEN** no threshold is configured and a gap is compared for a lossless or pruned ride phase
- **THEN** the backend uses the maximum of one second, three expected preceding-phase persistence intervals, and three retained-resolution intervals

#### Scenario: Field minimization

- **WHEN** a query requests two fields from a session containing the complete stable catalog and extensions
- **THEN** storage selection and the response contain only those two fields plus ordering/snapshot columns

### Requirement: Exact samples and pageable transition events

A `samples` window query SHALL return chronological exact retained source rows for requested fields in keyset pages of at most 2,000 with explicit continuation state. A nearest-sample query SHALL use indexed exact, predecessor, and successor candidates and deterministically prefer earlier elapsed time and then lower sequence on a distance tie. An `events` query SHALL return chronological committed transitions for an elapsed range in keyset pages of at most 500. Sample and event cursors SHALL be opaque, bounded, versioned, and bound to their session, history revision, snapshot, range, and last `(elapsedMs, sequence)` key so all results remain reachable without duplication or cross-request replay.

#### Scenario: Crosshair exact value

- **WHEN** the frontend requests the nearest committed sample to a crosshair elapsed time
- **THEN** the backend returns zero or one exact stored row rather than an interpolated or envelope value

#### Scenario: Dense diagnostic window

- **WHEN** a valid retained-source range contains more than 2,000 committed rows
- **THEN** the first page reports continuation and subsequent cursors reach the remaining rows without silently truncating or loading the complete range

#### Scenario: Invalid cursor replay

- **WHEN** a sample or event cursor is reused with another session, snapshot, or elapsed range
- **THEN** the backend rejects it with bounded `invalid_request` semantics before returning data

### Requirement: Irreversible bounded ride pruning

The backend SHALL support cancellable preview and execution of pruning for one completed ride or all rides completed when a batch starts. Only one preview or mutation SHALL run at once. A preview SHALL share bounded historical reader concurrency, process batch rides sequentially, and return bounded aggregate counts rather than a ride list. A prune SHALL use elapsed-zero-aligned whole-second buckets and retain only real source rows: session endpoints, the first non-null source for each recorded numeric field, both sides of observed riding-phase boundaries, event source samples, deterministic summary-maximum source samples, and at most eight additional rows per bucket selected as bucket endpoints then by deterministic maximum coverage of ordered per-field minimum/maximum roles. Equal summary maxima SHALL protect the earliest `(elapsedMs, sequence)` source. Selection SHALL stream with memory bounded by field count and one bucket and SHALL use bounded-lifecycle file-backed SQLite staging proportional to retained sequences rather than raw history. It SHALL never prune an active or deleting ride. Re-pruning SHALL require a strictly coarser integer-multiple resolution. Each ride SHALL commit or roll back atomically, preserve recorded count, available-field metadata, summaries, transitions, and elapsed coverage, update stored count/policy, and increment history revision. Batch completion SHALL be per-ride rather than all-or-nothing, SHALL be bounded by a captured upper session key, and SHALL exclude rides completed after the batch started. Cancellation observed after a ride commits SHALL reconcile and count that ride before stopping.

#### Scenario: Preview one ride

- **WHEN** a valid completed ride and 1-through-300-second target resolution are previewed
- **THEN** the backend returns a cancellable read-only estimate of retained rows and approximate reclaimable bytes without changing history revision or storage

#### Scenario: Prune one ride

- **WHEN** pruning a completed ride commits successfully
- **THEN** only deterministically retained source rows remain, protected evidence and original summaries/events remain reachable, stored count decreases or remains equal, and all older snapshots conflict with the newer history revision

#### Scenario: Batch cancellation

- **WHEN** the user cancels an all-rides prune after some rides commit and while another ride is being processed
- **THEN** completed rides remain pruned, the current ride rolls back unless already committed, later rides remain unchanged, and bounded application state reports the terminal progress

#### Scenario: Invalid re-prune

- **WHEN** a previously pruned ride is requested at an equal, finer, non-multiple, or misaligned resolution
- **THEN** a single-ride request is rejected and a batch request reports that ride as skipped without changing its revision

#### Scenario: Pruned export

- **WHEN** a pruned ride is exported or exact samples are queried
- **THEN** only retained exact source rows are returned and pruning policy plus recorded-versus-stored counts disclose that the original lossless stream is unavailable

### Requirement: Stable published session snapshots

The per-session database SHALL durably identify the history revision, latest committed sample sequence, event sequence, recorded/stored sample counts, event count, and elapsed boundary. `session.get` SHALL expose only a boundary that has also converged into the catalog and authoritative application state. Every session-data response SHALL echo the requested revision and boundary. Older published active-session boundaries SHALL remain queryable after newer commits within the same revision, later rows SHALL be excluded by sequence, and a future, stale-revision, or impossible boundary SHALL fail without mixing snapshots. Native persistence SHALL remain authoritative while the frontend is absent.

#### Scenario: Query while persistence continues

- **WHEN** an active session commits additional samples while the frontend queries an earlier published boundary
- **THEN** every result is internally consistent through that earlier boundary and logging continues without waiting for the historical scan

#### Scenario: Crash between databases

- **WHEN** the session database commits a newer marker but the process stops before the catalog update
- **THEN** startup reconciles the catalog from that marker before publishing recovered metadata

#### Scenario: Refresh growing history

- **WHEN** a later metadata request reports a newer published boundary
- **THEN** the same data protocol can query the expanded active history without changing request or rendering semantics

#### Scenario: Refresh pruned history

- **WHEN** metadata reports a newer history revision after pruning
- **THEN** prior cursors and snapshots conflict and refreshed requests observe one internally consistent retained source history
