# Proposal

## Why

Refloat 1.3.1-beta3 fixes filter calibration after IMU sample-rate changes, but Float View rejects its exact release identity. Its audited source retains beta2's wire formats and configuration schema, so support can reuse the existing profiles.

## What Changes

- Admit exact beta3 alongside beta2 using selectable telemetry, stable controls and IMU-timeout safety/history handling.
- Pin beta3 source and map its verified identical configuration bytes to the existing fixture files. Reuse beta2 protocol vectors and existing codec capabilities; retain exact-version configuration and Tunes with verified readback.
- Exercise direct/CAN software scenarios and a stationary ExpressT production macOS BLE checkpoint, backing up and restoring configuration.
- Preserve all supported data and beta2 behavior. No API, persistence, dependency or host-binding change is required, and no existing supported path is superseded. Remove duplicate beta3 fixture files and redundant version lists.
- Non-goals: schema-admission relaxation, automatic future-release support, cross-version Tunes, motor operation, IMU failure injection or validation of the damaged controller's duty/eRPM values.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-telemetry-pipeline`: Extend audited admission to beta3 with existing beta2 telemetry, controls and timeout policy.
- `refloat-configuration`: Admit the exact beta3 schema and preserve exact-version Tune isolation.
- `backend-conformance`: Add beta3 software and scoped physical interoperability evidence; stop treating beta3 as an unaudited neighbor.

## Impact

Shared Rust compatibility/configuration catalogues, frontend Tune eligibility, conformance fixture profiles/scenarios, source-derived fixtures and `vendor/refloat`. Apple and Android consume the existing shared runtime and bindings. Update the separate unfinished `relax-refloat-schema-admission` proposal with the release/profile/fixture distinction and stable-release cleanup criteria; do not implement its admission-policy relaxation.
