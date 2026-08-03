## Why

Historical charts currently render one field per plot and derive every y-axis from the values in the visible range. That makes bounded metrics such as duty cycle look misleading when the chart never shows their full operating range, and it makes related telemetry such as motor and controller temperatures harder to compare. The existing field metadata is the natural place to describe these presentation rules before the chart implementation grows more special cases.

## What Changes

- Extend session field presentation metadata with chart-group membership, optional fixed axis bounds, and display formatting/scaling rules.
- Define the duty-cycle chart as a fixed 0–100% axis while preserving the backend’s ratio values, and use the same metadata path for other bounded fields as they are identified.
- Define a shared temperature chart containing motor temperature and controller temperature, with one y-axis calculated from the visible values of all enabled lines.
- Render a legend below every multi-line chart, using stable per-field colors and native button semantics.
- Let users click or tap a legend item to hide/show its associated line. Hidden items remain in the legend with a greyed-out, strikethrough appearance and an accurate pressed/hidden accessible state.
- Keep hidden-line state local to the chart view, preserve it across range/resize refreshes, and reset it when the chart grouping or session route is recreated.
- Add focused geometry, metadata, and component interaction coverage for fixed domains, shared domains, line visibility, keyboard activation, touch activation, and no-double-activation behavior.

Non-goals:

- No backend contract, native implementation, persistence schema, export, or conformance protocol changes.
- No new charting dependency and no replacement of the existing SVG renderer.
- No arbitrary user-created groups, cross-axis scales, or independent axes within one chart in this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: historical chart requirements now include metadata-defined chart composition, fixed/display-aware axes, and accessible interactive legends for multi-series plots.

## Impact

- `frontend/src/lib/features/sessions/session-fields.ts` becomes the source of truth for chart groups, axis policies, value display formatting, and line presentation metadata.
- `frontend/src/lib/features/sessions/SessionHistoryChart.svelte` will render chart groups rather than one chart per field and will own visibility and legend interaction state.
- `frontend/src/lib/features/sessions/history-geometry.ts` and its tests will accept an explicit shared domain so all visible lines use the same y mapping.
- Frontend unit/component tests will cover the new chart behavior. The session-data wire shape, native backends, persistence, and mobile transport remain unchanged.
