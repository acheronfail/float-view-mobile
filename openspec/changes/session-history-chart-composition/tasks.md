## 1. Chart presentation metadata

- [x] 1.1 Extend `session-fields.ts` with typed chart metadata for group identity, line color token, automatic/fixed axis policy, and value formatting, while keeping a safe single-field fallback for unknown extension fields.
- [x] 1.2 Define the initial metadata policies for the duty-cycle ratio and the shared motor/controller temperature group; add only additional fixed bounds whose source semantics are verified.
- [x] 1.3 Add pure metadata helpers that preserve field order, assemble visual groups independently from progressive request groups, derive group titles/units, and handle missing or extension fields deterministically.

## 2. Shared chart geometry and formatting

- [x] 2.1 Update finite-domain and axis-tick calculation to accept explicit fixed or automatic policies, preserving constant-series padding, zero references, null handling, and deterministic output.
- [x] 2.2 Update SVG path and exact-point geometry to accept the resolved group domain rather than deriving a separate domain per field; keep continuity-generation and time-gap breaks intact.
- [x] 2.3 Implement the shared field-value formatter used by y-axis labels and exact tooltips, including raw duty-cycle ratios displayed as percentage values without changing the session-data columns.
- [x] 2.4 Add geometry and formatter tests for fixed 0–1 duty domains labeled 0%/100%, shared temperature domains, constant series, mixed nulls, and values outside automatic padding.

## 3. Render visual chart groups

- [x] 3.1 Refactor `SessionHistoryChart.svelte` to map the metadata-defined visual groups over the existing bounded request-group results, rendering a group as soon as any of its requested columns is available.
- [x] 3.2 Resolve one domain per chart group from all loaded columns and use it consistently for every line, grid guide, tick, zero reference, path, and inspection point; retain the existing progressive loading, viewport, and stale-response guards.
- [x] 3.3 Render each field as a separately colored SVG path within its visual group, preserve broken/null segments, and retain the existing single-field layout as the fallback for ungrouped fields.
- [x] 3.4 Add theme-safe line colors and matching legend swatches using existing CSS variables or minimal chart variables; verify light and dark theme contrast without introducing a dependency.

## 4. Interactive legends and visibility state

- [x] 4.1 Add route-local field-keyed visibility state that preserves toggles across series refreshes and viewport changes, resets on route/session recreation, and never changes request fields, cache keys, or viewport state.
- [x] 4.2 Render a legend beneath every multi-line chart with native buttons, stable field labels, matching swatches, `aria-pressed` visibility state, and accessible show/hide labels; render hidden items greyed and struck through without relying on color alone, and render one consolidated per-chart tooltip with newline-separated field-colored values.
- [x] 4.3 Wire legend buttons through guarded primary-pointer release and keyboard/assistive-click activation so primary mouse/touch/pen activation toggles once, non-primary/cancelled input does nothing, and plot gestures are not started.
- [x] 4.4 Reconcile hidden-field toggles with exact inspection: move the active field to a visible sibling or dismiss it, cancel/ignore stale hidden-field requests, preserve the shared crosshair for visible lines, and keep the chart/legend usable when all lines are hidden.

## 5. Focused verification and handoff

- [x] 5.1 Add pure state/metadata tests for group ordering, fallback fields, visibility toggles, all-hidden groups, route reset, and active-field reassignment; add the focused browser/component coverage available in the existing frontend test workflow for pointer, keyboard, touch, assistive activation, consolidated tooltips, colored value lines, and matching inspection markers.
- [x] 5.2 Run `just test frontend` and review the session chart in light/dark themes for duty-cycle endpoints, combined temperature rendering, legend state, progressive partial loading, and zoom/pan/crosshair behavior.
- [ ] 5.3 Reconcile this delta with the overlapping `performance-session-visualisations` chart requirements, then run `just verify` as the complete software gate.
