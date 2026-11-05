# Proposal

## Why

Nested iOS views have back controls but no familiar left-edge swipe. Add native gesture recognition while preserving shared route and dismissal behavior.

## What Changes

- Enable a native left-edge back gesture while a nested frontend back destination is available.
- Commit a deliberate completed swipe through the existing shared Back action; ignore short, vertical, reversed and cancelled gestures.
- Cover the availability boundary and native gesture navigation, including preserved scroll and overlay guards.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: iOS edge-swipe return behavior.

## Impact

iOS WebView, a closed frontend-to-host presentation capability, shared shell, and focused browser/native UI tests. No backend contract, storage, Android or macOS behavior changes. No new dependencies or superseded paths. Non-goals: interactive page dragging, a native navigation stack, browser-history gestures and general swiping outside the edge.
