## Context

See proposal.md for motivation. `WidgetGauge.svelte` currently renders the large gauge without an interactive centre. `LiveView.svelte` owns its live composition. Session review already shares elapsed-time selection between `SessionHistoryChart.svelte`, `SessionLocationMap.svelte`, and `SessionView.svelte`.

The SQLite session file already contains ordered `session_event` rows, nullable sample references, JSON details, and committed event boundaries. The sole writer has a sample-independent profile-change append path. However, the public event reader explicitly filters to telemetry event kinds and the frontend event page assumes numeric source sample sequences. Markers must not be shoehorned into that telemetry representation. The product store also deletes closed sessions without riding or sufficient distance.

## Goals / Non-Goals

**Goals:** Preserve existing ownership, give a save acknowledgement only after commit, keep markers independently accessible regardless of telemetry volume, and use one review selection for charts and the map.

**Non-Goals:** See proposal.md. In particular, this does not introduce a second session store, another native runtime, platform-specific gesture code, or a generic annotation framework.

## Decisions

### 1. Capture through the shared application owner

Add authenticated `session.marker.add` with `{ sessionId, operationId }`. `sessionId` is the active saved session displayed when the gesture completes; `operationId` is a bounded UUID generated once for that action. The result contains the committed typed marker and the published session boundary. Define these types in `contract.ts`, with client/mock and canonical conformance changes in the same implementation slice. Apple and Android reuse existing shared dispatch and bindings.

The shared service accepts only the current saved session with an attached writer. An existing reconnect window retaining that same session remains eligible; a guest connection, detached/ended session, or changed session returns a bounded error. Serialize the command with close, reconnect, sample, and location work. Compute time from the session's existing monotonic clock at command admission, never from browser wall time or the latest downsampled sample. This also permits idle-time markers. A response from an old session must not animate a newly connected board.

Alternatives rejected: frontend-only markers disappear with the WebView; OS handlers duplicate product policy; using the latest sample timestamp loses timing during idle/downsampling.

### 2. Reuse durable session events, expose a typed marker read

Append `session_marker` through the existing sole SQLite writer, using a globally ordered event sequence, monotonic elapsed time, observed wall-clock timestamp, and bounded details containing operation ID and optional recorded location. The marker ID is its event sequence scoped by session ID. Marker events have no required sample reference. Reuse the existing serde packages for typed marker JSON validation; this adds a direct crate dependency but no new third-party package.

Flush/order admitted work through the runtime as needed, then atomically write the event and advance its committed event count/sequence and elapsed boundary. Preserve sample boundaries, aggregates, location boundaries, and monotonic publication when later samples arrive. A marker after the latest sample extends the review duration; do not let later aggregate recomputation move it backwards. Update the catalog via the existing reconciliation path. Return success only after the durable event transaction commits; recovery must rediscover it if catalog publication or the response fails afterwards.

An identical `{sessionId, operationId}` returns the original marker without a second event, including after a lost response. Check durable deduplication before rate/capacity checks. Do not replay an action against a different session or automatically submit a fresh operation ID after an ambiguous failure. The UI shows an unconfirmed-save message if the transport outcome is unknown; review or an explicit retry with the same ID resolves it.

Add `kind: 'markers'` to the bounded session-data plane with session ID, published snapshot, elapsed range, limit, and cursor. Return typed markers ordered by `(elapsedMs, eventSequence)`, filtered at the committed event boundary. Add a compact marker count to session metadata. Read markers directly by kind instead of paging through all telemetry events; use an appropriate event-kind/time index if needed. Keep telemetry event queries and their current output unchanged. Existing session reader workers remain bounded independent readers.

Limits: 1,000 markers per session, 256 per read page, one new marker per 500 ms per session, one pending UI save, and existing request/response byte limits. Reject excess input with the established bounded error envelope. All marker pages fit within four bounded reads; cancel/reload on session or snapshot changes. Validate limits and payloads during import as well as live writes. Do not embed marker arrays in global state snapshots or session metadata.

Alternatives rejected: a dedicated table duplicates event sequencing and archive plumbing; broadening telemetry events with nullable heterogeneous data complicates existing consumers; scanning all event pages makes marker access depend on ride length.

### Retain explicitly marked sessions

A committed marker exempts its session from automatic no-riding/minimum-distance discard, including recovery finalization. This makes “Marker saved” meaningful for a stationary incident or short test ride. Query committed marker existence through the existing store; do not infer it from all event counts or keep an unrelated frontend flag. Explicit session/board deletion continues to remove its markers. Sessions without markers retain their existing policy.

### 3. Store an optional real fix at capture time

