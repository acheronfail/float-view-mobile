# Proposal

## Why

An accidental End ride tap must not split the ride unintentionally.

## What Changes

- Confirm every End ride action, including idle and paused rides.
- Place End ride after Disconnect in the Live menu.
- Non-goals: changing Disconnect, recording, persistence or Bluetooth policy. Remove the immediate-completion UI paths; add no dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: confirmation and menu ordering.
- `live-board-dashboard`: End ride always confirms independently of Disconnect safety confirmation.

## Impact

Shared frontend and browser coverage only; no contract, storage or native host changes.
