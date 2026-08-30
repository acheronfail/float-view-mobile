## 1. Candidate boundary and locked evidence

- [x] 1.1 Add `session-sqlite-writer` to the locked Rust workspace with direct `refloat-telemetry-service`, maintained bundled SQLite, and deterministic JSON dependencies only.
- [x] 1.2 Add literal sample/location input and expected database-state fixtures covering canonical creation, nulls, extensions, transitions, duplicate replay, extrema ties, multiple batches, and reopen.
- [x] 1.3 Extend root verification to require the crate/fixtures, exact schema inclusion and dependency allowlist, and reject host selection, bridge crates, copied native sources, catalog/server/runtime ownership, generated bindings, and production native/frontend/SQL changes.

## 2. Canonical session file lifecycle

- [x] 2.1 Implement stable payload-free errors, bounded public metadata/location/marker values, redacted debug output, checked SQLite integer conversion, and candidate-owned temporary path cleanup.
- [x] 2.2 Implement new-file creation from byte-exact `backends/sql/session.sql`, metadata/marker initialization, checkpoint/close, and atomic target publication without overwrite.
- [x] 2.3 Implement compatible-file open and validation for integrity, required canonical tables/indexes/singletons, positive cadence metadata, and incompatible/corrupt rejection without mutation.
- [x] 2.4 Add independent tests for exact schema/pragmas/initial rows, existing-target preservation, failed-create cleanup, compatible reopen, and malformed/corrupt database rejection.

## 3. Atomic telemetry writer

- [x] 3.1 Validate 1–1,024 strictly ordered samples, durable integer extents, finite optional numerics, closed wire mappings, and 32-name/64-byte/16,384-byte deterministic extension JSON bounds before mutation.
- [x] 3.2 Implement one immediate transaction for canonical sample insertion/replay, first stable/extension numeric-field sources, changed transition events, and durable event sequencing.
- [x] 3.3 Stream ordered durable rows in Rust to recompute and atomically publish exact marker sequences/counts/time, riding elapsed, trapezoidal distance, absolute extrema with earliest source ties, and last speed/state.
- [x] 3.4 Add fixture and adversarial tests for null/extension values, transitions, duplicates, tie sources, missing prior speed, multi-batch/reopen equivalence, every input bound, and deterministic redacted failures.

## 4. Atomic location writer and transaction recovery

- [x] 4.1 Validate and deterministically order 1–1,024 location observations with durable time/elapsed, coordinate, accuracy, and numeric bounds.
- [x] 4.2 Implement one immediate transaction for elapsed/coordinate deduplication, consecutive location sequences, and exact committed/recorded/stored/status marker updates.
- [x] 4.3 Add fixture/adversarial tests for unordered input, boundary coordinates, nullable accuracy, duplicate-only batches, multi-batch/reopen continuation, and invalid location rollback.
- [x] 4.4 Inject mid-sample and mid-location SQLite failures and prove each entire batch rolls back while prior durable rows, fields, events, counts, and marker state remain unchanged.

## 5. Focused and repository verification

- [x] 5.1 Run locked offline writer unit/integration tests, schema/dependency/ownership audits, formatting, and warnings-denied workspace linting.
- [x] 5.2 Run strict change validation, `git diff --check`, and the complete `just verify` software gate.

## 6. Physical regression and completion

- [x] 6.1 Run the canonical ESP32 fixture matrix on the macOS host and record every non-location checkpoint.
- [x] 6.2 Run the canonical ESP32 fixture matrix on the connected Android device, including background BLE/session persistence; defer only bounded location acquisition.
- [x] 6.3 Run the canonical ESP32 fixture matrix on the connected iOS device, including background BLE/session persistence; defer only bounded location acquisition.
- [x] 6.4 Confirm no production Apple/Android/frontend/SQL source or package input changed, record exact verification evidence, and mark the change ready for implementation commit, canonical spec sync, and archive.

## Verification Evidence

- 2026-08-30: Focused writer tests, warnings-denied workspace linting, ownership/dependency/schema audits, strict OpenSpec validation, `git diff --check`, and every constituent of `just verify` passed. The complete root gate needed the repository Android AVD to be started before its Android conformance constituent could pass; the final audit then passed.
- 2026-08-30: macOS passed all 16 applicable ESP32 scenarios against `/dev/cu.usbmodem111401`. A transient pre-assertion BLE disconnect interrupted the first continuous run during `direct`; the canonical runner resumed at `direct`, and every remaining identity, protocol, routing, control, BMS, ride, reconnect, session, and cleanup assertion passed.
- 2026-08-30: Android device `1cfb7634120d7ece` passed all 17 ESP32 scenarios across clean resumable runs after two fixture `ADVERTISE_FAILED` setup errors. Background BLE telemetry, session persistence, lifecycle recovery, and cleanup passed. Timed location acquisition alone was explicitly deferred and remained disabled for that checkpoint.
- 2026-08-30: iOS device `00008101-00195021268B001E` passed all 17 canonical ESP32 scenarios in one signed-runner invocation. The background lifecycle runner moved the application to the background and foreground successfully while BLE telemetry continued through response 90, session state remained available, and cleanup passed. Timed location acquisition alone was explicitly deferred and reported `status=disabled`, `background=unavailable`, and unchanged location rows/sequences.
- 2026-08-30: `git diff --name-only fae172a -- backends/apple backends/android frontend backends/sql` and the focused host package-input diff were empty, while `git diff --check` passed. The change is production-isolated and ready for implementation commit, canonical spec sync, and archive.
