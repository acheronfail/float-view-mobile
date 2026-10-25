# Proposal

## Why

Riders need the peak speed of their current ride in Live and Custom dashboards, including after reconnecting or resuming the same ride.

## What Changes

- Add a shared current-ride maximum speed reading and offer it in both widget catalogs.
- Restore the peak from existing supported session summaries when resuming; reset only for a new ride.
- Non-goals: firmware statistics requests, all-time records, new storage or archive formats, and native Live Activity layout changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: preserve the observed current-ride speed peak across resume.
- `live-board-dashboard`: offer Max speed in Live and Custom with shared units and formatting.

## Impact

Additive live contract field and widget identifier, shared Rust telemetry/runtime/server, frontend client/mock and canonical conformance. Both native hosts use the shared projection. Reuse persisted session maximums without changing storage baseline or archive compatibility. No dependencies or superseded paths are needed. Extends the existing ride restore path independently of the active current-ride distance change.
