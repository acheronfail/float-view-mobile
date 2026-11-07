# Proposal

## Why

Three retained incidents can replace a useful error before a user exports it. The user also authorizes configuration contents, telemetry values, schema identities and packet data in manually shared diagnostics, while keeping location, device identities, credentials and arbitrary error text excluded.

## What Changes

- Retain up to ten incidents within the existing seven-day, total-byte and per-incident bounds.
- Add bounded configuration/value and numeric telemetry captures, actual schema fingerprints/signatures, and classified protocol packet bytes.
- Preserve location and identity exclusions: strip routing wrappers, omit identity/unknown packets and location-bearing raw responses, and keep decoded telemetry free of location and arbitrary extension text.
- Add an explicit restartable diagnostic storage v1-to-v2 upgrade preserving old incidents and journals, plus report format v2. Keep product storage baseline and archive formats unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `support-diagnostics`: Larger incident count and richer bounded local diagnostic contents with remaining privacy exclusions.

## Impact

Shared Rust protocol coordinator, service diagnostics, independent diagnostic storage/export, user-facing privacy copy and focused tests. No public request/result or native ABI change, new dependencies or automatic upload. Remove the three-slot assumptions and blanket exclusions for the newly authorized data. Non-goals: CAN topology collection, GPS/location capture, arbitrary packet dumps, raw errors, changing polling/capture windows or general storage migrations.
