## Context

The detailed telemetry change widened every persisted series point to more than thirty stable numeric fields plus bounded extensions, while `session.get` still combines as many as 2,000 points and 2,000 events in one JSON WebSocket response. A representative 2,000-point row-shaped response is measured in megabytes rather than the contract's 65,536-byte message bound, so the frontend closes the socket with `Backend sent an invalid or oversized message`. Raising the frame limit does not address allocation, parsing, SVG geometry, or a four-hour 10 Hz ride containing about 144,000 raw samples.

The current stores already index samples and events by monotonic elapsed time and query SQLite natively. Their display downsampler groups by elapsed bucket and averages every field, which limits the returned row count but erases narrow extrema. The current frontend then renders static SVG polylines without range navigation. `vendor/float-view` demonstrates the useful rendering principle: collapse data to approximately horizontal pixel resolution and render one SVG path per series. Its whole-file CSV inflation, object parsing, maximum-absolute reducer, separate mouse/touch handlers, and complete in-browser ride ownership are not adopted.

The repository requires one disposable shared Svelte frontend, hand-written platform-neutral contracts, native storage ownership, loopback-only authenticated servers, bounded resources, mirrored Swift and Kotlin implementations, and canonical black-box conformance. Cross-language runtime code remains intentionally absent. This change corrects the active `expand-refloat-telemetry-workbench` historical transport assumptions before those delta specifications are treated as the final product contract.

## Goals / Non-Goals

**Goals:**

- Make completed long sessions open reliably and produce a useful mobile overview in less than one second.
- Make arbitrary elapsed-range zoom, pan, reset, and exact persisted-value inspection responsive through SVG charts.
- Preserve narrow extrema, endpoints, ordering, nullability, and gaps while bounding response size by requested fields and display resolution rather than raw sample count.
- Keep WebSocket control messages at 65,536 bytes and move bulk immutable or snapshot-bounded history to authenticated gzip HTTP responses.
- Keep historical reads consistent with active native persistence so the same protocol can later support current-session history from the live experience.
- Keep Swift and Kotlin implementations small and behaviorally identical through immutable vectors and canonical conformance rather than a shared runtime.

**Non-Goals:**

- Loading or retaining the full lossless ride in WebView memory.
- Adding the live-screen entry point for active-session history now.
- Sharing native query code through Rust, C, Kotlin Multiplatform, generated models, or another backend process.
- Adding GPS, maps, cross-session comparison, derived analytics, or data-recorder command support.
- Adding a downsampling-pyramid schema before bounded device measurements demonstrate that indexed snapshot scans cannot meet the target.
- Changing CSV/archive export, Bluetooth polling, persistence cadence, or controller protocol behavior.

## Decisions

### 1. Separate the small control plane from the bulk historical data plane

`/api/ws` remains the sole state-event and mutation transport with `MAX_MESSAGE_BYTES = 65_536`. `session.get` becomes a metadata request returning the summary, available numeric fields, event count, and a committed sample/event boundary. It no longer embeds numeric series or events.

Bootstrap additionally returns one session-data URL. The frontend sends discriminated JSON POST bodies for `series`, `samples`, or `events` queries and supplies the per-launch token in an `X-Float-View-Token` header. POST is selected so the browser supplies an Origin, query details do not enter URLs, and the bounded body can express field lists and cursors. The backend validates token and exact application Origin before parsing the body, returns `Cache-Control: no-store`, binds only to loopback, and never logs the token or complete telemetry.

One discriminated route is preferred over several near-identical routes because authentication, limits, errors, cancellation, and cross-platform dispatch stay in one small adapter. Moving all contract traffic to HTTP was rejected because complete monotonic `state.changed` snapshots and mutation correlation already fit the conflated WebSocket control plane. Raising the WebSocket bound was rejected because it treats only the first observed failure and weakens an established resource boundary.

### 2. Use columnar JSON with standard HTTP gzip

Bulk responses encode each requested field independently with elapsed and numeric arrays grouped into explicit continuous segments. Columnar data avoids repeating every field name for every sample and avoids null-filled unrequested records. The frontend uses ordinary `fetch` and receives decompressed JSON through WebView HTTP support. Request bodies are limited to 16 KiB and decoded responses to 4 MiB; field, bucket, exact-sample, event-page, concurrency, and duration limits constrain allocations before serialization.

