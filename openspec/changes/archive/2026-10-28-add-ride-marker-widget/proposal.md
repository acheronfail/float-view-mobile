# Proposal

## Why

Custom View has no direct way to mark a ride. A visible button makes capture discoverable and usable with one tap.

## What Changes

- Add a configurable Save ride marker button with pending, confirmed and failed feedback.
- Extend dashboard widgets with an additive action variant supported by the frontend/mock and shared Apple/Android service.
- Preserve existing dashboards and marker capture policy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `custom-board-dashboard`: Add an accessible marker action widget and its feedback.

## Impact

Frontend contract, picker, configuration, renderer, shared product-store validator and canonical conformance tests. No new dependencies or storage migration; existing data remains valid. No superseded paths. Non-goals: changing gauge gestures, marker storage, admission policy or native adapters.
