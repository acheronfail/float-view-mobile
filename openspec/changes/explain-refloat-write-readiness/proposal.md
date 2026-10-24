# Proposal

## Why

Refloat write warnings are hidden above scrolling fields and disappear from view when users dismiss a failed-write confirmation. The generic writable flag cannot explain which authoritative condition blocks an edited configuration write.

## What Changes

- Add a bounded nullable write-block reason to complete configuration summaries, produced by the shared backend on both native hosts and mirrored by the mock.
- Keep actionable readiness guidance beside Write and inside its confirmation, including when readiness changes while the confirmation is open.
- Retain failed-write feedback in the footer after dismissal without clearing edits, moving scroll, or replaying a write.
- Remove the generic scrolling warning. No new dependencies, persistence changes, write permissions, automatic retries, or Tune admission changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Authoritative edited-write block reasons and visible, retained warning feedback.

## Impact

Frontend contract/client validation, mock, shared mutation/application policy and public projection, canonical conformance, and editor presentation/tests. Apple and Android consume the same shared projection through existing bindings; no host-specific policy or storage changes are needed.
