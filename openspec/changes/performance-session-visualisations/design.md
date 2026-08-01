## Context

Detailed session review currently combines up to 2,000 wide numeric records and 2,000 transitions in one WebSocket response even though authenticated control messages are limited to 65,536 UTF-8 bytes. A representative response is measured in megabytes, and raising the frame limit would not address allocation, parsing, SVG geometry, or a four-hour ride containing about 150,000 persisted samples.

The existing session schema stores only core query fields as columns; most stable numeric telemetry is repeated in `detail_json`, extensions are stored in `extension_json`, and the two hosts discover extensions differently. A disposable 150,000-row desktop comparison of the current 28 JSON-path aggregation against equivalent dedicated columns took approximately 1.20 seconds versus 0.40 seconds before native reduction, HTTP, gzip, frontend parsing, or rendering. The elapsed index already supports the proposed range/order scan, so the first ship-ready schema must correct the row layout rather than defer it behind the UI benchmark.

The current per-session `commit_marker` records only the committed sample sequence. Samples and transitions commit in the session database before counts and summaries update in the catalog database, so process interruption can leave denormalized catalog metadata behind durable history. Long historical scans also run while holding each store's broad synchronization boundary. These facts must be corrected before active-session snapshots can be considered stable or non-obstructing.

The repository requires one disposable shared Svelte frontend, hand-written platform-neutral contracts, native storage ownership, loopback-only authenticated servers, bounded resources, mirrored Swift and Kotlin implementations, and canonical black-box conformance. Stored data is still disposable test data: this change defines the canonical first production schema, bumps the storage marker, recreates incompatible local databases, and introduces no migration or compatibility layer.

## Goals / Non-Goals

**Goals:**

- Target a useful first visible chart group for a 150,000-sample complete-catalog session within one second on the iPhone 12 and Samsung Galaxy S9 baselines.
- Provide immediate bounded SVG feedback during horizontal pan and pinch zoom, then target an uncached settled range within 250 milliseconds without imposing a correctness timeout.
- Provide exact retained-source-value crosshair tooltips through mouse hover, touch or pen contact and drag, keyboard controls, and assistive technology.
- Preserve finite extrema, endpoints, ordering, nullability, and configurable honest continuity breaks while bounding geometry by requested fields and CSS-pixel resolution rather than raw sample count.
- Make the per-session database authoritative for published sample/event boundaries, counts, elapsed coverage, available fields, and the bounded summary projection needed to recover catalog denormalization.
- Allow a user to irreversibly reduce stored resolution for one completed ride or all completed rides while retaining real source samples, semantic evidence, original summaries, deterministic revision behavior, and cancellable progress.
- Keep WebSocket control messages at 65,536 bytes and move bounded immutable or snapshot-constrained history to authenticated gzip HTTP responses.
- Keep logging, historical reads, cancellation, recovery, deletion, and WebView recreation safe without a shared cross-language runtime.

**Non-Goals:**

- Retaining the complete lossless ride in WebView memory.
- Preserving or migrating pre-release SQLite files; the optimized layout becomes the first supported schema.
- Adding the live-screen entry point for active-session history now.
- Sharing native query code through Rust, C, Kotlin Multiplatform, generated models, or another backend process.
- Adding GPS, maps, cross-session comparison, derived analytics, BMS history, or recorder commands.
- Adding a persisted aggregation pyramid unless the early native storage checkpoint proves that the indexed column scan cannot meet the target.
- Adding frontend prune controls or choosing their eventual placement and copy; only backend operations and bounded state are defined now.
- Changing Bluetooth polling, persistence cadence, or controller protocol behavior.

## Decisions

### 1. Keep WebSocket control separate from bounded HTTP history

`/api/ws` remains the state-event and mutation transport with `MAX_MESSAGE_BYTES = 65_536`. `session.get` returns only the summary, ordered available-field descriptors, event count, and one published snapshot containing sample sequence, event sequence, and committed elapsed boundary. It embeds no series or events. The obsolete `session.getDiagnostics` method is deleted; its exact-window behavior becomes a `samples` data request.

