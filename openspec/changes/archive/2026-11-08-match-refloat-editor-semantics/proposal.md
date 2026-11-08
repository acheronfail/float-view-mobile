# Proposal

## Why

Refloat fields currently show raw fractions with percentage suffixes and ignore VESC Tool editor precision. Users expect the same displayed units and edited values in both applications.

## What Changes

- Preserve upstream editor scale and percentage-editor metadata through the shared schema contract.
- Apply editor units, precision and steps to numeric input and confirmation displays.
- Keep canonical values in requests, Tunes and persisted drafts; preserve existing identities and untouched values.
- Replace raw numeric presentation. No new dependencies or parallel editor path.
- Non-goals: wire overflow wrapping, write eligibility changes, firmware admission or release publication.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Numeric editor units, precision and review consistency.

## Impact

Shared Rust schema normalization and transient schema records, public schema fields, shared Svelte editor, mock/direct conformance and host boundary verification. Compressed cached schemas, fingerprints, wire signatures, Tune values and durable draft format remain unchanged.
