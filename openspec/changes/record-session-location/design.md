## Context

Float View already has a native-owned, per-session SQLite history for board telemetry and a shared frontend that reads bounded historical data through the authenticated loopback contract. The WebView can be recreated, and iOS and Android can suspend or background the UI, so location capture cannot live in Svelte stores or depend on a browser geolocation API.

Location samples are also materially different from board samples: they arrive at a lower and less predictable cadence, may be unavailable for part of a ride, and have accuracy that must be preserved rather than hidden. The design therefore needs an independent location track with a common session clock, while still making chart review and CSV export convenient.

The upstream `vendor/float-view` map provides the visual precedent: Leaflet renders a base path and a travelled path, and a custom rider icon marks the selected point. The new implementation will reuse that interaction model and the upstream rider asset, but will select by elapsed time rather than assuming one GPS point per telemetry row.

## Goals / Non-Goals

### Goals

- Make location recording explicitly optional and safe to decline.
- Keep permission, background execution, persistence, export, and lifecycle ownership in native services.
- Capture location only for an authoritative active board session and recover it after frontend recreation.
- Preserve sparse data, accuracy, and gaps without fabricating `(0,0)` values or silently interpolating across outages.
- Expose bounded, authenticated location reads and a location-aware session summary without putting coordinates into complete live state snapshots.
- Keep the historical map synchronized with the chart's selected elapsed time.
- Make CSV useful by flattening location onto telemetry rows deterministically, while keeping the independent track in the full archive.
- Use provider power controls and ride-state lifecycle to limit battery use.

### Non-Goals

- Browser or WebView geolocation ownership.
- Turn-by-turn navigation, geocoding, social sharing, route comparison, or a packaged offline tile database.
- A user-facing GPS polling interval in the first version.
- Filling every telemetry row with synthetic or interpolated coordinates.
- Adding coordinates to `state.changed`, default diagnostics, or routine logs.

## Decisions

### Native location service and permission model

Introduce a platform-neutral location recorder owned by the application service. It will depend on a small platform adapter that can report authorization, request foreground/background capability where supported, start and stop a session-scoped stream, and deliver bounded native observations. A separate permission broker maps OS-specific outcomes into stable contract values and the action to retry permission or open settings.

The persisted setting `locationRecordingEnabled` defaults to `false`. Enabling it immediately requests the next supported OS permission; it does not block connection or board logging. A separate persisted `includeGpsLocationWhenExporting` option defaults to `false` and controls only whether GPS columns are added to CSV exports. If permission is denied, restricted, unavailable, or no fix is obtained, the board-only path remains fully functional and the authoritative state exposes a non-blocking status for the live warning. A permission action is idempotent and never starts capture unless the effective setting and authorization allow it.

The status model will distinguish at least `disabled`, `notDetermined`, `foregroundOnly`, `authorized`, `denied`, `restricted`, and `unavailable`, plus a bounded user-facing reason where needed. The frontend does not infer platform behavior from user-agent checks.

### Session clock and location rows

Every accepted location observation is assigned an independent monotonic `locationSequence` and the active session's monotonic `elapsedMs`, captured by native code at receipt. Persisted rows contain latitude, longitude, horizontal accuracy, elapsed time, and an observation timestamp suitable for export/audit. Coordinates are bounded to valid ranges; reported accuracy is preserved as supplied, including poor or unavailable accuracy, with no quality threshold in the first version.

Location points live in their own SQLite table and index, keyed by session ID and sequence/elapsed time. They are appended independently of telemetry rows, in bounded batches where the platform provider delivers bursts. Capture begins when the authoritative board ride state enters `riding` and stops when it leaves `riding`; it does not run during connected-idle or post-ride tail time. A commit marker includes the last durable location sequence and elapsed boundary alongside the telemetry boundary, so published session metadata identifies one consistent durable read point.

The reader returns points in chronological order and explicit gap metadata derived from adjacent observations and provider availability. No point is synthesized for a gap, and no cross-gap interpolation is performed. A map may draw only the base path through accepted points and a travelled path through points at or before the selected elapsed time.

Location rows follow the existing provisional-session rules: they are deleted with rejected/aborted sessions, recovered with the session database, included in size/prune accounting, and removed by the same deletion/pruning operations. A schema-version mismatch recreates the current disposable logging store; older disposable databases are not migrated.

### Contract and state boundaries

Extend the authenticated session-data plane with a bounded `locations` request/result. The request carries session ID, published snapshot revision, cursor/limit, and an elapsed-time range. The result carries location points, explicit gaps, the next cursor, and the published boundary it was read against. Bounds, cursor semantics, and invalid-session behavior match the existing series/samples/events readers.

Session metadata and the published snapshot gain bounded location count, availability/status, and location boundary information. `AppState` gains only location recording status and permission-action availability; it does not contain coordinates or a historical track. This keeps complete state snapshots safe and small while allowing the frontend to query location history only when a session has location data.

CSV export is intentionally lossy relative to the independent track and is opt-in through `includeGpsLocationWhenExporting`. When enabled, each telemetry row receives the nearest eligible recorded location point within the documented time tolerance; the chosen point's accuracy, longitude, and latitude are repeated across all rows to which it is eligible. If two points are equally near, choose the lower location sequence. If no point qualifies, leave the GPS columns blank. CSV does not interpolate coordinates; the independent track remains available through session review and bounded location reads.

