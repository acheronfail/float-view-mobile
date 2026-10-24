# Design

## Context

See proposal.md. The existing recording boundary stores exact sample endpoints but no cause. A writer can detach or the process can exit before another sample arrives. Ride-checkpoint pause timestamps also occur for automatic reconnects and cannot establish intent.

## Goals / Non-Goals

Keep interruption classification in the shared service and durable session history. Preserve the existing generic gap flag for integration/trace discontinuities independently of pause presentation. Do not infer from timestamps, connection contexts, speed or duration.

## Decisions

- Persist a `recordingInterrupted` event anchored to the last committed sample before detaching. Store a closed reason: `pause` for explicit disconnect/native user stop; `connectionLoss` for automatic reconnect; `unknown` for other interruptions. Repeated notifications are idempotent; an explicit pause may supersede a connection loss while no samples arrive. Bound these events to 2,000 (two per supported boundary) and fail without partial writes at capacity.
- On reopening, recover the pending event only if it belongs to the current sample tail and follows the latest resumed boundary. Resume copies its reason into `recordingBoundary`. Without a pending known cause, resume records `unknown`. Attempt to publish event commits through the existing runtime/service marker path before detach. An unsuccessful reason write never creates deliberate-pause provenance.
- Reserve native disconnect, timer cancellation and location stop before fallible cleanup. Run reason persistence, writer release, catalog finalization, runtime reset and disconnected-state publication independently, returning the first cleanup error only afterwards. Explicit disconnect bypasses expiry/catalog preflight and ordinary queue admission; it uses the existing reserved cleanup slots. Native stop follows the same service path. Do not add retries, deferred writers or another application owner.
- Writer detach clears both its ownership slot and published marker even on checkpoint failure, and always attempts to close the writer. Failed catalog writes preserve committed data; logical paused-ride state may remain in memory so a temporary failure does not require another live connection or invent a historical reason. Restart uses existing recovery.
- Extend the existing compact metadata tuples to `[startElapsedMs,endElapsedMs,reason]`. Keep all interruptions available, with bounded validated endpoints and causes. The shared Rust server projects them for both native hosts; native bindings and adapters remain unchanged.
- The frontend passes only `pause` intervals to the existing reversible compact timeline and all its consumers. Other gaps retain original width and disconnected traces, including map-filter composition. Remove unconditional compaction rather than adding another axis.
- Use additive event kinds/details in the existing schema and archive format 4. Missing historical reasons map to `unknown`; malformed or unrecognised reasons fail actionably. Do not rewrite old events, reset storage, infer provenance or add a migration.

## Risks / Trade-offs

- A crash before recording the interruption cannot establish intent → retain unknown; a committed interruption survives reopen before the first resumed sample.
- Storage failure may lose a pause marker → prioritize disconnect; retain unknown history instead of blocking the user or guessing a cause.
- Compact tuple growth approaches message bounds → verify worst-case metadata size alongside the existing 1,000-boundary limit.
- Existing archives will no longer collapse their unknown breaks → intentional compatibility behavior; preserve import, prune and re-export support and original timestamps.

## Migration Plan

No schema or archive upgrade. Ship frontend and shared backend together. Preserve frozen baseline fixtures. Verify new and existing archive round trips; leave historical OpenSpec archives intact. No commits are made without the user's explicit authorization.
