## 1. Contract baseline and canonical vectors

- [ ] 1.1 Revert the temporary 4 MiB WebSocket-bound experiment across the frontend, mock, conformance client, Apple, and Android; restore and test the canonical 65,536-byte UTF-8 control-plane limit.
- [ ] 1.2 Replace `SessionDetail` with bounded session metadata containing summary, available numeric fields, event count, committed sample sequence, committed event sequence, and committed elapsed boundary; remove embedded series and events from `session.get`.
- [ ] 1.3 Add bootstrap session-data URL discovery and hand-written discriminated `series`, `samples`, and `events` HTTP request/result/error types with token-header, Origin, byte, field, range, bucket, sample, event-page, and cursor bounds.
- [ ] 1.4 Add immutable aggregation fixtures covering bucket edges, first/minimum/maximum/last order and deduplication, duplicate elapsed times, sequence ties, sparse/null/gap segments, stable and extension fields, snapshot exclusion, exact-nearest selection, and event pagination.
- [ ] 1.5 Implement the TypeScript reference reducer and mock session-data route against the immutable fixtures, including columnar JSON, gzip, no-store, authentication, stable errors, active snapshot boundaries, and cancellation cleanup.
- [ ] 1.6 Add contract, bootstrap, mock-auth, fixture-vector, bounds, gzip, and session-metadata unit tests before switching production consumers.

## 2. Apple session-data adapter

- [ ] 2.1 Add Apple session metadata retrieval that atomically captures available fields and committed sample/event boundaries for completed and active sessions.
- [ ] 2.2 Add an Apple read-only WAL snapshot cursor that selects only validated requested fields, elapsed ranges, and sequence boundaries without retaining the session-writer or application-service lock during the scan.
- [ ] 2.3 Implement the bounded pure Swift first/minimum/maximum/last reducer and exact-nearest selection against every immutable aggregation vector, including null gap segmentation and deterministic ties.
- [ ] 2.4 Implement bounded chronological Apple event range pagination with opaque cursor validation and a 500-event page maximum.
- [ ] 2.5 Add the authenticated Apple session-data POST route with pre-body token/Origin validation, request dispatch, standard gzip, no-store responses, decoded-size enforcement, bounded concurrency, timeout, and disconnect/cancellation cleanup.
- [ ] 2.6 Add Apple tests for metadata, active snapshots, writer non-obstruction, reducer equivalence, exact samples, event cursors, authentication privacy, compression, byte bounds, cancellation, failures, and resource release.

## 3. Android session-data adapter

- [ ] 3.1 Add Android session metadata retrieval that atomically captures available fields and committed sample/event boundaries for completed and active sessions.
- [ ] 3.2 Add an Android read-only WAL snapshot cursor that selects only validated requested fields, elapsed ranges, and sequence boundaries without holding the synchronized session-writer path during the scan.
- [ ] 3.3 Implement the bounded pure Kotlin first/minimum/maximum/last reducer and exact-nearest selection against every immutable aggregation vector, including null gap segmentation and deterministic ties.
- [ ] 3.4 Implement bounded chronological Android event range pagination with opaque cursor validation and a 500-event page maximum.
- [ ] 3.5 Add the authenticated Android session-data POST route with pre-body token/Origin validation, request dispatch, standard gzip, no-store responses, decoded-size enforcement, bounded concurrency, timeout, and disconnect/cancellation cleanup.
- [ ] 3.6 Add Android tests for metadata, active snapshots, writer non-obstruction, reducer equivalence, exact samples, event cursors, authentication privacy, compression, byte bounds, cancellation, failures, and resource release.

## 4. Shared interactive SVG session review

