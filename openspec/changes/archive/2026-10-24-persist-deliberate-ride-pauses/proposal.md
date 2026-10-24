# Proposal

## Why

The archived collapse-confirmed-recording-breaks change treats accidental reconnects and deliberate ride pauses alike. Persist the distinction at interruption time so charts mark only intentional pauses, including after restart and export/import.

## What Changes

- Attempt to persist a bounded interruption cause before detaching a recording and copy a successfully saved cause into the resumed boundary. Storage failure must never block disconnect.
- Extend recording-break metadata with `pause`, `connectionLoss` or `unknown` provenance.
- Collapse and mark only explicit user pauses. Preserve ordinary missing-time width for connection loss and unknown history.
- Keep supported storage and archives readable; absent reasons remain unknown without inference or rewriting.
- Always issue Bluetooth disconnect, release recording ownership and publish disconnected state even when saving the reason, detaching/checkpointing, draft cleanup or catalog work fails; report cleanup errors afterwards.
- Non-goals: infer intent from duration, fix radio reliability, change ride summaries, add a migration framework or native host policy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-continuity`: durable interruption causes and deliberate-pause-only chart markers.
- `backend-contract`: bounded recording-break reason metadata shared across hosts.

## Impact

Shared SQLite writer/reader, runtime/service interruption handling, loopback projection, frontend contract/client/mock, chart mapping and canonical conformance. Replace unconditional interruption compression. Additive event details use existing tables and archive format; no dependencies, schema change, storage reset or archive upgrade.