Bootstrap returns the WebSocket URL, session-data URL, and one per-launch token. Production uses the same loopback origin; browser development resolves or rebases both URLs through the existing Vite origin. The frontend sends discriminated JSON POST bodies and the token in `X-Float-View-Token`. The backend validates the exact `Origin` and token before reading the body or resolving a session, binds only to loopback, returns `Cache-Control: no-store`, and never places the token or complete telemetry in a URL, error, or log.

One route dispatches `series`, `samples`, and `events` requests so authentication, body limits, errors, cancellation, and cross-platform dispatch have one owner. Moving all contract traffic to HTTP was rejected because complete monotonic `state.changed` snapshots and correlated mutations already fit the WebSocket control plane. Raising the WebSocket limit was rejected because it weakens the established bound without solving historical allocation.

### 2. Make the current SQLite schema the optimized first production schema

Every stable numeric telemetry field receives a dedicated nullable `REAL` column in `sample`; `detail_json` is removed. `extension_json` remains only for at most 32 negotiated finite numeric extensions. This wide row avoids repeated JSON keys on disk and lets a validated field-to-column map select only required values. When extensions are requested, each row's extension object is parsed at most once by the native adapter rather than invoking one SQL JSON extraction per field.

A bounded `session_numeric_field` table records field name, stable-or-extension source, and first committed sample sequence. The writer inserts or ignores presence rows in the same transaction as the first non-null value, so metadata can select fields through a snapshot boundary without scanning samples. Results use canonical stable-catalog order followed by lexicographically ordered extensions; stable descriptors carry contract units and extension units remain explicitly unknown/source-defined.

The singleton commit marker stores committed sample sequence, committed event sequence, recorded sample count, currently stored sample count, committed event count, committed elapsed boundary, history revision, commit time, and the bounded session-summary projection needed by the catalog: riding elapsed, distance, maximum speed, maximum duty, the source sequences proving those maxima, and last persisted speed/state needed to continue those aggregates. Maximum ties retain the earliest `(elapsedMs, sequence)` source so protected evidence is deterministic. The marker exists from session creation with zero boundaries and is updated in the same transaction as sample rows, transition rows, and field presence. Each transition also records its source sample sequence. Session metadata captures the positive bounded expected riding and idle persistence intervals in integer milliseconds at creation, plus nullable prune time, retained resolution in whole seconds, and the applied policy. `history_revision` starts at zero and changes only when retained history changes; normal appends advance boundaries without invalidating older issued boundaries.

The first schema enables SQLite incremental auto-vacuum before creating tables so later logical pruning can reclaim file pages without a blocking full `VACUUM`. The test-stage storage marker changes and both hosts recreate their logging directory. No migrations, legacy readers, backfill, or partial-column fallback are added.

The existing elapsed indexes are retained initially because `sequence INTEGER PRIMARY KEY` is the SQLite rowid tie-breaker carried by those indexes. Representative `EXPLAIN QUERY PLAN` checks must prove that series, keyset pages, and predecessor/successor nearest-sample seeks use an elapsed index without a full scan or temporary sort. A non-duplicative index may be added only when those checked plans demonstrate a need.

### 3. Publish snapshots only after durable metadata converges

An append holds the existing short writer synchronization while it:

1. commits samples, transitions, field presence, and the authoritative marker in the session database;
2. updates denormalized catalog counts and summaries;
3. clears any dirty-reconciliation state, advances the in-memory published boundary; and
4. emits the newer authoritative application state.

Historical metadata may briefly take the same synchronization boundary to resolve the file and copy the catalog summary plus published boundary, but no range scan retains it. A boundary does not become visible merely because the session database transaction committed. If the catalog update fails after a session transaction commits, the in-memory session is marked dirty, no boundary or state is published, and the marker is reconciled into the catalog before another append or metadata read proceeds. Failure to reconcile is a visible storage failure rather than permission to continue with divergent metadata. If the process stops in that interval, startup recovery reads the authoritative marker, performs the existing bounded integrity checks, reconciles catalog boundaries/counts/summaries, and only then exposes the recovered session.

