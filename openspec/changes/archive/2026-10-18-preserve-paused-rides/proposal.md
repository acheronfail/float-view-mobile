# Proposal

## Why

Brief disconnects and charging breaks currently split one ride into multiple sessions. Keep the ride resumable independently of its Bluetooth connection.

## What Changes

- Pause saved rides on disconnect; resume the same verified board/controller within a configurable 1–1,440 minute window (default 60), including after process restart.
- Add End ride to the Live menu, a paused-ride Home card, and the paused session viewer. End ride closes and disconnects immediately; disconnected completion needs no confirmation.
- Preserve offline gaps without inventing riding time, distance, telemetry or GPS paths. Apply retention only at final closure.
- Keep Bluetooth retry timing separate. Non-goals: merging completed sessions, guest recording, new dependencies or a general migration framework.

## Capabilities

### New Capabilities

- `ride-continuity`: bounded durable pause/resume, ending rides, gap-safe summaries and shared UI.

### Modified Capabilities

- `backend-contract`: resume setting, paused-ride snapshot and End ride request; saved activation may resume a session.
- `live-board-dashboard`: distinguish Disconnect from End ride and update safety confirmation.

## Impact

Shared Rust service/store/telemetry/history, the hand-written contract/client/mock, shared Home/Live/Settings/session UI and canonical conformance. Both native hosts consume the same shared service. Replace connection-owned session lifetime and misleading disconnect copy; preserve baseline storage and archive imports.
