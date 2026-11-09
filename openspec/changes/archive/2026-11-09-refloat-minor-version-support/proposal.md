# Proposal

## Why

The Refloat maintainer guarantees no breaking configuration changes within stable minor releases. Exact patch allowlists unnecessarily reject compatible boards and make support guidance misleading.

## What Changes

- Admit stable 1.0.x, 1.1.x, 1.2.x and 1.3.x with existing version-selected telemetry/control behavior; retain exact beta2/beta3 exceptions and reject unknown minors/prereleases.
- Match validated configuration schemas within supported stable minor families, retaining fingerprint/signature checks and exact observed identity. Preserve exact Tune identity policy.
- Replace patch-release support guidance with minor ranges and explicit prerelease exceptions; align frontend riding eligibility and battery interpretation.
- Non-goals: unknown-minor fallback, cosmetic-schema normalization, cross-patch Tune sharing, storage migration, new wire protocols or hardware claims. The separate relax-refloat-schema-admission proposal remains unimplemented and must account for this stable-minor contract before future implementation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Stable minor admission and accurate support guidance.
- `refloat-configuration`: Schema matching within stable minor families, exact observed configuration identity.

## Impact

Shared Rust compatibility/configuration/public projection, frontend eligibility and guidance, existing simulator and focused checks. Removes patch-based eligibility decisions without removing source evidence fixtures. No contract shape, host ABI, persistence or archive format changes; both hosts reuse the shared decision.
