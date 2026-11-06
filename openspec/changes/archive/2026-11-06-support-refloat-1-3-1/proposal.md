# Proposal

## Why

Refloat 1.3.1 is released, but exact release admission currently rejects it. Its application source is identical to the audited beta3 source.

## What Changes

- Pin the submodule to stable 1.3.1 and admit its exact identity with existing telemetry, controls, IMU-timeout history, configuration and Tune policy.
- Reuse source-verified fixture bytes while preserving stable provenance and shared `1.3.1` Tune compatibility for audited beta2, beta3 and stable.
- Retain beta2/beta3 support, as required by the current compatibility plan; remove stable 1.3.1 from unsupported-neighbor expectations.
- Non-goals: implement `relax-refloat-schema-admission`, broaden unknown-version admission, retire betas, change storage formats, wire APIs or host bindings, or flash hardware.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-telemetry-pipeline`: admit exact stable 1.3.1 with existing beta3 behavior.
- `refloat-configuration`: admit stable schema and share audited beta/stable Tunes.
- `backend-conformance`: verify stable admission and retained prereleases.

## Impact

Shared compatibility/configuration catalogues, frontend Tune policy, simulator and source fixture manifest. No duplicate fixture bytes, protocol implementation or dependencies are needed. The broader active change remains planning-only; this scoped release admission does not require its fallback or contract changes.