Gzip is a secondary optimization after range, field, and resolution reduction. It is selected over a ZIP container because HTTP clients decompress it transparently, over Brotli because equivalent platform encoders add complexity, and over CBOR/Protobuf/custom float packing because those require another browser codec or generated layer. The format remains versioned by the hand-written TypeScript discriminated unions and canonical fixtures.

### 3. Reduce native ordered rows with a first/minimum/maximum/last envelope

A series request supplies session ID, committed sample boundary, inclusive elapsed range, allowlisted fields, and `targetBuckets` from 1 through 1,024. The store reads only requested stable columns or validated JSON paths, constrained by elapsed range and committed sequence, ordered by `(monotonic_elapsed_ms, sequence)`. Each field independently retains first, minimum, maximum, and last non-null observation within each elapsed bucket; observations filling several roles appear once in original order. A null or missing run closes the current segment, so the SVG cannot join values across a gap. Output is at most four observations per bucket per field.

This M4-style envelope keeps endpoints and diagnostic extrema. The existing SQL `AVG` reducer is deleted because a voltage sag, current spike, or duty peak can disappear. The vendor maximum-absolute reducer is rejected because it similarly loses the opposite extremum. LTTB is rejected as the only overview reducer because visual-shape selection does not guarantee physical minima and maxima.

The reducer is a pure bounded streaming kernel implemented once in Swift and once in Kotlin. Shared immutable vectors define exact bucket, ordering, gap, and deduplication behavior; the mock provides the TypeScript reference and black-box conformance compares all hosts. A shared Rust/C/WASM implementation was rejected because its toolchain, FFI, ABI, binary-size, and lifecycle cost exceeds these small cursor adapters.

### 4. Make every read a stable committed snapshot

`session.get` captures both the catalog's committed sample sequence and the greatest committed event sequence. Every data request repeats those boundaries, and SQL predicates exclude later rows. Series, samples, and events therefore remain internally stable even while the active session commits more data. A later metadata request can provide newer boundaries for a refresh without changing protocol shape.

Historical cursors use a separate read connection and WAL snapshot rather than holding the application-service/session-writer lock for the scan. Each host bounds concurrent reads, gives the newest interactive request priority, and releases its cursor, reducer, and compression buffers after success, cancellation, timeout, or disconnect. Completed sessions naturally retain stable final boundaries. The frontend currently reaches review only after leaving the live screen; adding an active-session history affordance is deferred, but no transport redesign will be required for it.

### 5. Separate overview envelopes, exact samples, and transition pages

`series` returns resolution-aware per-field envelopes. `samples` returns a requested lossless field subset for a bounded window of at most 2,000 rows and also supports deterministic nearest-sample lookup for the crosshair, preferring earlier elapsed time then lower sequence on an exact-distance tie. `events` returns chronological pages of at most 500 transitions within an elapsed range with an opaque cursor and explicit continuation state.

This removes the newest-only 2,000-event truncation that makes early transitions permanently unreachable. It also prevents overview points from being labeled exact: chart inspection always resolves the nearest stored sample. Export remains the mechanism for complete offline lossless data rather than an unbounded viewer response.

### 6. Render a resolution-bounded interactive SVG once in the shared frontend

The session route first renders metadata and measures chart width with `ResizeObserver`. It requests a full-duration overview using horizontal CSS pixel resolution, then builds one SVG path per field segment. SVG is retained because the geometry is bounded, integrates with the current frontend, scales crisply, and keeps labels and accessible surrounding controls straightforward. No DOM node is created per raw sample.

Charts share one elapsed viewport and generic request/cache owner. Horizontal one-pointer drag pans, two primary touch pointers pinch around their midpoint, tap or drag moves a crosshair, and reset restores the complete duration. Pointer capture and cancellation prevent stale gestures; touch-action is chosen and tested so vertical page scrolling remains usable. Visible buttons and keyboard controls provide zoom, pan, reset, and inspection without relying on gestures. Requests are debounced only until interaction settles, older fetches use `AbortController`, and a monotonically increasing frontend request generation prevents a late response from replacing the newest viewport.

