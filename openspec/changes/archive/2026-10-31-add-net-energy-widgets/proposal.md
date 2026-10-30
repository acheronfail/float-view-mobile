# Proposal

## Why

Riders can see drawn and charged energy separately but cannot select net consumption in Live or Custom View.

## What Changes

- Offer net watt-hours and net amp-hours as drawn minus charged in both widget pickers.
- Persist the additive reading identifiers through the existing shared settings/profile contract on Apple and Android.
- Show unavailable when either counter is unavailable; retain negative net values.
- Non-goals: changing telemetry, session exports, counter resets, storage schemas, or existing selections.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `custom-board-dashboard`: Shared net consumption readings for Live boxes and Custom widgets.

## Impact

Frontend contract, derived display catalogue, shared Rust settings validation and canonical conformance coverage. Both native hosts already delegate validation and persistence to this shared service. No dependencies, migration, new telemetry fields, or superseded paths are needed.
