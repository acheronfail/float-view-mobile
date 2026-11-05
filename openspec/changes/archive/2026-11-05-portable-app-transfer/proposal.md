# Proposal

## Why

Users need one portable backup of saved rides, current boards/profiles, tunes and preferences, and repeatable imports across Apple and Android. Session imports currently generate new IDs and therefore duplicate recordings on re-import.

## What Changes

- Add Settings actions to export a versioned app-transfer ZIP, merge an exported ZIP, and explicitly clear saved app data.
- Preserve recording IDs across session and app import/re-export; skip existing recordings and preserve existing board profiles/tunes.
- Include current board/profile records (even without rides), saved tune identities/values, settings and all completed recordings. Remap auto-connect by portable board identity.
- Validate the complete transfer before one catalog merge; use issued files and existing orphan recovery for interruption safety.
- Offer an explicit preferences-import choice; never apply controller configurations during import.
- Keep exports, OS permissions and device-local associations outside portable data. No replace-from-backup mode, cloud sync, history merging, migration framework or controller aliases.

## Capabilities

### New Capabilities

- `app-data-transfer`: Portable app export, repeatable merge import, and user-requested data clearing.

### Modified Capabilities

- `saved-board-session-history`: Stable recording IDs and repeatable archive import.
- `backend-contract`: App transfer/control endpoints and summaries.

## Impact

Shared Rust product-store/file worker/service and loopback routes; frontend contract/mock/settings and native delivery consumers; canonical conformance. Reuse ZIP, scoped native file presentation, bounded transfers, profile/settings validation and orphan recovery. No new dependency or storage-schema change. Existing supported baseline archives remain readable; exported individual archive version stays 4.
