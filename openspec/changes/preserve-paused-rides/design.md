# Design

## Context

See proposal.md. Sessions currently belong to the connection; explicit disconnect resets the runtime. SQLite summaries also integrate across missing intervals.

## Goals / Non-Goals

Keep one resumable ride in the shared Rust owner, including process replacement. No native product logic, completed-session merging, new dependencies or general migration machinery.

## Decisions

- Persist a singleton recording checkpoint with controller identity, session identity, timeline and resume deadline. A bounded session owner survives connection teardown; a verified matching activation resumes it. Different-board/controller activation finalizes first. Bluetooth retries remain independent; explicit disconnect suppresses them.
- Add `rideResumeWindowMinutes` (60, integer 1–1,440), `pausedRide` state and identity-scoped `session.end`. Keep completion idempotent against duplicate frontend activation and reject stale identities.
- Record explicit interruption boundaries and carry forward sample/event cursors and elapsed origin; summaries, pruning, charts and maps must respect boundaries. Never synthesize offline telemetry.
- Preserve baseline 100 and archive compatibility. Any incompatible catalog/session schema extension uses an explicit transactional, restartable 100-to-next-marker upgrade, with marker publication after commit and frozen fixture coverage. No reset or pre-baseline fallback.
- Show one reusable paused-ride status/action on Home and the session viewer. Live End ride uses existing safety confirmation; paused completion is immediate. Settings changes apply to future pauses, keeping an existing deadline stable.

## Risks / Trade-offs

- Wall-clock changes across process restart → validate persisted timestamps; never extend an existing deadline because the clock moved backwards.
- Crashes between session and catalog writes → recover only committed boundaries, preserve data, and reconcile before resuming.
- Charging may reboot a controller → allow the verified identity to resume while resetting connection-scoped firmware counters.
- Long pause accidentally joins rides → show remaining eligibility and provide End ride; cap at 24 hours.
