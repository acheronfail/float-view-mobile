# Design

## Context

See proposal.md for motivation and scope. `SessionView.svelte` receives a single chart viewport and passes it to `SessionLocationMap.svelte`, which clips and recenters its route. `SessionHistoryChart.svelte` owns continuous-time gestures and queries. The map loads at most four location pages without checking whether a cursor remains. `SessionReviewController` cancels earlier series requests with the same field list, which prevents independent concurrent interval requests today.

Upstream Float View computes point visibility from map bounds and marks skipped row indices with dashed chart separators. Our independent timestamped GPS track and bounded telemetry endpoint require time intervals rather than a shared row visibility array.

## Goals / Non-Goals

**Goals:** Preserve one shared frontend, existing authenticated reads, exact original-time inspection, bounded memory/work, and a single explicit owner for navigation state.

**Non-Goals:** No server-side geographic policy, persisted preference, new package, storage migration, or duplicated native implementation. Summary statistics and event pages retain ordinary review semantics; their labels must not imply that they are spatially filtered.

## Decisions

### 1. Coordinate mode and selection in the session view

The session feature owns transient filter mode, saved ordinary viewport, current geographic bounds, and ordered included intervals. Keep Leaflet objects private to the map. Expose plain bounds and completed track data through feature-local interfaces; do not reload the same track for filtering.

Enabling preserves current map bounds, restores the full base route layer without fitting it, cancels any chart gesture, and switches charts to inspection mode. Disable navigation buttons with an explanation and guard all gesture paths as well. Disabling restores the saved ordinary viewport and its normal map synchronization. Hiding the map disables filtering. Session change/unmount resets mode. A same-session snapshot refresh retains mode and bounds but invalidates all derived data until a consistent replacement is ready.

While enabled, chart updates and selection never fit/recenter the map. Coalesce duplicate move/zoom completion notifications and submit one update for the final bounds; do not issue requests during every animation frame. Full-route reset deliberately fits the full route and recomputes filtering, retaining the mode. This explicit ownership avoids the feedback loop that bidirectional viewport callbacks would create.

The toggle uses native button semantics, `aria-pressed`, primary Pointer Events and guarded keyboard/assistive activation, with cancellation and no double activation. It defaults off and has no durable settings field; navigation state does not need a cross-platform persistence contract.

### 2. Clip continuous route edges to geographic bounds

Reuse existing location segmentation, including explicit gaps and the 10-second continuity threshold. Clip each eligible edge in the map's projected coordinate space against its visible rectangle; interpolate original elapsed time at entry/exit. Normalize wrapped longitudes around the current map center so antimeridian routes and equivalent world copies behave consistently. Include boundary edges; discard zero-duration tangencies and isolated points. Merge adjacent clipped portions only within the same continuous segment and with no excluded duration.

Derive intervals from the complete track, independent of the saved chart viewport. A point-only containment mask was rejected because it misses crossings when both endpoints are outside the map. Never interpolate across a location gap or extrapolate beyond track endpoints.

### 3. Share a piecewise timeline projection

Add feature-owned pure geometry mapping each original-time interval to a cumulative included-duration axis. Ordinary charts use a single interval. Replace duplicated chart, tooltip, selection and marker x-coordinate calculations with this projection. Preserve original elapsed timestamps in query data and inspection; compressed coordinates are presentation only.

Each interval renders separate paths, retaining existing null/generation breaks inside it. Draw aligned dashed separator lines at joins, with a small visual break so unrelated values never connect. At an exact join, resolve inspection deterministically to the start of the later interval; the final edge resolves to the final interval end. Automatic y domains use only included values; fixed domain policies remain unchanged.

When bounds change, retain inspection if its timestamp remains included; otherwise clear inspection until another eligible selection. Outside-filter marker/fault activation may identify the item, but clears chart inspection and shows an outside-filter indication without moving the map or changing intervals. Filter chart marker projections by included time. Keep the session marker list available and existing map overlays on the full route.

Compressed duration was chosen over a continuous axis with large blank spans so small repeated visits can fill the available width. Uniform row spacing was rejected because telemetry and retained sampling rates vary.

### 4. Query each included interval with bounded scheduling

Reuse the existing single-range series endpoint per included interval and visible field group. Allocate a total bucket budget of `min(1024, max(intervalCount, floor(plotWidth)))` per field group, assigning at least one bucket per interval and distributing the remainder by duration. Keep progressive field-group loading and exact inspection reads. Clip any returned boundary observations to the requested interval and preserve interval identity when combining results.

Use one generation for a complete filter selection, with at most two interval requests in flight across the filtered chart loader. Extend controller-internal request ownership so intervals in the same generation do not cancel each other, while a newer selection cancels the entire prior group. Keep the existing 32-entry/8 MiB cache; bound retained decoded filtered chart data to 16 MiB. Publish each field group's interval set atomically, never a partial set labelled complete. Mark old plots stale or hide them while new bounds load so they cannot masquerade as current results.

Cap filtering at 128 intervals. Exceeding the cap or decoded budget yields an explicit actionable limit state, never interval truncation or merging across excluded time. This keeps the initial implementation frontend-only. A multi-range server endpoint is deferred unless measured performance later justifies the broader contract change.

### 5. Complete location loading and consistent failure handling

Replace the four-page cutoff with cursor pagination, retaining the existing maximum 2,000 rows per response. Apply total review limits of 100,000 points, 100,000 gap records, 16 MiB decoded track data, and 128 pages; detect repeated cursors. Filtering requires successful exhaustion of the cursor within these limits and at least one positive-duration continuous edge. Never offer filtering over an incomplete prefix. Ordinary review remains usable with a visible track error.

All interval queries use the same session snapshot. On conflict refresh metadata once per failed generation, then rebuild track/intervals and query against the replacement snapshot; do not run an unlimited retry loop. Transport failures expose retry. Cancel work on unmount, disable, session change, or superseding selection; ignore late completions. A reconnect cannot revive stale results.

## Risks / Trade-offs

- Many short visits multiply requests → interval cap, shared concurrency budget, caching and proportional bucket allocation; test worst supported cases.
- GPS noise near a boundary can create many intervals → preserve honest geometry and report the limit; do not silently smooth away visits.
- GPS precision limits entry/exit accuracy → interpolate only existing continuous edges and retain gap exclusions.
- Long sessions can exceed review budgets → explicit limits and ordinary review remain available; no silent tail loss.
- Mode switching can leave active pointers or stale requests → cancel gestures and request generations at each ownership change; cover pointer cancellation and late completion tests.

## Migration Plan

Commit planning artifacts before implementation. Implement in verified frontend slices, then sync and archive together after affected checks pass. There are no persisted data or protocol changes; rollback consists of reverting the frontend change. No native or physical BLE acceptance gate is required for this scope.
