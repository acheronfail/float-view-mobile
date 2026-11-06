# Design

## Context

See proposal.md. The controller serializes ActivityKit submissions but currently depends on in-process timers without an execution assertion. ActivityKit stale dates do not dismiss activities.

## Goals / Non-Goals

Bound presentation after background loss using existing native ownership. Do not change shared service contracts, reconnect decisions, storage or Android.

## Decisions

- Use a ten-second monotonic grace deadline, beginning when an existing reconnecting activity is observed outside the foreground. Keep this constant with the tested lifecycle policy.
- Acquire one UIKit background task before awaiting ActivityKit updates. Denial ends immediately. A separate grace timer and the expiration handler independently request immediate ActivityKit end and release the assertion after completion. Do not rely on the serial worker being free to process expiration.
- Detach the activity before awaiting end and guard post-update state by activity identity. Capture assertion identifiers and per-allowance tokens so old callbacks cannot affect new work.
- Release on recovery, foreground, dismissal and connection replacement. Repeated snapshots cannot renew the deadline. Never start an activity while reconnecting.
- Keep the existing activity during foreground reconnect. Ending immediately on every loss would unnecessarily disrupt brief radio gaps; waiting for backend timeout would retain the suspension failure.

## Risks / Trade-offs

- No automatic activity recreation following a later background recovery; foreground connected state recreates it.
- UIKit execution is bounded and may be denied; denial and expiration take the immediate cleanup route. Abrupt process termination can still leave an orphan until existing startup reconciliation or OS removal.
- Physical iPhone checks must establish locked-device grace expiry, recovery within grace, foreground transitions and independent reconnect behavior. Software tests cover decisions and iOS compilation only.

## Migration Plan

No data migration. Rollback is confined to native presentation files.