### Map and chart synchronization

The session view loads the session metadata first, then requests location pages only when location is available. It renders a sticky map preview beside or above the chart according to the existing responsive session layout. Leaflet is the initial map renderer because it matches the checked-in upstream implementation; tile attribution and provider URL remain explicit configuration, and no geocoding or offline tile cache is introduced.

The chart controller owns one selected elapsed-time value. The map subscribes to it and derives:

1. the full accepted track,
2. the travelled polyline through points whose elapsed time is no later than the selection, and
3. the rider marker at the latest eligible point, or at a bounded interpolation only between two adjacent points within the same gap-free segment.

The rider marker reuses `vendor/float-view/src/assets/rider-icon.svg` through the same visual treatment as the upstream `Map.ts`. Selection changes update the map without issuing an unbounded query per pointer movement; the controller uses the loaded bounded window and debounces/guards additional page reads.

### Platform lifecycle

On Android, the existing `LoggingForegroundService` remains the lifecycle owner for an active session and is extended with the location foreground-service type and notification policy. The application requests foreground location immediately when the user enables recording and requests background capability/settings when the platform requires it. Capture starts when the native ride state enters `riding` and stops when it leaves `riding` or when the effective setting/permission no longer permits it.

On iOS, the shared Apple application service owns a Core Location manager. The iOS host declares the required usage descriptions and background location capability, and the manager is configured for ride capture while the app is backgrounded. A foreground-only grant records while the app is eligible in the foreground and reports that background capture is unavailable. macOS uses the same service and contract with macOS authorization behavior; it does not depend on iOS suspension semantics. When the UI server stops in the background, the native session and recorder remain independent and the next frontend bootstrap recovers current state.

Mock and browser-development hosts expose deterministic permission/status fixtures and location rows for contract tests. They do not create a parallel browser geolocation implementation.

### Battery policy

The first version does not add a configurable GPS polling interval. Native provider settings use balanced/low-power defaults, an accuracy ceiling, and a minimum movement threshold where supported. The recorder starts only for an enabled, authorized active session, avoids duplicate observations, batches durable writes, and stops promptly when the session ends or recording is disabled. Instrumentation and conformance capture provider/update counts so a later configurable cadence can be justified by measurements rather than added speculatively.

### Verification and privacy

The canonical conformance suite covers permission paths, board-only fallback, location persistence, ordering, gaps, deletion/pruning, CSV tie-breaking, snapshot boundaries, frontend recreation, and mobile background continuity. Physical iOS and Android checkpoints verify that the native recorder—not the WebView—continues while the application is backgrounded, with the fixture providing deterministic board telemetry.

Coordinates are excluded from default native logs, diagnostics, and complete state events. Export and session review are explicit user actions. Tests use fixed fixtures and must not print real addresses or location payloads in failure diagnostics.

## Risks / Trade-offs

- OS background permission rules vary. The contract reports capability and offers settings navigation, while each native adapter owns the exact request sequence.
- GPS fixes may be sparse or inaccurate. Accuracy and gaps are visible in the track, and repeated nearest-row CSV values are documented as a convenience projection rather than a claim that every telemetry row was independently located.
- Leaflet and remote tiles add frontend weight and a network/privacy dependency. Keeping the map feature isolated and retaining an explicit tile-provider seam preserves a later swap or offline strategy.
- Location persistence increases storage and privacy sensitivity. Bounded rows, pruning/deletion parity, no default coordinate logging, and explicit export reduce the exposure.
- Background capture can affect battery. Provider power controls, ride-state gating, batching, and measured update counts are preferred over a user-facing interval that may not map reliably to native providers.
- The location boundary must stay in step with the existing commit marker. Recovery and readers must test partial writes so the frontend never reads beyond the published durable boundary.

## Migration Plan

1. Add the contract types, validators, permission/status actions, settings field, session metadata, published location boundary, and `locations` session-data method to the frontend client/mock and canonical conformance types.
2. Add the disposable SQLite location table/index and commit-marker fields, then implement shared application-service recording, bounded reads, export projection, deletion, pruning, and recovery behavior.
3. Implement the Apple adapter/host declarations and the Android adapter, foreground-service declarations, and permission/settings flows. Keep location disabled by default while the vertical slice is completed.
4. Add the live warning/permission action, session map, chart selection bridge, upstream rider asset, and CSV/archive presentation.
5. Run software conformance and `just verify`, then perform the explicit physical mobile background checkpoints and inspect update-count/battery instrumentation.

The repository's current storage policy recreates disposable stores on schema mismatch, so this change bumps the current storage/schema version and does not migrate older disposable databases. If the implementation is rolled back, the setting remains absent/false and board-only logging continues; no location rows are required for older sessions.

## Open Questions

- Confirm the smallest acceptable Android location-provider dependency/API for the current minimum SDK and whether its background permission model needs a separate settings action on any supported target.
- Confirm the product-approved tile provider and attribution/privacy wording before shipping the remote map preview.
- Set the exact nearest-row tolerance, gap threshold for map interpolation, and maximum location page size in the contract implementation after reviewing fixture cadence and storage measurements. CSV repeats the selected recorded point and does not interpolate.
- Decide whether location update-count diagnostics remain test-only or become an internal support metric; they should not become a user setting without battery evidence.
