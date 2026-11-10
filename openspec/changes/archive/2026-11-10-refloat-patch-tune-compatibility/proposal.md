# Proposal

## Why

Tunes saved on Refloat 1.3.0 are unavailable on 1.3.1 despite the stable-minor configuration compatibility guarantee already used for board support. Exact Tune version and full-fingerprint equality unnecessarily separate audited compatible patch schemas.

## What Changes

- Share Tunes across stable patches in supported 1.0.x–1.3.x minors, retaining the audited beta2/beta3 equivalence with stable 1.3.x.
- Treat audited schema fingerprints within a minor as equivalent only with matching wire signatures. Validate complete portable values against the target schema and retain all mutation/readback gates.
- Align filtered pagination, apply, overwrite and frontend availability. Preserve stored versions, fingerprints and all existing records; retain existing beta public-version normalization.
- Remove exact-patch Tune isolation checks. Non-goals: cross-minor sharing, unknown-schema admission, new prerelease support, migration, API/ABI changes or the separate unknown-firmware fallback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Stable-minor Tune compatibility with audited schemas, preserved identities and target validation.

## Impact

Shared Rust schema catalogue, product store and apply dispatch; frontend Tune policy and mock; focused native service, persistence, conformance and browser coverage. Both native hosts use the shared service without boundary changes. No storage or archive format changes. The separate relax-refloat-schema-admission draft must retain this new baseline.