Completed sessions have one final published boundary for each history revision. For an active session, every request supplies a boundary previously returned by metadata. Older issued boundaries remain valid after newer commits in the same revision; a future, negative, mismatched, stale-revision, or otherwise impossible boundary is rejected. SQL always filters the applicable sequence as well as elapsed range, so later commits cannot leak into an older response. Pruning increments the history revision and intentionally invalidates all earlier snapshots and cursors because their source rows may no longer exist.

### 4. Use dedicated cancellable WAL readers and per-session leases

After resolving an allowed session under the short store synchronization, each historical request obtains a per-session read lease and a dedicated read-only/query-only SQLite connection. It begins a deferred WAL read transaction, validates the marker and request boundary, performs its indexed statements within that snapshot, then finalizes the cursor, rolls back or commits the read transaction, closes the connection, and releases the lease on every exit path.

At most two historical reads run per backend and at most one superseded interactive range request per session remains active. Native query work runs off the loopback event executor and never on the application-service/session-writer path. Apple cancellation uses a dedicated connection plus `sqlite3_interrupt`/progress checks; Android uses a dedicated database plus `CancellationSignal`/coroutine cancellation. Cancellation is available throughout query, reduction, serialization, pruning preview, logical pruning, and incremental compaction. Cancelled work releases reducer and compression buffers as well as SQLite resources; cancellation of a write transaction rolls that ride back.

The engineering targets are to release a cancelled reader or pruning step within 250 milliseconds and to complete ordinary reads and deletion coordination within two seconds on the reference devices. They are measured diagnostics, not hard-coded correctness deadlines: a heavily loaded device may continue longer while progress remains bounded and the operation remains cancellable. A target miss is recorded with stage timings and resource state rather than converted automatically into an `unavailable` result.

Deletion first marks the catalog row pending so no new lease can start, requests cancellation from existing leases, and removes the database/WAL/SHM files and catalog row only after every lease releases. `session.delete.cancel` can win only before irreversible file removal; it clears pending state and makes the unchanged session readable again. Once removal wins the race, deletion completes and cancellation reports the stable terminal result. If reader release is delayed, deletion stays visibly pending and user-cancellable instead of deleting beneath an open connection or relying on a correctness timeout. Export, pruning, compaction, and recovery use the same per-session lease/maintenance boundary where their file operations could overlap a reader. This replaces a broad lock around a long scan without creating a file-deletion race or allowing an abandoned reader to pin WAL silently.

### 5. Compose protocol limits and use explicit columnar shapes

Request bodies are at most 16,384 UTF-8 bytes. A series or sample request contains 1 through 16 unique allowlisted fields. Series requests contain 1 through 1,024 target buckets and may return at most 65,536 observations across all fields. Sample pages contain at most 2,000 rows; event pages contain at most 500 rows. Sequences, revisions, and elapsed values are non-negative JavaScript-safe integers. Successful decoded bodies are at most 4,194,304 bytes, and all arrays must have the lengths declared by their response kind.

Series results contain the echoed session/snapshot/range and an array of `{ field, segments }`; each segment contains parallel `elapsedMs` and `values` arrays. Samples contain parallel `sequence` and `elapsedMs` arrays plus `{ field, values }` columns, an optional bounded continuation cursor, and explicit `hasMore`. Nearest-sample mode returns zero or one row in the same sample shape. Events contain parallel sequence, elapsed, kind, and value columns plus cursor/continuation state. Field names are values rather than dynamic JSON object properties.

Sample and event cursors encode a version, session, history revision, snapshot, original range, and last `(elapsedMs, sequence)` key. They are opaque to the frontend and strictly validated so a cursor cannot be replayed across a session, revision, snapshot, or range. Pagination is chronological and duplicate-free. The frontend reads `Response.body` incrementally, counts decoded bytes before `JSON.parse`, then validates the discriminated shape; it does not call an unbounded convenience `response.json()`.

Authentication failures use HTTP 403 before session lookup. Invalid shapes, fields, ranges, bounds, and cursors map to the existing `invalid_request` semantics; missing/deleting sessions map to `not_found`; impossible or stale-revision snapshot boundaries map to `conflict`; bounded concurrency or temporarily unavailable maintenance state maps to `unavailable`; unexpected failures map to `internal`. Error bodies are themselves bounded and disclose no session existence before authentication.

