# Proposal

## Why

Live distance resets when the board restarts even though the current ride resumes. The dashboard and Live Activity should retain the distance already travelled in that ride.

## What Changes

- Display accumulated current-ride distance in all live distance consumers, including Live Activity and Dynamic Island.
- Restore saved distance when resuming and add travel only across observed continuous samples using the same speed integration as saved session summaries.
- Keep raw board trip distance in historical samples. Non-goals: GPS distance, firmware changes, storage migrations, and changing odometer behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: retain and advance live current-ride distance across reconnects and process replacement.

## Impact

Shared telemetry runtime, backend live projection, client/mock semantics, canonical conformance and live activity projection shared by Apple and Android. The existing live `distanceM` field changes from board trip distance to current-ride distance; recorded telemetry and supported storage remain compatible. Remove the direct raw board distance projection from live consumers. No dependencies or compatibility machinery are added.
