# Proposal

## Why

The native edge gesture works but gives no feedback until release, making it hard to discover and cancel confidently.

## What Changes

- Track the finger with a viewport-sized moving page layer and a quiet back affordance underneath.
- Settle back on cancellation or blocked navigation; slide away only after shared Back succeeds and parent presentation is ready.
- Preserve native recognition, logical parent navigation, save/dismissal guards and list restoration.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: interactive feedback for the existing iOS edge Back gesture.

## Impact

iOS presentation and shared frontend Back completion reporting, with focused native/browser tests. No backend contracts, storage, new dependencies or other-host gesture changes. Replace the release-only native dispatch with an acknowledged transition. Non-goals: a live parent preview while dragging, a native route stack or retaining additional WebViews.
