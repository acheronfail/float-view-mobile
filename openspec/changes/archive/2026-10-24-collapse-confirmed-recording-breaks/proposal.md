# Proposal

## Why

Confirmed recording interruptions leave large empty areas in ride charts. A rest can obscure the recorded riding without adding useful detail.

## What Changes

- Expose bounded, exact recording-break endpoints from existing durable recording-boundary events in session metadata.
- Collapse only confirmed recording breaks to vertical chart separators, retaining original elapsed times for inspection and selections.
- Keep ordinary sampling delays visible and traces disconnected.
- Non-goals: diagnose or fix radio interruptions, collapse idle samples, change summaries, storage schemas or archive formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: present confirmed recording breaks compactly and honestly.
- `backend-contract`: include bounded recording-break endpoints in session metadata.

## Impact

Shared SQLite reader and loopback metadata projection used by both Apple and Android; frontend contract, mock and chart coordinate handling; canonical conformance. No dependencies or persistence changes. Supersede the uncompressed chart coordinate path for confirmed interruptions only.