### 6. Reduce ordered rows with a bounded continuity-aware M4 envelope

For an inclusive integer elapsed range, buckets use checked arithmetic equivalent to `min(bucketCount - 1, (elapsed - start) * bucketCount / (end - start + 1))`. This assigns the final endpoint to the final bucket without overflow or a fencepost bucket. Rows are streamed in `(monotonic_elapsed_ms, sequence)` order.

For each field and bucket, the reducer selects the first, minimum, maximum, and last non-null source observations. Observations occupying multiple roles appear once and selected observations are emitted in original order. A raw null or missing observation advances that field's continuity generation. A time discontinuity also advances every field's generation when the elapsed delta exceeds the selected gap policy. `settings.historyGapThresholdSeconds` is a persisted nullable finite number from 0.1 through 3,600 seconds. When set, it is the threshold for every edge. When unset, each edge derives `max(1 second, 3 × session-captured expected persistence interval for the preceding ride phase, 3 × retained history resolution)`; unpruned history contributes zero for the resolution term, and the reducer selects `ride_state` internally to determine the phase even when it is not a returned field. Arithmetic converts seconds to checked integer milliseconds only at the query boundary.

Each selected observation retains its generation, and the response joins adjacent selected observations only while the generation remains equal, including across bucket boundaries. Thus alternating valid/null data cannot create more than four observations per bucket, and neither an unavailable run nor a recording outage is drawn as measured continuity. The series request captures the configured nullable threshold; cache keys and returned policy echo include it so a settings change cannot reuse geometry computed under another policy.

The reducer is a pure bounded kernel implemented once in Swift and once in Kotlin. Immutable vectors cover checked bucket edges, extrema in both orders, role deduplication, duplicate elapsed values, sequence ties, nulls before/inside/after a bucket, alternating gaps, cross-bucket continuity, stable columns, extensions, and snapshot exclusion. Average, maximum-absolute, and LTTB-only reducers were rejected because they can discard physically significant minima or maxima.

### 7. Keep overview, exact samples, and transitions separate

`series` provides display-resolution envelopes. `samples` provides keyset-pageable exact retained-source windows and a nearest mode for tooltips; those windows equal the original lossless stream only before pruning. Nearest lookup performs indexed exact/predecessor/successor seeks and compares candidates in native code; it never orders a full range by `ABS(elapsed - target)`. Distance ties prefer earlier elapsed time and then lower sequence. If no sample exists at the published boundary, the result is explicitly empty.

`events` provides chronological range pages. This removes the newest-only 2,000-event truncation and makes early transitions reachable without loading them all. Export remains complete over the currently retained source history and explicitly identifies a pruned ride; the UI does not automatically page an entire ride into memory.

### 8. Progressively render bounded native SVG paths

The session feature owns a route-scoped historical controller; the backend owner retains only bootstrap/transport concerns. The controller loads metadata first, waits for a positive measured width, then requests the first visible telemetry group at integer CSS-pixel resolution. One reusable `IntersectionObserver` boundary prefetches nearby groups and loads later groups for the current shared viewport. Stable groups contain at most nine fields; extension fields are chunked into groups of at most eight, remaining within the 16-field protocol limit.

Cache keys include session, history revision, snapshot, gap policy, ordered fields, elapsed range, and bucket count. An LRU is bounded by both 32 entries and 8 MiB of decoded response estimates; snapshot or history-revision replacement, gap-policy change, or route disposal aborts work and releases obsolete cache entries. Metadata, each series group, exact inspection, and event pages have independent loading/error state rather than one global busy flag. Resizes are quantized to changed CSS-pixel bucket counts and coalesced so observer noise does not refetch the same geometry.

Each field renders one SVG `<path>` per returned continuity segment, never one DOM node per observation. Path strings are built with bounded loops. The x-axis shows shared elapsed ticks and the visible range. Each field derives a finite y-domain from the visible M4 extrema, applies deterministic padding for a constant series, and shows a zero reference when zero lies in the domain. The prior domain is held during gesture preview and replaced only with the newest matching response, avoiding axis flicker from stale data.

