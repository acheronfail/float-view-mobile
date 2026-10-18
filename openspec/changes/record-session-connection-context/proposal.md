# Proposal

## Why

Saved rides currently lose the firmware versions and connection path that produced their telemetry. Persisting this context, including each successful reconnect, makes session review and shared archives useful after the board's firmware or topology changes.

## What Changes

- Persist typed `connectionContext` session events with Refloat version, selected controller firmware, Bluetooth peer firmware, and direct/CAN routing.
- Capture initial activation and every successful reconnect/resume, preserving earlier context even when values are unchanged.
- Show firmware, connection path and chronological reconnect history in session details, including active-session refresh and explicit missing context for older recordings.
- Add bounded, authenticated context-history reads and a metadata count using the shared contract on Apple, Android and the mock.
- Preserve context through restart/recovery, pruning, and native archive export/import; validate imported payloads and retain supported baseline recordings and archives.
- Non-goals: full CAN inventories, raw device identifiers, firmware/configuration snapshots, changes to reconnect policy or wire polling, guest recording, diagnostic collection changes, and CSV metadata export. CSV remains telemetry-only.

## Capabilities

### New Capabilities

- `session-connection-context`: durable firmware/topology provenance, activation history, review and retention.

### Modified Capabilities

- `backend-contract`: mandatory bounded session context-history reads and metadata counts.
- `saved-board-session-history`: archive ownership requirements explicitly permit public firmware/topology context while continuing to exclude private device identifiers.

## Impact

Shared Rust backend-service/application-runtime, session-sqlite readers/writer/pruning/recovery, product-store archive validation, loopback-server, frontend contract/client/mock and session details, and canonical conformance coverage. Both native integrations consume the same contract through existing bindings; no host-owned product logic or new dependency is needed. Reuse `session_event` and remove no currently supported paths; replace any session-details assumption that firmware context comes from the live connection. No planned schema or archive-number change: compatible event additions must be verified against supported fixtures and validators; an actually incompatible layout change would require an explicit supported-data upgrade with independent archive version handling.
