## 1. Contract and shared types

- [ ] 1.1 Add the location recording setting, default-disabled GPS export setting, authorization/status model, permission actions, location metadata, published location boundary, and bounded location point/gap types to `frontend/src/lib/backend/contract.ts`.
- [ ] 1.2 Add `locations` request/result validation, cursor/limit bounds, elapsed-time range validation, and stable location error handling to the session-data contract/client.
- [ ] 1.3 Extend `AppState`, `Settings`, `SessionMetadata`, and published session snapshot handling without including coordinates in complete state snapshots.
- [ ] 1.4 Update the mock backend and frontend contract fixtures for disabled, not-determined, granted, denied, restricted, unavailable, gap, and boundary states.
- [ ] 1.5 Add contract-level tests for permission actions, settings revision conflicts, bounded location queries, stale snapshots, and invalid limits.

## 2. Shared persistence and export model

- [ ] 2.1 Add the disposable SQLite location table, indexes, and current schema version update in `backends/sql/session.sql`.
- [ ] 2.2 Extend the commit marker and session metadata queries with the last durable location sequence, elapsed boundary, count, and availability state.
- [ ] 2.3 Implement a native location recorder abstraction that assigns session-relative monotonic elapsed time and independent location sequence values only while the ride state is `riding`.
- [ ] 2.4 Implement bounded location appends with coordinate/payload validation, preservation of reported accuracy without a quality threshold, duplicate suppression, durable batching, and boundary publication only after commit.
- [ ] 2.5 Implement paged location reads with elapsed-time filtering, explicit gap derivation, snapshot-bound validation, and no cross-gap interpolation.
- [ ] 2.6 Integrate location rows with provisional-session retention, interruption recovery, integrity checks, deletion, pruning, storage-size accounting, and schema recreation.
- [ ] 2.7 Extend native CSV export with the opt-in GPS columns while keeping the independent location track available through session-data review.
- [ ] 2.8 Implement opt-in CSV GPS columns (`gpsAccuracy`, `gpsLongitude`, `gpsLatitude`) by repeating the nearest eligible point across rows, using the lower location-sequence tie-break and blank unmatched fields.
- [ ] 2.9 Add persistence tests for monotonic sequencing, sparse observations, partial writes, restart boundaries, gaps, deletion, pruning, and CSV projection.

## 3. Apple location implementation

- [ ] 3.1 Add a Core Location adapter under the shared Apple application ownership boundary for authorization, provider configuration, updates, and settings presentation.
- [ ] 3.2 Wire Apple permission/status changes and session start/stop into the application service without making the WebView a recorder owner.
- [ ] 3.3 Add iOS location usage descriptions and the required background location declaration while preserving the existing Bluetooth background mode.
- [ ] 3.4 Verify macOS authorization and lifecycle behavior through the same Apple service, including settings fallback and no iOS-only assumptions.
- [ ] 3.5 Add Apple unit/integration coverage for permission transitions, background UI-server teardown, native capture continuity, and location-boundary recovery.

## 4. Android location implementation

- [ ] 4.1 Add an Android location-provider adapter for authorization, balanced/low-power configuration, movement threshold, updates, and settings presentation.
- [ ] 4.2 Wire immediate permission resolution, foreground/background capability, and location status into the Android application service with board-only fallback.
- [ ] 4.3 Extend `LoggingForegroundService` to own eligible location capture for the active session, including notification and bounded start/stop behavior.
- [ ] 4.4 Add Android manifest permissions, location foreground-service type, and runtime/settings flow required by the supported target SDK.
- [ ] 4.5 Add Android tests for permission denial, missing background capability, service continuity, restart recovery, and stop-on-session-close behavior.

## 5. Frontend live-view and session review

- [ ] 5.1 Add adjacent settings controls for location recording and `includeGpsLocationWhenExporting`, with both defaults and independent revision-checked persistence.
- [ ] 5.2 Add live-view location option/warning states and immediate permission action driven only by authoritative contract state, with accessible pointer, keyboard, and assistive-technology activation.
- [ ] 5.3 Add the shared permission/settings action handling and refresh behavior after the native status changes.
- [ ] 5.4 Add Leaflet using the smallest maintained dependency path, preserve tile attribution, and reuse `vendor/float-view/src/assets/rider-icon.svg` for the rider marker.
- [ ] 5.5 Add the bounded session location loader with snapshot/cursor handling, explicit gaps, and no request per chart pointer movement.
- [ ] 5.6 Add the sticky session map preview with base track, travelled line, rider marker, responsive layout, and board-only omission.
- [ ] 5.7 Connect chart selected elapsed time to map rendering, including latest eligible point, gap-free interpolation tolerance, and selection updates.
- [ ] 5.8 Add session review and export UI handling for location availability, independent track review, the default-disabled GPS export option, and repeated CSV GPS columns without changing board-only review.
- [ ] 5.9 Add frontend tests for warning activation, immediate permission flow, adjacent settings, WebView recreation, map omission, sparse tracks, gap behavior, marker selection, and deterministic CSV display/export state.

## 6. Canonical conformance and fixtures

- [ ] 6.1 Add deterministic location-provider fixtures with configurable permission transitions, cadence, accuracy, sparse points, gaps, and interruption points.
- [ ] 6.2 Add shared contract scenarios covering settings, permission actions, status events, location queries, bounds, stale snapshots, and board-only fallback.
- [ ] 6.3 Add shared persistence scenarios covering sequence/elapsed alignment, published boundaries, recovery, deletion, pruning, archive rows, and no fabricated coordinates.
- [ ] 6.4 Add CSV projection scenarios covering repeated nearest-row selection, equal-time tie-breaking, blank unmatched fields, GPS-export opt-in, and independent track retention.
- [ ] 6.5 Add redaction and battery-policy assertions for state events, diagnostics, idle capture, provider stop timing, and bounded update counts.
- [ ] 6.6 Extend Android/iOS target adapters with only the required permission preparation, foreground/background action, bootstrap refresh, and cleanup seams.
- [ ] 6.7 Add physical Android and iOS checkpoints that background an active location-enabled session and verify native continuity after foreground recovery.

## 7. Verification and delivery

- [ ] 7.1 Run focused frontend, Apple, Android, SQL, mock, and conformance tests for the location slices and resolve contract drift across all targets.
- [ ] 7.2 Verify that coordinates are absent from default logs, diagnostics, and complete state events, and that export paths are explicit and bounded.
- [ ] 7.3 Run `just verify` and keep the change buildable with location disabled by default.
- [ ] 7.4 Execute the required physical mobile background checkpoint on supported devices and record provider/update-count observations for the battery decision.
