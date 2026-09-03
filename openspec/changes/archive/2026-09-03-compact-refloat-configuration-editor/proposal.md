## Why

Configure Refloat currently renders every schema subgroup and every field description in one long surface, making the generated editor difficult to scan and cumbersome on mobile. Reusing the schema's existing subgroup structure for page selection and moving help into on-demand dialogs will make the experiment substantially more compact without weakening its typed, schema-driven behavior.

## What Changes

- Replace the vertically stacked subgroup list with a native select input that displays one logical schema subgroup page at a time, including aggregation of backend transport chunks that belong to the same group and subgroup.
- Keep search within the selected subgroup while preserving complete-editor draft, validation, dirty-state, and write-diff behavior across all subgroups.
- Remove always-visible field help text and add a `(?)` help button beside every field label that opens a focused informational modal.
- Preserve native control semantics, primary-pointer and keyboard/assistive activation, modal focus management, and inline validation errors.
- Remove the superseded all-subgroups rendering path and inline field-description presentation.
- Non-goals: changing the frontend/backend contract, schema pagination, native backends, Refloat serialization or safety policy, Tune storage, exposing upstream XML separator rows, or hiding internal schema subgroups.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: Refine the generated Refloat configuration experience to use selectable subgroup pages and on-demand field-help dialogs.

## Impact

- Affects the Svelte generated Refloat configuration editor, field presentation, supporting controller projections, and frontend tests under `frontend/src/lib/features/connection/`.
- No cross-platform API, native backend, persistence, protocol, dependency, or compatibility-catalogue changes are required.
