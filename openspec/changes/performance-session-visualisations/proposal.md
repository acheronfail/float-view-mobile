## Why

Detailed session review currently requests up to 2,000 wide telemetry points and 2,000 events in one WebSocket response even though authenticated messages are limited to 65,536 bytes. Real responses exceed that bound, and merely raising it would still make long rides slow and memory-heavy: a four-hour ride persisted at 10 Hz contains about 144,000 samples, while a mobile chart can display only a few hundred horizontal positions. The current SQLite layout also stores most stable numeric telemetry inside per-row JSON and discovers extension fields by scanning samples, so it cannot reliably meet the mobile target or return complete metadata cheaply.

## What Changes

- **BREAKING** Keep the authenticated WebSocket as the bounded control plane and remove numeric series, transition collections, and `session.getDiagnostics`; `session.get` retains only bounded metadata and one published committed snapshot boundary.
- Add an authenticated loopback HTTP data plane for resolution-aware series, exact or pageable sample windows, and range-pageable transition events. Validate the launch token and exact application origin, compose field/bucket/point/page limits, keep requests and decoded responses bounded, and deliver explicit columnar JSON with standard HTTP gzip compression.
- Replace the disposable test-stage session schema with the canonical first ship-ready schema: store every stable numeric telemetry field in a dedicated nullable column, retain JSON only for bounded negotiated extensions, record field presence incrementally, and make a per-session commit marker authoritative for sample/event boundaries, counts, elapsed coverage, and the bounded summary projection required to recover the catalog. Recreate incompatible local test data; add no migration or backfill machinery.
- Query only validated requested columns and elapsed ranges through cancellable read-only WAL transactions that do not retain the session writer, reconcile denormalized catalog metadata after interruption, and coordinate cancellable pending deletion with active readers.
- Reduce overview and zoom responses natively to the requested display resolution with an ordered first/minimum/maximum/last envelope that preserves extrema, endpoints, deterministic ordering, and configurable honest continuity breaks instead of averaging or joining across unavailable data.
- Add irreversible native ride pruning for one completed ride or all stored completed rides. Pruning retains only deterministic real source samples at a bounded target resolution, protects field-presence, semantic-boundary, summary, and event evidence, revisions every affected history snapshot, reports preview/progress/cancellation state through the backend contract, and reclaims SQLite storage incrementally. Backend support is in scope; user-facing prune controls are deferred.
- Replace static historical polylines with progressively loaded responsive SVG chart groups that share one elapsed viewport and provide immediate gesture previews, arbitrary bounded pinch zoom, horizontal pan, reset, and exact retained-source-sample inspection without retaining the complete ride in WebView memory.
- Provide one coherent cross-input inspection experience: chart tooltips and crosshairs expose the same exact values through mouse hover, touch or pen contact and drag, keyboard controls, and assistive technology without presenting envelope coordinates as exact data.
- Support consistent published snapshots so the data plane can query an active, growing session safely. Showing historical charts from the live screen remains a future UI feature; this change continues to open review from the sessions screen.
- Target a useful initial overview within one second and a settled zoom or pan within 250 milliseconds for a 150,000-sample complete-catalog session on an iPhone 12 and Samsung Galaxy S9 reference baseline. Record misses rather than turning these targets into correctness timeouts; slow operations remain cancellable.
- Supersede every historical WebSocket, embedded `SessionDetail`, `session.getDiagnostics`, 2,000-point/event single-response, `detail_json`, and average-downsampling requirement introduced by the telemetry workbench. Restore the 65,536-byte WebSocket limit and delete the temporary 4 MiB message-bound experiment and superseded storage/query paths.

Non-goals:

- Loading an entire lossless ride into WebView memory or moving SQLite ownership into the frontend.
- Adding a shared Rust, C, Kotlin Multiplatform, or generated cross-platform backend implementation; Swift and Kotlin retain small mirrored data-plane adapters verified by shared fixtures and conformance.
- Adding in-session navigation from the live dashboard to current-session history in this change.
- Adding GPS maps, cross-session comparisons, derived ride analytics, or a precomputed downsampling pyramid unless device measurements prove the bounded native scan cannot meet the agreed targets.
- Adding frontend settings or controls for ride pruning; this change exposes and verifies backend operations and state for later UX work.
- Preserving or migrating disposable pre-release session databases; the optimized schema becomes the first supported production schema.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `backend-contract`: Split small session metadata from authenticated compressed HTTP historical reads, remove obsolete WebSocket history operations, and define published/revisioned snapshots, explicit columnar shapes, composed bounds, exact/pageable samples, pageable events, pruning operations/state, and stable errors.
- `platform-hosts`: Make the optimized current SQLite schema and commit marker authoritative, and require mirrored Apple and Android read-only WAL adapters, native range reduction, gzip delivery, cancellation, recovery reconciliation, reader/deletion/pruning coordination, resumable batch pruning, storage reclamation, and active-session safety.
- `live-board-dashboard`: Replace static historical charts with progressively loaded native SVG paths, shared zoom/pan/reset, exact crosshair tooltips across mouse, touch, pen, keyboard, and assistive technology, plus configurable gap, axis, loading, event-page, revised-history, and error behavior.
- `backend-conformance`: Cover HTTP authentication, compression, exact shapes and composed bounds, aggregation and configurable-gap equivalence, pagination, active and recovered snapshots, deterministic pruning/revision/recovery, query plans, cancellation/resource cleanup, tooltip/gesture behavior, and representative long-session performance.

## Impact

- `frontend/src/lib/backend/contract.ts`, bootstrap/client/state ownership, session feature components, SVG chart logic, backend mock, and interaction/performance tests.
- `backends/sql/session.sql` and `backends/sql/catalog.sql` become the canonical first production schemas with dedicated stable numeric columns, bounded extension JSON, field-presence metadata, authoritative sample/event plus catalog-recovery commit metadata, captured persistence cadence, history revision/prune metadata, event-to-source evidence, bounded maintenance-job state, and incremental auto-vacuum; the test-stage storage version changes and existing local data is recreated without migrations.
- Apple and Android loopback servers, application/session query and prune ports, publication boundaries, recovery, read/maintenance leases, SQLite readers, bounded envelope reducers, sample/event pagination, deterministic row selection, persisted batch progress, storage compaction, gzip encoding, cancellation, and focused native tests.
- `conformance/` client, mock target, canonical session scenarios, immutable aggregation vectors, long-session fixtures, and performance checkpoints.
- Existing WebSocket history consumers and conformance assertions change together; exports disclose when history was pruned and are lossless only over retained source rows, while durable logging cadence, live snapshots, and physical BLE protocol behavior remain unchanged. Deletion and pruning coordinate with cancellable historical readers before mutating files.
- No database migration framework, compatibility fallback, or new shared runtime dependency is introduced. Retained elapsed indexes and every new read shape receive representative query-plan and timing checks before the UI slice proceeds.
