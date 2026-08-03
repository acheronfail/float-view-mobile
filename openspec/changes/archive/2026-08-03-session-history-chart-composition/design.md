## Context

`SessionHistoryChart.svelte` currently has two different notions of grouping: request groups are bounded batches used by the progressive loader, while the rendered UI creates one SVG chart for every field. Each rendered field independently derives a padded domain, and `history-geometry.ts` derives that domain again while building paths. `session-fields.ts` currently supplies only a display name and unit, so axis behavior, colors, and chart composition would otherwise become field-name conditionals in the component.

The session-data endpoint already returns independent columnar fields. This change therefore belongs entirely to the frontend session feature. The request grouping and bounded response model remain unchanged; a visual chart group can contain columns loaded by different request groups and can render incrementally as those columns arrive.

## Goals / Non-Goals

**Goals:**

- Make field presentation metadata the single source of truth for visual group membership, field color, axis policy, and value formatting.
- Keep raw backend values unchanged while allowing display-aware axes and labels, including duty-cycle ratios rendered as 0–100%.
- Render one shared y-axis and one SVG plot for compatible fields such as motor and controller temperature.
- Make every line independently discoverable and toggleable through an accessible legend that works with mouse, touch, keyboard, and assistive technology.
- Preserve bounded SVG/path behavior, exact-tooltip semantics, shared viewport state, and progressive loading.

**Non-Goals:**

- No backend, native, persistence, export, or wire-contract change.
- No arbitrary user-defined chart composition or support for multiple y-axes in one plot.
- No chart-library dependency, canvas migration, or persistent visibility preferences.

## Decisions

### 1. Extend field metadata and derive visual groups from it

Add chart presentation metadata beside each known field in `session-fields.ts`. The metadata should identify a stable group key, a line color token, an axis policy, and a value formatter. A field without chart metadata falls back to a single-field group with the current automatic numeric formatting, so extension fields and newly negotiated fields remain renderable without editing the chart component.

The group key is a presentation concern and is intentionally separate from the progressive request-group index. The initial shared group is `temperature` for `motorTempC` and `controllerTempC`; fields that do not have compatible units or scale remain in separate groups. Group title and unit are derived from the group’s fields, with a deterministic fallback for extension fields.

Alternative considered: maintain a second hard-coded map inside `SessionHistoryChart.svelte`. Rejected because it would duplicate the field catalog and make future fields easy to forget when adding a chart, axis, or legend rule.

### 2. Use an explicit raw-value axis policy with display formatters

Represent an automatic axis separately from a fixed axis `{ min, max }`. `finiteDomain` (or a focused successor) accepts that policy and returns fixed bounds verbatim, without padding or expanding them from the visible samples. Automatic domains retain the existing finite-value filtering and deterministic constant-series padding.

Axis ticks and exact tooltip values use the same field formatter. For duty cycle, the stored ratio remains in the column and geometry uses a fixed raw domain of `0..1`; the formatter labels those endpoints as `0%` and `100%` and formats exact values consistently. This avoids mutating backend data or mixing display units into the transport contract. Add fixed policies only for fields whose source semantics are verified; start with duty cycle and other clearly bounded ratio fields, leaving uncertain fields automatic.

Alternative considered: multiply values into percentages before creating chart columns. Rejected because it duplicates or mutates source data, complicates exact-value handling, and makes generic geometry less reusable.

### 3. Compute one domain per chart group from all its columns

For an automatic group, combine finite values from every loaded column in the group and derive one domain. The domain remains based on all group columns even when one line is hidden, so toggling a legend item does not rescale the remaining line or move the crosshair unexpectedly. Fixed group policies continue to win over sample values. A group with no visible lines still retains its axes and legend, allowing the user to restore any line.

Pass the resolved domain explicitly to `buildPathSegments` and `pointAtElapsed`; neither helper should recalculate a per-line domain. Rendered paths, guide lines, tick labels, zero references, and inspection points must all use the same mapping.

Alternative considered: recompute automatic domains from visible lines only. Rejected for the surprising scale jump on every toggle and the loss of visual comparability between the two temperature lines.

### 4. Keep visibility state local and keyed by field name

Store hidden fields in a route-local reactive set keyed by the field name. A legend action toggles only its field and does not affect requests, cache keys, viewport, or other groups. When a hidden field is the active exact-tooltip field, switch inspection to the first visible field in that chart group or dismiss inspection if none remain. New series responses preserve visibility by field key; route/session recreation starts with all lines visible.

The legend is rendered below a chart only when the group has more than one field. Each item is a native `button type="button"` with the line swatch, field label, `aria-pressed` reflecting visibility, and an accessible label describing whether activation will show or hide the line. Hidden labels use greyed styling and `text-decoration: line-through`; state is not conveyed by color alone. Pointer activation uses the existing primary-pointer release helper and the guarded keyboard/assistive click path so a touch activation cannot double-toggle.

Each chart group renders one tooltip at the shared inspection x-position. Its content contains one block per visible field, ordered like the legend, separated by line breaks, with each block colored using that field’s line token. The tooltip anchor uses the common x coordinate while each field’s marker uses its own y coordinate, so every displayed tooltip value has a visible dot on its corresponding line. Hidden fields are omitted from both the marker and the consolidated tooltip.

### 5. Preserve progressive loading and interaction boundaries

The existing loader continues to request fields in bounded chunks. The render layer maps the ordered metadata fields to visual groups after each response and draws a group whenever at least one of its columns is available. Missing columns show the existing waiting state and do not block other groups. Chart SVGs keep the existing `touch-action: pan-y` and pointer gesture handlers; legend buttons live outside the SVG and do not participate in plot gestures.

Use stable metadata colors through existing theme CSS variables or a small set of new light/dark-safe chart variables in `frontend/src/app.css`; do not add a color package. The SVG path stroke and legend swatch use the same token, while accessible text and line visibility provide the non-color distinction.

## Risks / Trade-offs

- [A fixed domain can clip an unexpected out-of-range sample] → Apply fixed bounds only to verified bounded fields, test the policy against fixture values, and keep automatic domains for uncertain telemetry.
- [A shared domain can make a small series harder to see beside a larger series] → Restrict grouping to compatible units/scales, keep one shared axis by design, and let users hide a line without changing the stable group scale.
- [Multiple request groups can leave a combined chart partially loaded] → Render available lines independently, keep the legend complete from metadata, and retain a bounded waiting state for missing columns.
- [Line colors may lose contrast across themes] → Reuse or add theme-aware CSS variables and verify visible and hidden legend states in both themes; never rely on color alone.
- [A hidden active field can leave inspection state inconsistent] → Move the active field to a visible sibling or clear it atomically when toggling visibility.
- [The active performance change also touches historical chart requirements] → Keep this delta frontend-only and reconcile its live-board-dashboard requirements with `performance-session-visualisations` before syncing or archiving either change.

## Migration Plan

1. Add and test the metadata/geometry model, then update the chart to derive visual groups while preserving the existing request groups.
2. Add fixed duty-cycle formatting/domain behavior and the initial temperature group.
3. Add legend rendering, guarded input handling, visibility state, and focused frontend tests.
4. Run the focused frontend checks and `just verify`; if the overlapping performance change is still active, reconcile the two deltas before archive.

Rollback is a frontend revert. No stored data, native code, or protocol compatibility path is affected.

## Open Questions

None for the first implementation. Additional fixed axis policies should be added only after their telemetry semantics are confirmed.
