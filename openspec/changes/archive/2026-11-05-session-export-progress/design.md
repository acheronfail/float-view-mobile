# Design

## Context

Bulk exports already use cancellable shared file workers. Web downloads use HTTP while native exports await a control response, so progress and cancellation must remain callable independently of either in-flight request.

## Goals / Non-Goals

Add bounded, ephemeral progress for the existing board export. No persisted jobs, generic scheduler or host API changes.

## Decisions

An optional opaque operationId on board exports correlates authenticated session.export.status and session.export.cancel controls across HTTP and WebSocket connections. Retain one bounded operation in the process owner; reject replacement while its worker runs. Status contains only operation ID, phase and completed/total counts. Worker completion advances counts after each encoded/copied artifact; completion and cancellation use the existing transfer cancellation/cleanup path. Status polling is sequential and bounded in frequency; it does not increment product revisions or broadcast snapshots. No session-ID collections or filesystem paths enter progress.

The shared frontend opens a modal immediately, polls while preparing, and requests cancellation without discarding the in-flight request. Cancellation remains pending until the worker exits, so repeat export cannot race its resources. A late cancellation after preparation is reported as too late; native save/share retains its own cancellation. Web download cancellation aborts delivery after preparation when needed. Errors and final outcomes remain visible; polling failures never invent progress or success.

## Risks / Trade-offs

- Session counts do not estimate within-session encoding time: show an indeterminate initial stage and honest completed-session counts.
- Polling may start before the HTTP request registers: treat bounded not-found retries as preparation, while export response remains authoritative.
- Cancellation races completion: serialize acceptance with file completion under the service owner lock; suppress presentation only for accepted cancellation.

## Bulk size and storage

User-selected scope: one larger ZIP with free-space checks. Bulk ZIP output is bounded at 2 GiB and overall staged reservations at 3 GiB; individual artifacts/import compatibility retain the existing 128 MiB bound. Preflight estimates total output plus reusable scratch and 64 MiB free-space headroom, and checks again before each session. Storage can change after preflight, so write failures still fail atomically and clean up. Record actionable bounded errors in progress even when admission fails. Tracked preparation has a ten-minute deadline rather than the existing 30-second idle transfer deadline.

Use rustix's safe statvfs wrapper (already available offline) to preserve the product-store unsafe-code prohibition across Apple/Android, rather than implementing unsafe syscall bindings. No archive/persistence version change is needed.