### 9. Use one explicit gesture and exact-tooltip state machine

The plot uses Pointer Events and `touch-action: pan-y`. A primary pointer tap on plot background selects and pins a crosshair. Horizontal background movement beyond a small threshold becomes pan. Movement beginning on the visible crosshair handle drags inspection instead. Two active touch pointers, regardless of `isPrimary`, enter midpoint-preserving pinch zoom; a non-primary pointer cannot initiate a one-pointer action. Pointer capture, `pointercancel`, lost capture, route disposal, and a browser-owned vertical scroll all clear transient gesture state.

During pan or pinch, the current bounded paths receive an animation-frame transform/clip preview with the viewport clamped to the session and a minimum one-millisecond span. No series request is issued for each move. After interaction settles, the controller aborts the previous generation and fetches the refined range; only a response matching session, snapshot, range, fields, buckets, and newest generation may replace paths.

Mouse and hovering pen movement shows a crosshair tooltip at the nearest elapsed position. Touch and contact pen input shows the same tooltip on tap, allows it to remain pinned, and updates it while the crosshair handle is dragged. One field is active at a time, while the shared crosshair synchronizes that elapsed position across the group. At most one nearest-sample request is in flight per group with one coalesced latest target; the returned retained source row populates exact values for the group cache, while the tooltip presents the active field. While exact data is pending the UI says so and never labels envelope coordinates as exact. The tooltip stays within the visible viewport, does not intercept plot pointers, shows elapsed time, field label, exact retained value/unit or unavailable state, and dismisses on pointer leave when unpinned or through an explicit close/Escape action when pinned.

Visible native buttons provide zoom in/out, pan earlier/later, and reset without gesture dependence. A native range input or equivalently conformant slider exposes crosshair position to keyboard and assistive technology with `aria-valuetext`; arrows move inspection, Home/End select viewport bounds, and the exact tooltip content is associated without flooding an assertive live region. Guarded pointer/click activation prevents double actions. This is one semantic inspection model across desktop mouse, touch clients, pen, keyboard, and screen readers.

### 10. Prune completed rides with deterministic source-row retention

The control contract adds `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`. Preview and start accept either one session identifier or `all`, plus a whole-second target resolution from 1 through 300. Only completed, non-deleting sessions are eligible. A ride that is already pruned accepts only a strictly coarser target that is an integer multiple of its current resolution, with buckets aligned from elapsed zero; a finer, equal, or misaligned rewrite is rejected or reported as skipped in batch results. Preview is read-only and cancellable, uses the same global reader concurrency and per-session leases as history, scans batch rides sequentially, reports exact aggregate retained-row estimates under the selector plus approximate reclaimable bytes, and never returns an unbounded per-ride list or promises the exact result of page-level compaction.

Pruning never averages, interpolates, or synthesizes telemetry. It always retains the first and last session samples, the first non-null source sample referenced by every `session_numeric_field` row, both source samples at riding/idle phase boundaries when present, every sample referenced by a transition event, and the source samples supporting persisted summary maxima. Field protection is globally bounded by the stable-catalog plus extension-field limits and keeps available-field metadata truthful. These protected rows are outside the ordinary per-bucket cap. Within each target bucket, pruning retains at most eight additional real source rows: bucket first and last, then a deterministic greedy choice that covers the greatest number of still-unrepresented per-field minimum/maximum roles across the ordered available-field catalog. Ties use the earliest uncovered catalog role and then `(elapsedMs, sequence)` source order. Immutable cross-platform vectors define role calculation, multi-role rows, nulls, extensions, protected rows, and ties.

The selector streams ordered source rows once and retains only the previous row, current bucket roles, ordered field extrema, and eight chosen ordinary candidates in native memory. It inserts deduplicated protected and chosen sequences into an indexed connection-local SQLite keep-set whose temp storage is file-backed and proportional to retained rows rather than the raw ride. Preview counts that keep-set in a read transaction; execution uses the same algorithm under exclusive maintenance and deletes rows not in the keep-set inside the atomic session transaction. Cancellation and failure close statements and remove temporary storage. Neither host builds an in-memory set of every raw or retained sequence.

