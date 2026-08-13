## 1. Candidate Boundary

- [x] 1.1 Add a locked safe `backends/core/refloat-configuration-service` workspace crate that depends only on `refloat-configuration-codec` and exposes bounded platform-neutral facts, events, effects, snapshots, identities, and stable payload-free errors.
- [x] 1.2 Implement the audited release catalogue, legacy patch resolution, connection begin/reset, cache admission decisions, and bounded schema-page assembly through the existing codec.
- [x] 1.3 Implement current/default reads, monotonic revisions, flywheel policy, operation and generation matching, write encoding, acknowledgement-to-readback sequencing, normalization, conflicts, aborts, and failure recovery.

## 2. Candidate Evidence

- [x] 2.1 Add source-derived fixture tests for every audited release, legacy resolution, cache hit/miss/corruption, exact schema paging, and direct/CAN opaque route identity.
- [x] 2.2 Replay the complete selected Swift and Kotlin service transition vectors, including reads, defaults, write verification, flywheel transitions, reconnect/reset, revisions, snapshots, effects, and native-equivalent failures.
- [x] 2.3 Add adversarial tests for stale or malformed generations, operations, routes, indexes, commands, chunks, offsets, lengths, acknowledgements, failures, aborts, ordering, counter wrap, bounds, determinism, and payload-safe diagnostics.

## 3. Ownership And Isolation

- [x] 3.1 Document the new candidate boundary and later deletion path in core ownership guidance without changing the frontend contract, persistent formats, or production owners.
- [x] 3.2 Extend workspace, dependency, source-selection, bridge-catalogue, symbol, generated-artifact, package-input, and safety audits to prove the candidate is locked, safe, codec-only, and absent from every production package and call path.

## 4. Software Verification

- [x] 4.1 Run focused candidate tests, locked formatting, warnings-denied linting, dependency/ownership audits, strict change validation, and repository diff checks.
- [x] 4.2 Run `just verify` and all packaging checks with the isolated candidate present, resolving every non-physical regression.

## 5. Physical Regression

- [x] 5.1 Run the complete applicable Android ESP32 matrix on the connected device and verify every non-location assertion.
- [x] 5.2 Run the complete applicable iOS ESP32 matrix on the connected device and verify every non-location assertion, deferring only bounded location-sample acquisition when necessary.
- [x] 5.3 Run the complete applicable macOS ESP32 matrix on the host and verify every assertion.
- [x] 5.4 Record the physical evidence, rerun final focused and complete software checks, and leave the candidate unselected for separate host-cutover changes.

## Verification evidence

- 2026-08-14: Android physical conformance passed all 17 applicable scenarios against `/dev/cu.usbmodem111401` on connected device `1cfb7634120d7ece` through the unchanged selected Kotlin configuration service. Background BLE/session assertions passed; location was authorized and available but produced no sample during the bounded 10-second dwell, so only location persistence remains deferred to manual coverage. One fixture `ADVERTISE_FAILED` occurred before any app assertion and the affected scenario passed on a clean retry.
- 2026-08-14: macOS physical conformance passed all 16 applicable scenarios against the same fixture through the unchanged selected Swift configuration service. CAN-preference scenarios were paired with identity in fresh-storage runs, and every BLE, protocol, configuration schema/current/default/write/readback, BMS, control, telemetry, session, shutdown, reconnect, and cleanup assertion passed.
- 2026-08-14: iOS physical conformance passed all 17 applicable scenarios against the same fixture on connected device `00008101-00195021268B001E` through the unchanged selected Swift configuration service. Bluetooth authorization was granted before the matrix. Background BLE/session accumulation, foreground recovery, persistence boundaries, and cleanup passed; location authorization remained unresolved during its bounded wait and the location checkpoint stayed at `rows 0->0`, `sequence 0->0`, so only location coverage remains deferred to manual verification as requested.
- 2026-08-14: Focused service tests, warnings-denied linting, structural audits, strict change validation, and the complete `just verify` software gate passed in the final implementation state. The candidate remains absent from production packages and call paths for separate host-cutover changes.
