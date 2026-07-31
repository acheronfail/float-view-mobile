## Why

Detailed session review currently requests up to 2,000 wide telemetry points and 2,000 events in one WebSocket response even though authenticated messages are limited to 65,536 bytes. Real responses exceed that bound, and merely raising it would still make long rides slow and memory-heavy: a four-hour ride persisted at 10 Hz contains about 144,000 samples, while a mobile chart can display only a few hundred horizontal positions.

## What Changes

- **BREAKING** Keep the authenticated WebSocket as the bounded control plane and remove numeric series and event collections from the `session.get` response; retain session metadata and a committed snapshot boundary there.
- Add an authenticated loopback HTTP data plane for bounded historical series, exact sample windows, and range-pageable transition events. Validate the launch token and application origin, keep requests and decompressed responses bounded, and deliver columnar JSON with standard HTTP gzip compression.
- Query only requested fields and elapsed ranges. Reduce overview and zoom responses natively to the requested display resolution with an ordered first/minimum/maximum/last envelope that preserves extrema, endpoints, null gaps, and sample ordering instead of averaging them away.
- Replace the static historical polylines with responsive interactive SVG charts that provide a fast full-session overview, arbitrary pinch zoom, horizontal pan, reset, and tap/drag crosshair inspection of the nearest exact persisted sample.
- Keep chart loading simple. Prefer one bounded overview request; use a small reusable lazy-loading boundary only if measurement shows it is needed to meet the performance requirements without materially complicating the frontend.
- Support consistent committed-sequence snapshots so the data plane can query an active, growing session safely. Showing historical charts from the live screen remains a future UI feature; this change continues to open review from the sessions screen.
- Require a 150,000-sample session with the complete telemetry catalog to show a useful initial overview within one second and render a settled zoom or pan within 250 milliseconds on representative supported mobile hardware.
- Supersede the historical transport, 2,000-point/event single-response, and average-downsampling decisions introduced by `expand-refloat-telemetry-workbench`. Restore the 65,536-byte WebSocket limit and delete the temporary 4 MiB message-bound experiment.

Non-goals:

- Loading an entire lossless ride into WebView memory or moving SQLite ownership into the frontend.
- Adding a shared Rust, C, Kotlin Multiplatform, or generated cross-platform backend implementation; Swift and Kotlin retain small mirrored data-plane adapters verified by shared fixtures and conformance.
- Adding in-session navigation from the live dashboard to current-session history in this change.
- Adding GPS maps, cross-session comparisons, derived ride analytics, or a precomputed downsampling pyramid unless device measurements prove the bounded native scan cannot meet the agreed targets.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `backend-contract`: Split small session metadata from authenticated compressed HTTP historical reads, define committed snapshots, range/resolution queries, exact samples, pageable events, bounds, and error semantics.
- `platform-hosts`: Require mirrored Apple and Android loopback HTTP routes, native SQLite range reduction, gzip delivery, cancellation, and active-session snapshot safety while retaining platform-owned storage.
- `live-board-dashboard`: Replace static historical charts with performant responsive SVG overview, zoom, pan, reset, crosshair, exact-value, gap, loading, and error behavior on desktop and touch clients.
- `backend-conformance`: Cover HTTP authentication, compression, range and field validation, aggregation equivalence, pagination, active snapshots, cancellation/resource bounds, extrema preservation, and representative long-session performance.

## Impact

- `frontend/src/lib/backend/contract.ts`, bootstrap/client/state ownership, session feature components, SVG chart logic, backend mock, and interaction/performance tests.
- Apple and Android loopback servers, application/session query ports, SQLite readers, bounded envelope reducers, event pagination, gzip encoding, and focused native tests.
- `conformance/` client, mock target, canonical session scenarios, immutable aggregation vectors, long-session fixtures, and performance checkpoints.
- Existing WebSocket `session.get` consumers and conformance assertions change together; export, deletion, durable logging, live snapshots, and physical BLE protocol behavior remain unchanged.
- No database migration framework or new shared runtime dependency is introduced. Existing sample and event indexes are retained or adjusted only when measurement demonstrates a query need.
