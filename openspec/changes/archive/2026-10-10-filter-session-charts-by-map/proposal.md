# Proposal

## Why

Session review currently lets chart zoom narrow the route, but cannot use a location to find the corresponding telemetry. Filtering charts by the visible map area makes it possible to inspect a hill, turn, or repeated passes without finding each time interval manually.

## What Changes

- Add an off-by-default **Filter charts by map** toggle beside the map controls for sessions with a usable location track.
- While enabled, map pan/zoom selects chronological ride intervals; all charts concatenate those intervals by included duration, with dashed vertical separators and broken data paths between disjoint portions.
- Disable every chart zoom/pan input while filtering is enabled; retain inspection, original ride timestamps, field visibility, and eligible marker selection.
- Restore the previous ordinary chart viewport when filtering is disabled. Keep mode and bounds transient to the open session review.
- Handle empty areas, GPS gaps, complete bounded location loading, cancellation, and resource-limit failures explicitly.
- Replace duplicated continuous-axis coordinate calculations with shared timeline projection, and remove the map loader's silent four-page truncation. Add no dependencies or compatibility machinery.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-location-review`: Map-driven chart filtering, disjoint timeline presentation, interaction ownership, and bounded review lifecycle.

## Impact

Changes are scoped to the shared frontend session feature, its session review controller, geometry utilities, and frontend/mock browser coverage. Existing authenticated location and series range requests remain unchanged across Apple and Android. No public API, persisted settings, storage format, exports, or native behavior changes are planned.

## Non-goals

Geographic drawing tools, saved spatial filters, lap detection, changing session summary statistics or event-list filtering, chart zoom/pan within filtered results, new backend query kinds, and native or firmware changes are out of scope.
