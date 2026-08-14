## 1. Candidate Model and Composition Boundary

- [x] 1.1 Add `float-view-vesc-bms-telemetry-service` to the locked workspace with unsafe code forbidden and only auxiliary discovery, BMS values, and request scheduler as production dependencies.
- [x] 1.2 Define validated service configuration, request/timer identities, complete owned/redacted BMS observations, commands, snapshots, failures, diagnostics, and ordered effects.
- [x] 1.3 Implement exhaustive bounded conversion from borrowed command-96 values and exact read-only direct/CAN scan, firmware, and values scheduler facts.

## 2. BMS Application Lifecycle

- [x] 2.1 Compose auxiliary begin, reusable/preferred/enumerated discovery, CAN results, firmware results, probe/scan failure, selected route, and initial values work without host execution.
- [x] 2.2 Implement one current values poll, completion-driven 1,000 ms timer replacement, exact context/route/generation attribution, cancellation, reset, and stale/repeated event rejection.
- [x] 2.3 Implement single-failure accounting for timeout, queue-full, enqueue rejection, and malformed response; capped three-failure unavailability; retained observation; valid-response reset and recovery.
- [x] 2.4 Preserve standalone direct-BMS behavior, controller independence, read-only command safety, deterministic effect order, atomic validation, and explicit wrapping counter semantics.

## 3. Parity, Adversarial, and Isolation Evidence

- [x] 3.1 Add committed shared source-derived lifecycle vectors covering controller-attached and standalone direct/preferred/reused/enumerated discovery, polling, failure, recovery, and reset.
- [x] 3.2 Add focused tests for exact scheduler facts/bytes, every command-96 field, maximum collections/status, all scheduler completions, three-failure boundaries, stale/mismatched orders, invalid routes and observation-time boundaries, timer and generation rollover, privacy, and no mutation commands.
- [x] 3.3 Update workspace locks, core ownership documentation, focused root recipes, and repository audits to require the exact three-dependency unselected candidate and forbid host selection, bridges, callbacks, executors, persistence, sockets, or packaged artifacts.

## 4. Verification

- [x] 4.1 Run focused tests/audits, formatting, warnings-denied locked workspace linting, strict OpenSpec validation, and `git diff --check`.
- [x] 4.2 Run `just verify` with both selected native BMS lifecycle owners unchanged.
- [x] 4.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; require every non-location product checkpoint and defer only bounded location acquisition per the agreed manual follow-up.

## Verification Evidence

- 2026-08-14: Android physical conformance passed all 17 scenarios against `/dev/cu.usbmodem111401` on connected device `1cfb7634120d7ece`. Rapid consecutive fixture resets exposed an ESP32 advertise/termination race, so affected scenarios were rerun in isolated clean harness invocations. Background BLE/session assertions passed; location was authorized and available but produced no sample during the bounded dwell, so location persistence remains deferred.
- 2026-08-14: macOS physical conformance passed all 16 applicable scenarios against the same ESP32 fixture in one run.
- 2026-08-14: iOS physical conformance passed all 16 foreground scenarios against the same fixture on connected device `00008101-00195021268B001E`. During the isolated background checkpoint, location authorization acquisition exceeded its bounded wait and remains deferred; `FloatViewLifecycleTests.testBackground` passed in 0.568 seconds and production BLE telemetry continued from response 160 through response 430 while backgrounded. The already-documented `xcodebuild` parent-process hang then reached the harness timeout after XCTest success, so the passed XCTest and continuous fixture evidence are recorded as the non-location background result.
