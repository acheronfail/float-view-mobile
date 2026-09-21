## 1. Pin and derive Refloat 1.3.0 evidence

- [x] 1.1 Retain `vendor/refloat` at exact stable tag `v1.3.0` commit `47a2c5ce69f0bba4ef634990a8e53a1c07e07e93` and record source provenance for INFO, commands 10/33, state flags, controls, and configuration serialization.
- [x] 1.2 Add 1.3.0 to the configuration-fixture generator allowlist, regenerate immutable schema and value fixtures with VESC Tool, and verify the pinned fingerprint, `0x191a6c1b` signature, field counts, payload sizes, defaults, representative write, and readback normalization.
- [x] 1.3 Add source-derived 1.3.0 INFO, command-33 fast/slow/GNSS, command-10 fault/normal, ride-state, and product-control fixtures while preserving every existing fixture unchanged.

## 2. Add exact compatibility and safe codecs

- [x] 2.1 Extend `refloat-compatibility` with exact stable 1.3.0 admission, a closed selectable-command-33 profile, unchanged older mappings, canonical-record coverage, and adjacent/prerelease rejection tests.
- [x] 2.2 Extend `refloat-dynamic-telemetry` with canonical float32 command-33 fast, slow, and capability-gated GNSS request encoders using only documented non-reserved masks.
- [x] 2.3 Implement strict command-33 response decoding for echoed flags/masks, exact mask-derived extent and source order/types, state flags, all supported fields, finite/range validation, canonical typed mappings, and bounded `refloat.*` extensions.
- [x] 2.4 Add exhaustive safe-codec tests for individual and combined masks, float32 values, state boundaries, GNSS capability/ranges, truncation at every boundary, mismatches, unsupported bits/flags, trailing bytes, non-finite values, and record round trips.
- [x] 2.5 Make compact command-10 interpretation release-aware so 1.3.0 normal responses cannot publish booster torque as amperes while the special active firmware-fault response remains available.

## 3. Compose the 1.3.0 production lifecycle

- [x] 3.1 Update discovery and connection coordination to skip command 32 for exact 1.3.0, publish the selectable profile, start release-selected polling only after route validation, and preserve all older negotiation behavior.
- [x] 3.2 Extend `refloat-telemetry-service` with adaptive command-33 fast polling, once-per-second slow polling, capability-gated GNSS masks, a bounded command-10 mode-0 fault probe, one serialized scheduler lane, and no immediate retry.
- [x] 3.3 Implement generation-scoped 1.3.0 fast/slow cache replacement and merge, `slowObservedAt`, stale response rejection, reset/reconnect behavior, and fault-only command-10 updates without partial sample publication.
- [x] 3.4 Flow every mapped command-33 value and canonical extension through the coordinator, unified runtime, host projection, live snapshots, sessions, queries, and exports while keeping board GNSS out of OS location state and keeping unavailable internal-only fields null.
- [x] 3.5 Add the exact 1.3.0 configuration catalogue entry and exercise schema cache, current/default reads, safe write/readback, Tune save/list/apply isolation, reconnect, flywheel invalidation, and malformed identity behavior.
- [x] 3.6 Extend the stable control profile through 1.3.0 and verify command 12 package state, command 22 flywheel, command 20 lighting, command 36 fatal clear, mutation safety, and confirmation from command-33 state plus the retained active-fault probe.

## 4. Extend simulation and cross-platform conformance

- [x] 4.1 Teach the ESP32/conformance fixture and fixture-development model to advertise exact 1.3.0, parse canonical command-33 requests, emit complete fast/slow/GNSS responses, model limited command-10 faults and 1.3.0 configuration, and reject command 35 and every other non-allowlisted command.
- [x] 4.2 Add canonical direct/CAN, saved/guest, riding/idle/charging/flywheel, configuration/control, reconnect, stale-generation, malformed-response, persistence, retrieval, and export conformance cases for 1.3.0.
- [x] 4.3 Update frontend mock/test scenarios and stable telemetry catalogues for 1.3.0 typed values, nullable unavailable internal fields, and bounded command-33 extensions without adding an alert-history or board-location contract.
- [x] 4.4 Add regression coverage proving all audited 1.0.0–1.2.3 request, decoding, configuration, control, persistence, export, and failure paths remain unchanged.

## 5. Verify and checkpoint

- [x] 5.1 Run focused Rust codec/service/coordinator/runtime tests, fixture and frontend tests, configuration regeneration verification, formatting, warnings-denied linting, and dependency/source-layout/symbol audits; resolve every failure.
- [x] 5.2 Run `just verify` and strict OpenSpec validation with no unavailable software gate.
- [x] 5.3 Run the canonical flashed-fixture checkpoint on Android, iOS, and macOS with actual Refloat 1.3.0, covering direct/CAN telemetry, controls, configuration, reconnect, persistence/export, background behavior, and cleanup; record any genuine device-availability deferral without weakening software evidence.

Physical checkpoint evidence: `conformance/physical-checkpoints/2026-09-21-refloat-1-3-0.md`. The canonical ESP32 simulator advertised exact stable 1.3.0 over real BLE; all iOS/Android/macOS scenarios passed. Background location authorization and lifecycle passed, with new GPS fix acquisition explicitly unverified.