- [ ] 4.1 Add one frontend session-data client that retains bootstrap endpoint/token ownership, sends bounded authenticated POST requests, decodes responses, exposes abortable generations, and rejects invalid or oversized decoded payloads.
- [ ] 4.2 Rework session state ownership to load metadata first, request a full-duration field-minimized overview at measured chart resolution, cache snapshot/range results, abort superseded work, and prevent late generations from replacing the newest viewport.
- [ ] 4.3 Replace static telemetry polylines with responsive SVG field segments and bounded path geometry that preserve real elapsed spacing, extrema, endpoints, stable units, and visible null gaps without one frontend object per raw sample.
- [ ] 4.4 Implement shared elapsed viewport math for bounded horizontal pan, midpoint-preserving two-pointer pinch zoom, resize convergence, and full-duration reset using Pointer Events, capture, cancellation, appropriate touch-action, and newest-request ordering.
- [ ] 4.5 Add visible native buttons and keyboard/assistive-technology activation for zoom, pan, and reset with guarded click behavior and no double activation.
- [ ] 4.6 Add tap/drag crosshair selection, deterministic nearest-exact-sample requests, exact-versus-envelope labeling, unavailable-value presentation, and coherent selection across viewport refreshes.
- [ ] 4.7 Replace the truncated embedded transition list with chronological elapsed-range event paging and continuation UI that keeps annotations inspectable alongside the selected chart range.
- [ ] 4.8 Measure the one-request overview on the long-session fixture; add only one reusable IntersectionObserver-backed chart-group loader if required to satisfy the one-second or memory target, otherwise retain the simpler eager bounded overview.
- [ ] 4.9 Add frontend unit and component coverage for loading, failure, resize, gaps, extrema, pan bounds, pinch midpoint, cancellation, stale responses, reset, exact selection, event continuation, pointer variants, vertical scrolling, keyboard operation, and frontend recreation.

## 5. Canonical conformance and performance

- [ ] 5.1 Extend the canonical client and target bootstrap handling to discover and authenticate the session-data endpoint, decode gzip through ordinary HTTP semantics, enforce decoded bounds, and refresh credentials after frontend recreation.
- [ ] 5.2 Replace WebSocket series/event assertions with shared metadata plus `series`, `samples`, and `events` scenarios covering every mandatory success, stable failure, field/range minimization, snapshot, pagination, privacy, gzip, and bound.
- [ ] 5.3 Add cross-target cancellation, concurrent-query, stale-boundary, active-session append, and post-failure recovery scenarios that verify historical work remains bounded and does not obstruct durable persistence.
- [ ] 5.4 Add a deterministic 150,000-sample complete-catalog session fixture and software measurements for native scan, reduction, gzip, response size, frontend parsing, SVG geometry count, and settled range refresh.
- [ ] 5.5 Add a representative supported mobile performance checkpoint that verifies useful overview SVG within one second and newest settled zoom/pan SVG within 250 milliseconds, reporting concise stage diagnostics on failure.

## 6. Superseded paths, documentation, and verification

- [ ] 6.1 Delete old WebSocket historical series/event serialization, newest-only event truncation, frontend static-session request paths, and SQL average-downsampling code after every consumer uses the new data plane.
- [ ] 6.2 Review retained SQL indexes and query plans on representative completed and active long sessions; change the shared current schema only if measurements demonstrate a missing index, without adding a migration framework or downsampling pyramid.
- [ ] 6.3 Update backend, contract, conformance, and frontend documentation with the control/data-plane boundary, authentication, compression, snapshot, aggregation, SVG interaction, bounds, performance targets, and deferred live-screen history feature.
- [ ] 6.4 Reconcile the superseded `expand-refloat-telemetry-workbench` historical requirements and archive/sync ordering so canonical specs retain the 65,536-byte WebSocket bound and the new HTTP range-query behavior.
- [ ] 6.5 Run focused frontend, mock, conformance, Apple, Android, security, aggregation-vector, interaction, active-snapshot, and long-session performance checks.
- [ ] 6.6 Run strict OpenSpec validation for every overlapping change and then run `just verify` as the complete software gate.
- [ ] 6.7 Run the representative physical mobile performance checkpoint and review captured timings, response sizes, memory bounds, query cancellation, and active logging behavior before sync and archive.
