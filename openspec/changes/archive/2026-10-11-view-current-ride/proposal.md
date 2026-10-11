# Proposal

## Why

Riders need to inspect charts for the current ride without disconnecting. The existing persisted-session reader already supports active snapshots; the frontend needs a discoverable entry point and a viewer that follows growth without disrupting inspection.

## What Changes

- Add Current ride entry points on Live and Sessions for saved connections.
- Reuse session review with Recording/Reconnecting status, Back to Live, automatic visible-page refresh, and Jump to live.
- Preserve inspection, field visibility and map position across snapshot refreshes; remove snapshot-keyed viewer remounts.
- Non-goals: guest history, new telemetry buffers, native recording changes, new settings, API or storage changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: current-ride access and ongoing session review behavior.

## Impact

Frontend connection navigation, session review and backend-client request ownership; browser/mock coverage. No new dependencies, cross-platform API changes, migrations or persistence changes.
