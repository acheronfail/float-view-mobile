# Proposal

## Why

Refloat can change configuration labels or help text without changing the serialized settings. Float View currently requires an exact full-schema fingerprint for every release, so harmless metadata edits require a new runtime catalogue entry. Explore a narrower, safe admission rule for future verified Refloat releases after the focused 1.3.1-beta2 support change.

## What Changes

- Investigate which validated schema properties must gate configuration reads and writes, and which descriptive properties should remain identity and cache metadata only.
- Define how verified release admission, bounded schema parsing, signed values, cache validation, and mandatory write readback work together without silently admitting unknown firmware behavior.
- Preserve the special 1.0.0/1.0.1 identity distinction and supported configuration, draft, Tune, session, and archive data.
- Identify exact-fingerprint catalogue checks or release-specific fixtures that could be removed from runtime policy while retaining useful source-derived test evidence.
- **Non-goals:** implementing this policy as part of 1.3.1-beta2 support; automatic support for unreviewed releases; cross-version Tune application or migration; relaxing write safety or data compatibility.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Reconsider full-schema fingerprint admission for verified releases while preserving bounded validation, exact active schema identity, and safe writes.

## Impact

Likely affects the shared Rust configuration codec and service, the fixture generator and conformance coverage, and the `refloat-configuration` specification. A final design must determine whether the public contract or persisted identities need changes; neither is authorized by this placeholder. No dependency is planned.

This change is intentionally unfinished. Its design, delta spec, and tasks will be drafted after the beta2 work and a separate safety review.
