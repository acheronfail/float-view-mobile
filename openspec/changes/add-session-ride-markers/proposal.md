## Why

Riders need a quick way to bookmark something that happened without interrupting the live display. Durable markers let them find the same moment in telemetry charts and, when GPS is available, on the route after a ride.

## What Changes

- Double-tap the centre of the live view's large gauge to save a marker in the active saved session; provide equivalent keyboard and assistive activation.
- Confirm durable success with an approximately 250 ms glow/pulse and a small “Marker saved” toast. Failed saves never show success feedback.
- Retain a marker's session-relative time and optional recent recorded GPS fix in the session event log, including across reconnect, restart, pruning, and archive round trips.
- Show orange vertical marker bars on session charts and high-contrast orange marker icons on the map. A compact marker navigator and selectable icons/bars select the shared review time.
- Preserve sessions containing a committed marker even when they contain no riding or fall below the minimum session distance. Explicit user deletion still applies.
- Bound marker creation, storage, history reads, and rendering. Support markers without GPS; guest connections cannot save markers.
- Non-goals: marker text editing, categories, deletion, audio/photo attachments, automatic incident detection, haptics, firmware commands, and a separate live map. CSV remains a telemetry export; the native archive is the lossless marker export.

## Capabilities

### New Capabilities

- `session-ride-markers`: Durable manual ride bookmarks, quick capture feedback, bounded retrieval, and synchronized chart/map review.

### Modified Capabilities

- `backend-contract`: Retain explicitly marked sessions even when ordinary automatic retention criteria fail.
- `session-location-review`: Permit map review when recorded marker positions remain without a base track.

## Impact

- Extend `frontend/src/lib/backend/contract.ts`, session-data validation/client/mock, and the canonical conformance suite together. Both Apple and Android expose the same shared Rust command and read paths through existing bindings.
- Shared backend service owns session eligibility, elapsed time, GPS association, duplicate suppression, and persistence. Extend the existing event writer and reader; do not create another runtime or native marker policy.
- Update live gauge interaction, live feedback, session review selection, chart overlays, and Leaflet layers in the shared frontend.
- Audit event retention, archive validation/import/export, and short-session retention. If storage semantics require a format change, replace the development format and explicitly reset incompatible owned storage; never add migrations or reset corruption.
- No new dependencies, scripts, platform-specific frontend paths, or compatibility machinery. Reuse existing event storage, Pointer Events, Svelte animation, and Leaflet; replace any event-reader assumptions that prevent marker-only reads or sample-independent events.
