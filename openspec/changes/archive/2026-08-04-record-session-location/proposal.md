## Why

Float View records detailed board telemetry but cannot preserve where a rider was during a session. Optional, native-owned location recording would make historical charts spatially useful while preserving the current board-only behavior when location is unavailable, denied, or intentionally disabled.

This needs to be designed alongside the existing session-history contract because location capture must survive WebView recreation and mobile backgrounding, remain bounded and battery-conscious, and align sparse location points with persisted telemetry without fabricating data.

## What Changes

- Add an explicit opt-in setting for recording location during board sessions.
- Add a separate export option, disabled by default, for including GPS columns in CSV output.
- Add cross-platform location authorization/status reporting and permission actions for iOS, macOS, Android, and mock targets.
- Request location permission immediately when the user enables location recording, then start and stop native location capture with the board's `riding` state; continue capture while mobile apps are backgrounded whenever the platform grants the required capability.
- Persist accepted, time-aligned location points in the same per-session SQLite history as board logs, with bounded accuracy metadata, gap handling, recovery, deletion, pruning, and export behavior.
- Extend the authenticated contract and bounded session-data endpoint with location metadata and location-track queries.
- Add a sticky session map preview with a travelled line and the upstream Float View rider icon, synchronized to the selected elapsed point in the historical charts.
- Add a small live-view location option/warning that can request permission again or open platform settings, without blocking Bluetooth connection or board logging.
- When the export option is enabled, add `gpsAccuracy`, `gpsLongitude`, and `gpsLatitude` columns to CSV telemetry rows by repeating the nearest eligible recorded location point across nearby rows with deterministic tie-breaking; leave the columns blank when no point qualifies. Retain the independent location track for session review and bounded reads.
- Use native location-provider power controls and board ride state to avoid unnecessary updates. Do not expose a user-configurable polling interval in the first version unless device measurements show that a bounded setting is useful.
- Add mirrored Apple/Android implementation, mock behavior, canonical conformance coverage, and physical mobile background checkpoints.

Non-goals:

- No location data in complete `state.changed` snapshots or default diagnostic logs.
- No browser-owned geolocation path; native services remain the only permission, background, persistence, and lifecycle owners.
- No route navigation, geocoding, social sharing, cross-session map comparison, or offline map-tile package.
- No migration of older disposable databases; the session schema version will be recreated as required by the repository workflow.

## Capabilities

### New Capabilities

- `session-location-recording`: Optional native location permission, background capture, time-aligned persistence, lifecycle, privacy, battery policy, and export behavior.
- `session-location-review`: Bounded location-track retrieval and synchronized sticky map presentation in historical session review.

### Modified Capabilities

- `backend-contract`: Add location state, permission actions, persisted opt-in settings, session location metadata, published location boundaries, and bounded location-data requests/results.
- `platform-hosts`: Add Apple Core Location and Android location-provider permission/background wiring, including the existing Android foreground logging service.
- `live-board-dashboard`: Add the non-blocking live-view location option and permission warning behavior.
- `backend-conformance`: Add permission, persistence, background continuity, gap, privacy, export, map synchronization, and CSV flattening coverage across the mock and mobile targets.

## Impact

- Frontend: `contract.ts`, session-data validation/client/controller, settings, live view, session view/history, map feature, chart-selection state, rider asset, and tests.
- Apple: shared application domain/service, Core Location adapter, SQLite schema/store/export, iOS Info.plist/background mode, macOS location wiring, and tests.
- Android: location adapter and permission broker, `LoggingForegroundService`, manifest/foreground-service declarations, SQLite schema/store/export, and tests.
- Shared SQL and storage marker version; location-aware published snapshots, recovery, deletion, pruning, and bounded historical readers.
- Conformance fixtures, mock backend, scenario assertions, and mobile lifecycle/permission preparation.
- Likely frontend dependency addition: Leaflet, following the existing upstream `vendor/float-view` map implementation and attribution requirements.