Attach the most recent accepted, already committed location observation from the same session only if location recording is enabled, permission permits recording, and its session-relative age and observed timestamp age are each between zero and 10 seconds at capture. Otherwise store no location. Persist its sequence, elapsed time, observed timestamp, coordinates, and accuracy in the bounded marker payload so map placement survives track pruning. Never use a prior session's cached fix, future point, interpolation, a later fix, or a frontend position.

Markers without GPS remain valid on charts and in the navigator, labelled “No location”; creating one does not trigger a location permission prompt. Coordinate-bearing marker payloads receive the same privacy validation and archive handling as recorded location rows. The native archive preserves markers losslessly; CSV remains telemetry-only and does not gain hidden location-bearing marker payloads.

This deliberately chooses a stable capture-time position over retrospectively moving markers to whichever point later appears closest.

### 4. A deliberate gesture with accessible activation

Place a native `type="button"` centre target over the large gauge without moving its layout or hiding readings. The centre hit area is at least 64 CSS pixels, visually unobtrusive, and labelled “Save ride marker”; show a small discoverable “Double-tap to mark” hint when eligible.

Recognize two primary pointer releases within 350 ms and within 24 CSS pixels of each other, each with no more than 12 CSS pixels of movement and no more than 300 ms held down. Both releases must occur in the centre target. Cancel on pointer cancellation, a second simultaneous pointer, movement outside tolerance, target exit, route destruction, or session identity change. Consume each pair: a triple tap produces one action, a fourth can complete a new pair. Use `touch-action: manipulation` and prevent duplicate synthesized click activation. Mouse double-clicks use the same pointer path; do not add a separate `dblclick` handler.

Keyboard Enter/Space and assistive-generated activation perform one explicit marker action through the same command, without requiring timed double-key presses. Guard key repeats, pending state, and non-primary input. Disable saving when there is no active saved session. No detailed-view redesign is required.

### 5. Confirm the committed marker

On a successful acknowledgement for the current session, run a non-blocking approximately 250 ms orange glow around the live gauge area and display one small “Marker saved” toast for about two seconds, announced with a polite status region. Preserve gauge readability and input responsiveness. Reduced-motion users receive a static brief highlight and the same toast. Reuse the project's orange warning colour where contrast is sufficient, with an outline so markers remain recognizable in both themes.

On failure show “Marker could not be saved” with an appropriate short reason, or “Marker save not confirmed” for ambiguous transport loss. Do not animate success optimistically, stack unbounded toasts, or queue gestures for later sessions. Dispose timers on route destruction.

### 6. One marker selection across review surfaces

`SessionView` owns a bounded marker collection and selected marker ID, feeding both charts and map. Render a vertical orange bar at each in-range marker time on every time-series plot independently of downsampled data and field visibility. Map only markers with stored fixes, using orange outlined flag/bookmark icons distinct from the rider icon. Keep the map usable if the only retained coordinates belong to markers; do not fabricate route segments from them.

Add a compact “Markers” navigator with previous/next controls and current index/time, plus a bounded selectable list when needed. Selecting a chart bar, map icon, or navigator entry sets the same exact elapsed time, uses the existing bounded chart data for detail selection, reveals it in the chart viewport, and centres the map on the stored marker position if present. Wire selection into the chart as an input as well as its existing output. A no-location selection still works without showing an empty map. Keyboard activation is equivalent. Cluster overlapping screen positions visually with a count while keeping every underlying marker individually selectable in the navigator; never silently omit markers.

Alternatives rejected: static overlays alone leave tightly spaced markers difficult to revisit; independent chart/map selections can show different moments.

## Risks / Trade-offs

- GPS is sampled and may be absent → use only a recent real fix, preserve its timestamp/accuracy, and keep time-only markers useful.
- Transport loss after commit → durable operation IDs avoid duplication; do not claim success without acknowledgement or create a second operation on automatic retry.
- Markers can occur between samples or during reconnect → use the existing session clock and ensure event-only duration/publication never regresses.
- Many overlapping markers → enforce the session cap, paginate reads, render bounded overlays, and provide exact navigation independent of map/plot hit targets.
- A double tap is not discoverable to everyone → centre hint, accessible button name, and keyboard/assistive activation.
- New event details may expose coordinates through archive validation paths → apply the same validation, size limits, and privacy treatment as location observations; never log payloads by default.

## Migration Plan

Implement and verify the contract, shared storage/service, and shared frontend together. Reuse the event schema when possible; if an index or format invariant changes the owned storage format, advance its explicit marker and reset incompatible development storage only. Do not add migrations, old archive readers, or fallback schemas. Preserve current-format data on corruption or operational failures. Archive round-trip tests must prove marker identity, time, and optional location survive pruning and export/import. Commit the complete planning artifacts before implementation; finish with `just verify`, then sync and archive in the repository's final OpenSpec commit.
