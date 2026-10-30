# Proposal

## Why

Float View rejects Refloat 1.3.1-beta2, although its primary telemetry and controls use the supported 1.3 wire family. The beta also reports an IMU timeout as a distinct package error; the current firmware-fault-only command-35 decoder cannot represent it correctly.

## What Changes

- Support the audited `1.3.1-beta2` release for ride logging, live telemetry, Refloat configuration, and same-version Tune application by reusing existing 1.3 protocol and control behavior where the upstream source confirms it.
- Decode IMU timeout as a distinct alert, show its active state and retained stop reason, and persist bounded activation and clearing records in the existing session event, history, and export paths. Present them in the existing fault-history experience under inclusive “Faults and errors” wording.
- Add the exact beta2 configuration schema fixture and catalogue identity; retain the current full-fingerprint admission policy for this change.
- Keep release admission and alert capability decisions centralized. When stable 1.3.1 is audited, replace beta-specific runtime admission and fixtures with stable support rather than accumulating beta profiles. Preserve supported stored sessions, events, and Tunes; beta Tunes need not apply to stable firmware.
- Reconcile current specification text that still describes command 35 as excluded despite its existing firmware-fault-history use.
- **Non-goals:** cross-version Tune application or conversion; Refloat’s runtime tuning, remote, data-recording, or motor-control features; automatic admission of later betas or stable 1.3.1; a general schema-admission redesign; a parallel history subsystem or migration framework.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Admit the exact audited beta2 identity and reject neighboring unaudited identities.
- `refloat-telemetry-pipeline`: Reuse the 1.3 telemetry profile and decode, retain, and persist IMU timeout alerts and stop reason.
- `refloat-configuration`: Admit beta2’s audited exact schema while retaining safe read, write, and same-version Tune rules.
- `refloat-live-controls`: Reuse 1.3 controls with authoritative timeout-aware mutation safety.
- `backend-contract`: Add typed timeout live state and bounded historical event data to the shared contract.
- `live-board-dashboard`: Present active timeout and its history in the existing warnings and history experience.
- `backend-conformance`: Cover beta2 source fixtures, direct/CAN software behavior, supported-data preservation, and targeted physical interoperability.

## Impact

Shared Rust compatibility, telemetry, configuration, mutation, persistence, projection, and loopback history owners; frontend contract/client/mock and live/history UI; conformance fixtures, tests, and configuration generator; and `vendor/refloat` are affected. The public contract gains additive timeout fields and event kinds, implemented by both hosts through the existing shared runtime. Supported storage and archive data remain readable; no storage-layout, marker, archive-format, or dependency change is planned. Superseded beta-specific admission and fixture paths are removed when stable support replaces them.