Each ride is a separate atomic session-database maintenance transaction. The implementation constructs the retained sequence set, deletes only unretained samples, updates stored count/resolution/prune policy, increments `history_revision`, and commits the authoritative marker while preserving original recorded count, summaries, events, and elapsed coverage. The catalog then reconciles from that marker before the new revision publishes, using the same dirty-session rule as an append. Cancellation or failure before the session commit rolls that ride back. Events remain intact and continue to reference protected samples. Exports contain all retained source rows and unchanged transitions/summaries, identify the applied prune policy and recorded-versus-stored counts, and explicitly state that the export is not the original lossless sample stream.

One native pruning operation, including preview, may exist at a time. A persisted bounded catalog job record for mutation contains scope, policy, captured upper session key, keyset progress, aggregate counts, current session, cancellation request, and terminal error summary rather than an unbounded identifier list. Batch work visits sessions that were completed at job start sequentially in stable keyset order, skips ineligible or already-coarser rides, and publishes bounded progress through complete monotonic application state; rides completed later wait for another batch. Cancellation before the current session transaction commits rolls it back; cancellation after commit finishes marker-to-catalog reconciliation, counts that ride complete, and then terminates before the next ride. On restart, transaction recovery plus each ride's marker/history revision determines whether the current ride committed, then the job safely resumes or reaches its recorded terminal state.

Pruning first obtains an exclusive per-session maintenance lease, rejects new readers, and cancels or waits for existing readers to release; active recording is never pruned. After the logical transaction and catalog reconciliation publish the new revision, the maintenance connection performs a cancellable WAL checkpoint and bounded `incremental_vacuum` steps with progress checks so committed deletions reach the main file and free pages can be truncated without a full-file rewrite. A checkpoint or compaction interruption does not undo the already-atomic logical prune: the job records reclamation pending and can resume later. Reported reclaimed bytes come from measured files and remain distinct from preview estimates. Old snapshots, cursors, cached series, pending exact requests, and export reads conflict on the new history revision and must refresh metadata.

### 11. Verify storage, contract, interaction, pruning, and performance in that order

The implementation begins with the canonical schema, publication marker, immutable reducer and pruning vectors, and a native 150,000-row benchmark. Query plans, exact seeks, field discovery, recovery reconciliation, cancellation, deletion/pruning leases, and writer non-obstruction must pass before HTTP or UI work proceeds. A dedicated-column scan that misses its target is profiled and recorded; repeated material misses trigger design review for a persisted pyramid, but one slow run or a loaded device does not fail correctness or introduce a hard query timeout.

Canonical black-box conformance then exercises bootstrap, HTTP authentication, explicit shapes, composed bounds, gzip, pagination, active/older/recovered snapshots, cancellation, concurrency, deletion races, and post-failure recovery. A larger pathological fixture verifies response/resource bounds remain independent of raw row count even when it is not held to the interactive latency target.

The physical checkpoint uses an iPhone 12 and Samsung Galaxy S9 and records their exact OS/WebView versions, response sizes, SVG path/observation counts, cache state, cancellation-release timing, and stage timings. Across at least three runs with an empty feature cache, metadata-to-first-visible-useful-SVG targets one second and an uncached settled viewport targets 250 milliseconds. Target misses are retained as diagnostic findings with their stage breakdown; they do not impose a correctness timeout or make otherwise bounded, cancellable results invalid. Superseded responses must never win, active persistence cadence must remain within its existing bound, and operations must remain cancellable under load. Gesture tests additionally verify animation-frame preview and report frontend long tasks rather than treating a fast final fetch as sufficient interaction quality.

## Risks / Trade-offs