The initial implementation uses one bounded overview query for the rendered historical catalog. This is simpler than per-card ownership and should remain small after native reduction and gzip. If the long-session benchmark misses its one-second or memory target, one generic IntersectionObserver-backed group loader may defer offscreen fields; bespoke chart loaders or a general virtualization framework are not introduced.

### 7. Verify correctness and performance at three levels

Immutable aggregation fixtures exercise field selection, bucket edges, extrema in both orders, role deduplication, duplicate elapsed times, sequence tie-breaking, sparse fields, null gaps, extensions, snapshot exclusion, exact-nearest selection, and event cursors in the mock, Swift, and Kotlin unit suites. Canonical black-box conformance exercises bootstrap, HTTP authentication, gzip, bounds, errors, cancellation, snapshot refresh, and decoded equivalence on every target.

A deterministic 150,000-sample complete-catalog fixture measures response bounds and query behavior in software. A representative supported mobile checkpoint measures from available metadata to useful overview SVG at less than one second and from settled zoom/pan to newest rendered SVG at less than 250 ms. It reports concise stage timings, response sizes, and SVG point counts when failing. These targets include native query, compression, fetch, parse, and render work but exclude application launch, Bluetooth discovery, and session creation.

## Risks / Trade-offs

- [Scanning a complete-catalog active session may contend with logging] → Use requested-field SQL, a separate WAL snapshot reader, bounded concurrency, cancellation, and native timing tests before considering a persisted pyramid.
- [First/minimum/maximum/last can produce up to four times the target bucket count] → Bound buckets and fields before querying, deduplicate multi-role samples, and enforce the 4 MiB decoded-response limit.
- [Gzip can consume CPU on low-end devices] → Compress only already bounded responses, measure compression separately, and prefer fast platform-default levels over maximal ratios.
- [Custom pinch handling can conflict with vertical scrolling or browser zoom] → Use Pointer Events, explicit touch-action, pointer capture/cancellation, visible controls, and physical touch tests on both mobile hosts.
- [A late response can replace a newer viewport] → Combine fetch abortion with frontend request generations; render only the newest matching snapshot and range.
- [Overlapping active OpenSpec changes contain contradictory historical requirements] → This change explicitly supersedes their single-response and averaging requirements; sync/archive ordering must leave the canonical specs with the data-plane requirements in this change.
- [Two native reducers may drift] → Keep them small and pure, run both against the same immutable expected vectors, and verify decoded HTTP behavior through one canonical suite.

## Migration Plan

1. Commit this completed proposal, design, delta specs, and tasks before implementation.
2. Remove the uncommitted 4 MiB WebSocket experiment and restore the 65,536-byte frontend, mock, conformance, Apple, and Android control-plane bound.
3. Add shared request/response types, immutable aggregation vectors, and the mock/reference data endpoint before switching production session review.
4. Implement and verify the Apple and Android snapshot readers, reducers, event pages, HTTP authentication, gzip, cancellation, and native bounds against the shared vectors.
5. Switch `session.get` to metadata and migrate the shared frontend to overview, range, exact-sample, and event requests in the same breaking vertical slice; delete old WebSocket series/event serialization and average-downsampling paths after all consumers move.
6. Add interactive SVG navigation and performance instrumentation, retaining the simpler one-request overview unless benchmarks require the small generic deferred loader.
7. Run focused frontend, conformance, Swift, Kotlin, security, interaction, and long-session checks; then run `just verify` and the representative physical mobile performance checkpoint.
8. Sync this change after the telemetry change in an order that leaves the superseding historical requirements canonical, archive it, and retain no compatibility fallback for the test-stage contract.

Rollback before release reverts the coordinated metadata/data-plane/frontend slice together and restores the previous code, but does not retain the invalid 4 MiB experiment as an alternative. No persisted schema migration or user-data rewrite is required.

## Open Questions

None. SVG, active-snapshot-capable queries, the 150,000-sample benchmark, one-second overview, 250-millisecond settled navigation, native mirrored reducers, and no shared runtime are confirmed.
