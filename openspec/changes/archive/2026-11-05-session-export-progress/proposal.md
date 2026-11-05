# Proposal

## Why

Board exports can take time on mobile, but currently expose only a generic busy label and cannot be cancelled during preparation.

## What Changes

- Show a blocking, accessible progress modal with completed/total session counts and Cancel.
- Correlate bulk exports with an operation ID; provide authenticated status and cancellation controls backed by the existing worker cancellation flag.
- Keep the modal until preparation/download finishes or cancellation cleanup completes; distinguish cancellation from failure.
- Give bulk ZIPs a separate 2 GiB bound, disk-space preflight and actionable storage/size failures; preserve individual 128 MiB session compatibility.
- Preserve archive formats, recording, storage and native save/share interfaces. No general job framework or persisted jobs.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `saved-board-session-history`: Export progress modal and cancellation.
- `backend-contract`: Correlated board export progress/status/cancellation.

## Impact

Frontend contract/state/modal/mock, shared Rust export worker and service, loopback HTTP/control export routes, canonical browser/direct/native conformance. Reuse existing cancellation and dialog controls. Add rustix safe filesystem-space queries while preserving the unsafe-code prohibition; no storage migration. Remove the board's generic export busy label.
