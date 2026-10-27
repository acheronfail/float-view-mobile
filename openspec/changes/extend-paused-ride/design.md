# Design

## Context

See proposal.md for motivation. The shared service owns a singleton durable ride checkpoint and one RideResume timer. Its existing checkpoint validation requires deadline minus pause timestamp to equal window minutes.

## Goals / Non-Goals

Keep extension policy and persistence in shared Rust and expose it through the existing authenticated loopback server used by Apple and Android. Keep the frontend dependent on complete authoritative snapshots. No new native effects, schema, dependency or BLE operation is needed.

## Decisions

- Use `session.extendPause` with a session identity, parallel to `session.end`. Service dispatch already expires overdue rides before commands, preventing revival.
- Compute max(existing deadline, now + configured window). If unchanged, preserve the checkpoint. Otherwise atomically update its existing window, pause timestamp and deadline together. The pause timestamp represents the latest window anchor, not recording provenance; recording breaks retain their separate events. This preserves current validation and supported data without a schema migration.
- Replace the RideResume timer after persistence and reload the checkpoint before publishing. Old timer IDs become inert. Keep one timer regardless of repeated taps.
- Reuse the shared PausedRide card, primary-pointer/keyboard helpers and pending/error presentation. Extend needs no destructive confirmation.
- Mirror policy and timer replacement in the mock. Add canonical coverage exercised against the shared server and mock; both native hosts reuse that exact server through existing bindings.

## Risks / Trade-offs

- Extension at expiry → backend time decides; expired rides cannot be revived.
- Storage failure → atomic SQL update and no success snapshot before persistence.
- Old timer callback → cancel logical timer identity and test at old and renewed deadlines.
- Shorter settings → preserve the later deadline; no accidental shortening.

## Migration Plan

No data migration. Existing readers accept the updated checkpoint invariant. Ship frontend and shared server together through existing host packaging.
