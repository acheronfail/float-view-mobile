# Proposal

## Why

A background disconnect can leave the iOS Live Activity displaying Reconnecting after the app is suspended and its reconnect deadline stops executing.

## What Changes

- Allow a ten-second background reconnect grace period, protected by a bounded UIKit background task.
- Remove the activity at grace expiry, background-task expiration or denied execution allowance; recover within the grace period without replacing it.
- Recreate an ended activity only once connected and foregrounded.
- Keep shared reconnect and ride-resume policy unchanged. No public contract, persistence or Android changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-hosts`: Bound iOS Live Activity presentation after background connection loss.

## Impact

Apple Live Activity controller, lifecycle policy and focused tests. No dependencies or compatibility paths are added. Non-goals: APNs, background relaunch restoration, changing BLE reconnect policy, or guaranteeing cleanup after abrupt process termination. Remove reliance on the backend reconnect timeout as the only background cleanup mechanism.
