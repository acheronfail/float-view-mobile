## Why

The implemented reported-percentage fallback and Live battery modal keep incomplete boards useful, but setup still requires riders to understand cell profiles and series/parallel counts. Add familiar battery presets with Basic/Advanced setup so a rider can select their installed battery in one step while retaining manual control.

## What Changes

- Prefer the shared battery estimate; when it is unavailable because saved battery configuration is incomplete, show the controller-reported percentage consistently in both live views.
- Add a small accessible warning button that opens a battery-only subset of the existing board configuration modal.
- Add a two-segment Basic/Advanced control to the shared battery form. Basic uses one manufacturer-grouped battery select and a short configuration summary; Advanced exposes series/parallel counts, cell profile, optional pack capacity, and display-empty voltage.
- Apply named, variant-specific presets through the shared revisioned profile API, including nominal capacity. Use the generic profile internally when no dedicated curve exists, without an approximation label, warning, or extra confirmation in Basic.
- Preserve drafts and custom settings when switching modes, retain the selected battery name across recreation, and save preset fields atomically. Switch back to the estimate only after the authoritative snapshot reflects valid configuration.
- Preserve resolved-series per-cell voltage behavior, including its unavailable state when a count cannot be resolved.
- Replace the blanket no-fallback display branch and extract shared battery form controls rather than maintaining duplicate forms.

Non-goals: changing estimation curves or filtering, firmware settings, guest saving, BMS-only behavior, fallback for other estimate failures, recordings/exports, or persistence formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: Missing-configuration percentage fallback, source-aware presentation, and shared Basic/Advanced battery setup in the full editor and contextual modal.

## Impact

Frontend connection widgets/detail view, Live modal ownership, shared board battery controls, focused tests, browser/mock coverage, and frontend ownership documentation. This change owns the UI; `stabilize-battery-state-of-charge` owns the new shared preset catalog, preset identity/selection contract, optional capacity persistence, and affected host/conformance updates. Land that contract/catalog slice before the new preset UI; the already implemented fallback/modal remains valid meanwhile. No additional dependencies or storage format changes are introduced by this UI change.
