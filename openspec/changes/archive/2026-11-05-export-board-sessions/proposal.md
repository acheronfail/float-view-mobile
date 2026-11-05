# Proposal

## Why

Exporting a board's history currently requires opening and exporting each session individually. A board-scoped bulk export makes backups and analysis practical.

## What Changes

- Add Export all choices for Float View archives and CSV ZIPs to the board-history header menu.
- Produce one ZIP containing the existing individual exports for all non-active sessions belonging to that board, including sessions beyond the loaded page.
- Extend the authenticated export request with an exclusive saved-board selector, reusing native save/share and browser download delivery.
- Preserve existing individual export formats, GPS policy and import behavior. No bulk import, recording changes, storage migration or new host file interface.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `saved-board-session-history`: Board menu bulk export and its scope.
- `backend-contract`: Board-scoped export selector and bounded bundled delivery.

## Impact

Shared frontend/contract/mock, Rust product export worker, shared service and loopback export routes, canonical conformance and existing Apple/Android file presentation consumers. No new dependency; reuse ZIP encoding. No superseded paths to retain or remove.
