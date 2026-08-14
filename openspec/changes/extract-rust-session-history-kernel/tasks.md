## 1. Candidate boundary and shared evidence

- [x] 1.1 Add `session-history-kernel` to the locked Rust workspace with a standard-library-only manifest and document its unselected future-persistence ownership.
- [x] 1.2 Add literal shared series and prune fixtures covering canonical edge/tie/gap/phase/protection/eight-cap behavior without importing frontend or native implementations.
- [x] 1.3 Extend the root verification audit to require the candidate/fixtures and reject host selection, bridge crates, copied runtime sources, SQLite/network/runtime dependencies, and native-file changes.

## 2. Historical series reduction

- [x] 2.1 Implement bounded field/range/bucket/gap/cadence configuration and stable payload-free errors with checked allocation/arithmetic.
- [x] 2.2 Implement ordered incremental first/minimum/maximum/last reduction, chronological tie resolution, role deduplication, null/gap generations, and finite 65,536-observation output.
- [x] 2.3 Add shared-vector tests for explicit/automatic gaps, phase cadence, retained resolution, late fields, ties, dense buckets, and deterministic final column order.
- [x] 2.4 Add adversarial tests for every configuration/input bound, descending/duplicate keys, arity, non-finite values, overflow, generation exhaustion, and partial-publication rejection.

## 3. Prune selection

- [x] 3.1 Implement ordered elapsed-zero-aligned bucket selection with protected endpoints, first field sources, riding transitions, and caller-protected semantic sources.
- [x] 3.2 Implement deterministic ordinary first/last/extrema maximum coverage, chronological ties, uncovered-role reporting, and the eight-row cap.
- [x] 3.3 Return finalized bucket selections incrementally and release raw bucket state before accepting later unbounded history.
- [x] 3.4 Add shared-vector and adversarial tests for protection, ties, multi-role/dense coverage, bucket transitions, repeated aligned coarser selection, invalid ordering/resolution, and bounded memory state.

## 4. Focused and repository verification

- [x] 4.1 Run candidate unit/integration tests, the ownership/dependency audit, `cargo fmt --check`, and warnings-denied offline locked workspace linting.
- [x] 4.2 Run strict change validation, `git diff --check`, and the complete `just verify` software gate.

## 5. Physical regression and completion

- [x] 5.1 Run the canonical ESP32 fixture matrix on the macOS host and record every non-location checkpoint.
- [x] 5.2 Run the canonical ESP32 fixture matrix on the connected Android device, including background continuity; defer only bounded location acquisition.
- [x] 5.3 Run the canonical ESP32 fixture matrix on the connected iOS device, including background continuity; defer only bounded location acquisition.
- [x] 5.4 Confirm no production Apple/Android/frontend/SQL source or package input changed, record verification evidence, and mark the change ready for implementation commit, spec sync, and archive.

## Verification evidence

- 2026-08-14: `just core-session-history-kernel-audit` passed the locked offline candidate tests, workspace formatting, warnings-denied linting, dependency/ownership checks, shared vectors, and adversarial bounds. Strict OpenSpec validation and `git diff --check` passed.
- 2026-08-14: the complete escalated `just verify` gate passed with exit code 0, including frontend and conformance checks/tests, all Rust workspace tests, Swift and Kotlin tests, Apple and Android packaging, canonical macOS/iOS-simulator/Android execution, fixture software tests, and repository audits.
- 2026-08-14: macOS physical conformance passed all 16 applicable scenarios against `/dev/cu.usbmodem111401`. The first identity attempt disconnected before authoritative application state; a clean full retry passed identity, direct/legacy, direct/CAN configuration and control, shutdown, routing, BMS, ride, unexpected-disconnect recovery, session, and cleanup assertions.
- 2026-08-14: Android physical conformance passed all 17 scenarios on connected device `1cfb7634120d7ece` across clean harness runs. Scenarios 1–12 passed before the fixture returned `ADVERTISE_FAILED` while preparing scenario 13, before any application assertion; scenarios 13–17 then passed in a fresh process. Background BLE telemetry, session accumulation, lifecycle recovery, persistence boundaries, and cleanup passed. Automated location authorization/fix acquisition alone was deferred with `FLOAT_VIEW_DEFER_PHYSICAL_LOCATION=1`.
- 2026-08-14: iOS physical conformance passed all 16 foreground scenarios on connected device `00008101-00195021268B001E`. In the background checkpoint, `FloatViewLifecycleTests.testBackground` passed in 0.579 seconds with zero failures and production BLE telemetry continued beyond response 300 while backgrounded. The already-documented parent `xcodebuild` post-test hang reached the harness timeout after XCTest success; automated location acquisition alone was deferred.
- 2026-08-14: `git diff --name-only -- backends/apple backends/android frontend backends/sql` was empty after all gates. No production host, frontend, SQL schema, bridge crate, or package input changed; the candidate remains unselected.
