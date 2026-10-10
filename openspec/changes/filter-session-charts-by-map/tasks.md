# Tasks

## 1. Geographic intervals and chart projection

- [ ] 1.1 Add gap-aware route clipping against normalized map bounds; unit-test re-entry, outside-endpoint crossings, boundary tangencies, isolated points, GPS gaps and antimeridian wrapping.
- [ ] 1.2 Add a shared included-duration projection with original-time inverse lookup; unit-test unequal interval lengths, separator ownership, round trips, empty results and ordinary single-range behavior.
- [ ] 1.3 Integrate projection into paths, tooltips, ticks and chart marker positioning; verify tests prevent bridging spatial breaks while preserving existing telemetry generation/null gaps and value-axis policies.

## 2. Complete and bounded data loading

- [ ] 2.1 Replace silent four-page location truncation with complete cursor traversal and the documented total budgets; verify more-than-four-page tracks, repeated cursors, terminal errors, isolated-only tracks and limit states in focused tests.
- [ ] 2.2 Extend internal series request ownership for one cancellable multi-interval generation, retaining the wire contract; verify interval requests coexist, at most two run concurrently, and superseding generations abort or ignore old responses.
- [ ] 2.3 Allocate bounded buckets by included duration and publish complete field-group results under the documented memory/interval limits; test late failures, partial completion, visible-group loading, cache reuse and limit reporting.
- [ ] 2.4 Verify one-snapshot results, one automatic metadata refresh per failed generation, explicit transport retry and teardown cleanup using session-controller/mock tests.

## 3. Session interaction and presentation

- [ ] 3.1 Add the transient map-filter toggle and session-owned coordination; verify off-by-default behavior, current-bounds activation from a zoomed chart, previous-viewport restoration, hide-map behavior and session reset with component tests.
- [ ] 3.2 Disable chart zoom/pan controls and every viewport-changing input while retaining inspection and field visibility; test primary/non-primary pointer activation, cancellation, keyboard/assistive activation, wheel and pinch without double activation.
- [ ] 3.3 Connect settled map changes to interval selection without automatic recenter feedback; verify duplicate move/zoom notifications, full-route reset, active-gesture cancellation and rapid map changes in interaction tests.
- [ ] 3.4 Render aligned dashed separators, original ride timestamps and included-only automatic scales; verify empty/loading/error views and outside-filter marker/fault selection without navigation changes.
- [ ] 3.5 Document transient mode ownership, limits and unchanged summary/event semantics in the frontend ownership guide; review the text against implementation and ensure UI labels do not imply those sections are spatially filtered.

## 4. Integration verification

- [ ] 4.1 Add and run affected mock browser coverage for desktop and touch: repeated visits, aligned charts, hover/tap inspection, disabled navigation, toggle/reset, route-tail inclusion, no feedback loop and stale-result rejection; build the current frontend before browser checks as required by README.md.
- [ ] 4.2 Run affected frontend Vitest tests, `npm run check`, and relevant formatting/lint checks from `frontend/`; record commands and outcomes, resolving regressions in ordinary chart zoom/pan and map synchronization.
- [ ] 4.3 Validate the OpenSpec change strictly and review the final diff for unchanged public API/native/persistence scope; report actual verification evidence before completing the change.

## Workflow follow-up

- Commit completed planning artifacts before implementation; make coherent verified implementation commits with corresponding task updates.
- After affected verification passes, sync delta specs and archive together in a final commit. Do not treat historical migration acceptance gates as requirements for this frontend change.
