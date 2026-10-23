# Proposal

## Why

Live View provides a useful fixed dashboard with five configurable boxes, while Detailed View exposes diagnostic telemetry. Riders need a whole dashboard tailored to each board, with larger visualisations and straightforward editing that works across touch, desktop and changing viewport sizes.

## What Changes

- Add Custom View alongside Live and Detailed for saved boards, with one dashboard per board and a useful initial layout derived from existing live-box selections.
- Make searchable values the primary Add path, defaulting to Number/1×1, with separate visual panels and controls. Confirm a staged new widget before appending it; cancelling adds nothing.
- Offer ready-made widgets and explicit compatible presentation choices: numbers, bars, gauges, trend charts, battery, temperatures and footpads, plus existing headlights/LED controls. Size changes adapt a presentation without silently choosing another.
- Save widget order and bounded width/height intent (1–4 cells each), retaining recognised compact/medium/large layouts; automatically place widgets in a variable responsive grid using compact rectangular-cell footprints and ordered row packing, without fixed coordinates or separate device layouts.
- Add draft-based editing with Save/Cancel, add/remove and per-widget configuration modals. Widgets wiggle in edit mode, can be dragged anywhere on their surface, and open configuration on tap; normal widget actions are disabled. Include keyboard alternatives and reduced-motion cues.
- Move fault and Refloat-disabled warnings from the gauge into a shared parent overlay covering all three views. Keep unavailable widgets in place.
- Share one bounded frontend live-history buffer between custom charts and the existing live-chart overlay, consuming existing samples without changing telemetry collection or durable recording.
- Reuse the existing reading catalogue, renderers, board-profile revision checks and frontend interaction primitives. Remove superseded gauge-local warning markup and chart-dialog-local history ownership after extraction.

- Remember the last selected Live, Detailed or Custom view in each saved board’s profile and restore it across sessions; guest choices remain temporary.

## Capabilities

### New Capabilities

- `custom-board-dashboard`: per-board widget composition, responsive layout, compatible visualisations and accessible draft editing.

### Modified Capabilities

- `live-board-dashboard`: three eligible view choices, parent-owned warnings and shared bounded live-chart history.
- `backend-contract`: bounded optional dashboard data in revisioned Board Profiles, consistent validation/projection across hosts and mock, and supported storage/archive compatibility.
- `ui-state-restoration`: restore Custom View for the matching saved board while excluding editing drafts, drag state and chart samples.

## Impact

Changes touch the shared Svelte frontend and reading/chart components, `frontend/src/lib/backend/contract.ts`, client/mock, shared Rust product-store/service/loopback projection and checkpoint validation, and canonical conformance coverage for Apple and Android. Persistence adds optional board-profile data through the existing authenticated operations; no new host Bluetooth APIs, telemetry requests, background work or runtime owner are needed. Preserve existing five-slot Live View settings, supported storage markers and baseline archives; an additive JSON field must not trigger a reset or an unnecessary format bump. Verify actual profile/archive byte limits before implementing the bounded schema. No new runtime dependency is planned.

## Non-goals

Multiple named dashboards per board, dashboard sharing/templates/import UI, a guest dashboard editor, standalone BMS custom views, arbitrary coordinates or gaps, separate per-device/breakpoint layouts, automatic presentation switching, arbitrary formulas or user-defined widgets, new firmware support, longer durable chart history, and hinge-specific native APIs. Respond to the usable viewport supplied by the host; do not claim detection of an unreported physical hinge. No application implementation or Git commits are part of proposal preparation.
