## Why

Boards without battery cell counts or a cell profile currently lose their primary battery percentage even when the controller reports one. Show that reported value with an actionable warning so users retain useful information and can complete battery setup directly from Live.

## What Changes

- Prefer the shared battery estimate; when it is unavailable because saved battery configuration is incomplete, show the controller-reported percentage consistently in both live views.
- Add a small accessible warning button that opens a battery-only subset of the existing board configuration modal.
- Reuse cell profile, series/parallel counts, and display-empty voltage controls and existing revisioned profile saves. Switch back to the estimate after the authoritative snapshot reflects valid configuration.
- Preserve resolved-series per-cell voltage behavior, including its unavailable state when a count cannot be resolved.
- Replace the blanket no-fallback display branch and extract shared battery form controls rather than maintaining duplicate forms.

Non-goals: changing estimation curves or filtering, firmware settings, guest saving, BMS-only behavior, fallback for other estimate failures, recordings/exports, or persistence formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: Missing-configuration percentage fallback, source-aware presentation, and an accessible battery setup modal.

## Impact

Frontend connection widgets/detail view, Live modal ownership, board editor battery controls, focused tests, browser/mock coverage, and frontend ownership documentation. No public contract, shared Rust, Apple/Android adapter, storage schema, or dependency changes are expected; saves retain the existing cross-platform profile API and persistence semantics.
