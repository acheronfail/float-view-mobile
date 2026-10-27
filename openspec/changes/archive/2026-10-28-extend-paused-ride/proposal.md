# Proposal

## Why

Riders taking a longer break must currently reconnect and disconnect to restart the resume countdown. Let them extend the pause directly while preserving the same ride.

## What Changes

- Add Give me more time beside End ride in the shared paused-ride card.
- Add `session.extendPause` for the matching, unexpired paused ride. Restart its configured resume window from backend time without shortening an existing deadline.
- Persist the renewed deadline and replace its expiry timer without Bluetooth activity.

Non-goals: reviving expired rides, changing settings, recording during a pause, or changing reconnect policy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: Allow repeated durable pause extensions through the shared UI.
- `backend-contract`: Add the authenticated pause-extension command and authoritative snapshot behavior.

## Impact

Frontend contract, state client, mock, shared paused card, Rust product store, service and loopback dispatch, diagnostics method vocabulary, and canonical conformance. Apple and Android inherit the shared server behavior through their existing bindings; no OS effect or host-specific command is added. Existing checkpoint columns and validation remain compatible; no storage marker, archive change, dependency, or superseded production path is needed.
