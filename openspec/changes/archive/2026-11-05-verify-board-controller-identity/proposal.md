# Proposal

## Why

A known Bluetooth transport can identify a different motor controller. The current local-association fallback assigns unknown controllers to the old board, while a controller matching another saved board causes a conflict. Recording ownership must follow verified controller identity across platforms and shared transports.

## What Changes

- Resolve saved boards exclusively through the selected controller's validated UUID-derived identity.
- Reuse the matching saved/imported board or create a separate board with a default profile.
- Atomically detach the transport from a different board before associating it with the verified board; preserve all old profiles and history.
- Reject missing/invalid identification without creating or reassociating boards.
- Remove the Bluetooth-based ownership fallback. No history merging, controller aliases, replacement workflow, storage/schema migration or app-transfer implementation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `saved-board-session-history`: Verified controller ownership and safe local reassociation.

## Impact

Shared Rust product-store matching, service activation consumers and affected fixture/tests. Existing stored identities and archived data remain unchanged; no wire or native-host API change. This establishes the identity rule needed by the proposed app-transfer ZIP without implementing that separate capability.
