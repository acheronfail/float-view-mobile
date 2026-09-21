## Why

Refloat 1.3.0 is a stable release, but Float View currently rejects its exact INFO identity and expects the older internal commands 31/32 telemetry vocabulary. Supporting the release now lets users retain connection, telemetry, controls, configuration, sessions, and exports while moving the new release to Refloat's documented selectable command 33 instead of extending dependence on its explicitly internal telemetry interface.

## What Changes

- Add exact, fail-closed compatibility admission for stable Refloat 1.3.0 while preserving every audited 1.0.0–1.2.3 profile and rejecting prereleases, ranges, and unknown adjacent releases.
- Select documented command 33 for Refloat 1.3.0, with bounded float32 requests, echoed-mask response validation, and separate fast and slow masks covering every documented non-reserved field; retain commands 31/32 unchanged for audited 1.1.0–1.2.3 releases and the legacy path for 1.0.x.
- Normalize command-33 values into existing typed telemetry where semantics match and retain additional stable numeric values, including loop frequency and capability-gated board GNSS, as bounded extensions distinct from OS-owned location observations.
- Keep the current limited alert behavior: derive alert reason and fatal latch from command-33 state flags, retain the existing bounded low-cadence firmware-fault observation, and continue fatal clearing through command 36 without adopting unstable command 35, alert history, fault names, or a new alert contract.
- Make compact telemetry interpretation release-aware so Refloat 1.3.0's normalized booster-torque byte is never published as booster current in amperes.
- Add source-derived Refloat 1.3.0 custom-configuration schema/value fixtures and its exact fingerprint and wire signature; preserve exact-version Tune isolation and the existing verified read/write/readback lifecycle.
- Re-audit and exercise package, flywheel, stable-lighting, fatal-clear, direct/CAN routing, session, persistence, export, reconnect, and malformed-input behavior against the pinned 1.3.0 source.
- Update the vendored Refloat submodule from 1.3.0-beta3 to the exact stable `v1.3.0` commit and retain it as the offline source of truth.
- Non-goals: command 35, alert history or names, Refloat data recording/history, runtime tuning, Handtest, remote/motor control, automatic Tune migration, or replacing OS-owned location with board GNSS.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Admit exact Refloat 1.3.0 and select its command-33 telemetry profile without broadening older compatibility mappings.
- `refloat-telemetry-pipeline`: Define complete bounded command-33 requests, decoding, normalization, cadence, merge, extension, alert-boundary, persistence, and export behavior for 1.3.0.
- `refloat-configuration`: Add the exact audited 1.3.0 schema, value codec identity, read/write verification, cache identity, and Tune isolation behavior.
- `refloat-live-controls`: Extend the audited stable control catalogue through 1.3.0 while retaining the existing command allowlist and limited fatal-latch behavior.
- `backend-conformance`: Require source-derived 1.3.0 vectors and cross-host software and physical evidence without regressing older releases.

## Impact

The shared Rust compatibility, telemetry, configuration, mutation, discovery, coordinator, runtime, persistence, and host projection components will gain exact 1.3.0 behavior. The conformance fixture, immutable Refloat fixtures, frontend mock/test data, configuration generator allowlist, root audits, and canonical physical checkpoint will be extended. The wire contract need not add an alert model or board-location object; new command-33-only numeric data can use the existing bounded telemetry extension mechanism. No new third-party dependency or generated API layer is introduced, and no older decoder or compatibility entry is removed.
