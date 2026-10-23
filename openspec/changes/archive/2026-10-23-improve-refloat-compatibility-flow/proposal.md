# Proposal

## Why

A known unsupported Refloat package currently looks like a transient connection problem: a generic dialog offers Retry while the dashboard can say Reconnecting. Users need the detected version, compatibility guidance, and an explicit way back to boards.

## What Changes

- Preserve bounded public Refloat INFO version metadata even when admission fails, including its source-defined suffix.
- Show a specific unsupported-version dialog with Check again, primary Back to boards, and offline View supported versions guidance from the audited catalog.
- Distinguish absent or unreadable INFO from a confirmed unsupported release; keep retry available for temporary failures.
- Stop recovery presentation and automatic reconnection after confirmed compatibility failure; suppress the generic diagnostics prompt for that expected outcome.
- Remove the generic Retry/Cancel presentation for confirmed unsupported Refloat versions.
- Non-goals: adding firmware support, updating/flashing packages, enabling partial telemetry, changing storage, or changing native Bluetooth adapters.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: bounded compatibility failure metadata and recovery presentation.

## Impact

The frontend contract, client/mock fixtures, shared compatibility/coordinator/service/projection, and canonical conformance tests change together. Apple and Android consume the shared projection without new host APIs. No dependencies or persistence changes.