- [Wide stable rows require synchronized schema mappings] → Make the shared current SQL canonical, delete `detail_json`, map every stable contract field explicitly in both writers/readers/exports, and verify complete-catalog round trips.
- [Session and catalog databases cannot commit atomically] → Treat the session marker as durable authority, publish only after catalog convergence under the writer boundary, and reconcile after interruption before exposure.
- [Long WAL readers can retain pages and grow the WAL] → Use dedicated short read transactions, continuous cancellation, bounded concurrency, visible pending state, cancellation-release targets, and lease/resource tests during active commits.
- [M4 can return four times the requested buckets per field] → Compose the 16-field, 1,024-bucket, 65,536-observation, and 4 MiB limits before querying and deduplicate roles.
- [Progressive groups rescan the elapsed range] → Keep groups aligned with visible UI, prefetch only nearby groups, cache bounded decoded results, and measure the dedicated-column scan before considering persisted aggregates.
- [Exact hover can create request churn] → Permit one in-flight nearest lookup plus one coalesced latest target, use indexed predecessor/successor seeks, and distinguish pending from exact values.
- [Pruning is irreversible and a multi-field cap cannot preserve every field's extrema] → Require preview and explicit start, retain only real rows with protected semantic evidence, use a deterministic eight-row selector, preserve original summaries/counts, disclose loss in exports, and allow only aligned coarser re-pruning.
- [Logical deletion may not immediately reduce the database file] → Enable incremental auto-vacuum in the first schema, separate atomic logical completion from resumable compaction state, and report estimated versus reclaimed bytes honestly.
- [Pruning invalidates outstanding reads] → Carry history revision through metadata, snapshots, cursors, cache keys, exports, and maintenance leases, then force a metadata refresh on conflict.
- [Custom gestures can conflict with scrolling or browser zoom] → Limit custom handling to the chart surface with `pan-y`, use explicit gesture arbitration and cancellation, retain page zoom outside the surface, and verify physical touch behavior on both mobile hosts.
- [Two native reducers or cursor encoders may drift] → Use immutable expected vectors and decoded black-box equivalence rather than a shared runtime.
- [Overlapping canonical telemetry requirements remain contradictory] → Modify every superseded mandatory-method, historical-shape, persistence, UI, and conformance requirement in this delta before sync.

## Migration Plan

1. Commit the revised proposal, design, delta specs, and tasks before implementation.
2. Replace the disposable current session schema with the optimized canonical schema, authoritative recovery-summary marker, source evidence, history/prune metadata, pruning job state, and incremental auto-vacuum; bump the storage marker and verify recreation with no migration or legacy fallback.
3. Restore the 65,536-byte WebSocket limit and add marker/publication, dirty-catalog reconciliation, field-presence, recovery, read/maintenance-lease, query-plan, configurable-gap reducer, deterministic prune-selector, and long-session storage tests.
4. Add the hand-written session-data shapes, bounded streamed frontend decoder, immutable vectors, and authenticated mock endpoint.
5. Implement and verify Apple and Android read-only adapters, cancellation, gzip, sample/event pages, and HTTP routing without retaining writer or loopback executors.
6. Replace `session.get` history with metadata, delete `session.getDiagnostics`, and migrate all consumers in one breaking vertical slice before deleting old JSON/average/truncation paths.
7. Add the progressive SVG controller, axes, gestures, exact cross-input tooltips, event paging, gap setting, cache/resource tests, and performance instrumentation.
8. Add preview/start/cancel pruning ports on both hosts, revisioned snapshots/exports, atomic per-ride selection, resumable batch state, and incremental storage reclamation without frontend prune controls.
9. Run focused checks, strict validation for overlapping changes, `just verify`, and recorded checkpoints on the iPhone 12 and Galaxy S9.
10. Sync after the telemetry change in an order that leaves only the optimized schema and HTTP historical contract canonical, then archive without a compatibility fallback.

Rollback before release reverts the coordinated schema/contract/frontend slice and recreates disposable test data. There is no production database rollback or migration obligation because no schema has shipped.

## Open Questions

None. The first-production-schema reset, dedicated stable columns, published marker, composed bounds, configurable gap policy, progressive SVG groups, one-active-field exact cross-input tooltip semantics, diagnostic latency targets, deterministic irreversible pruning, and lack of shared runtime are confirmed.
