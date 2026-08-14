## 1. Dependency and State Model

- [x] 1.1 Add the existing protocol-neutral VESC controller-data crate as the coordinator's only new dependency, without adding a host bridge or runtime dependency.
- [x] 1.2 Extend coordinator state and snapshots with optional current limits while keeping debug output presence-only and telemetry values private.
- [x] 1.3 Add route-correct current-limit request and completion adapters that reuse the controller-data decoder and request scheduler context.

## 2. Application Lifecycle Composition

- [x] 2.1 Clear current-limit state on connection start, reset, and generation replacement so stale values cannot cross sessions.
- [x] 2.2 Enqueue exactly one command-14 controller-work request after configuration activation and before telemetry or BMS startup, preserving direct and CAN routes and the native timeout.
- [x] 2.3 Exclude the controller current-limit request from standalone-BMS activation.
- [x] 2.4 Decode and publish only an exactly attributed successful current-limit response while leaving malformed, failed, timed-out, cancelled, stale, or repeated completions nonfatal and absent.

## 3. Evidence and Integration

- [x] 3.1 Add source-derived direct, CAN, standalone-BMS, ordering, valid-response, unknown-signature, and optional-cell current-limit fixtures and tests.
- [x] 3.2 Add adversarial tests for truncation, malformed numeric fields, incorrect attribution, completion failures, reset isolation, and privacy-safe debug output.
- [x] 3.3 Add focused repository verification and ownership audits that reject copied controller decoding, host-package integration, and new bridge dependencies.
- [x] 3.4 Confirm the production Swift and Kotlin application services and every `*-apple-ffi` and `*-android-jni` package remain unchanged by this candidate-only composition.

## 4. Verification

- [x] 4.1 Run focused Rust tests, formatting, linting, dependency/ownership audits, strict OpenSpec validation, and `git diff --check`.
- [x] 4.2 Run the complete `just verify` software gate.
- [x] 4.3 Run the ESP32 fixture regression on the macOS host and connected Android and iOS devices, deferring only bounded location acquisition and documenting any harness-only timeout separately from application behavior.
- [x] 4.4 Record exact software and physical evidence in the change artifacts and leave all tasks complete before sync and archive.

## Verification Evidence

- Focused verification passed: coordinator tests (including three dedicated current-limit tests), offline locked workspace Clippy with warnings denied, formatting, dependency/copy/production-isolation audit, strict change validation, and `git diff --check`.
- `just verify` passed completely, including frontend, conformance, Apple, Android, locked Rust workspace, ABI, packaging, and audit gates.
- macOS physical conformance passed all 16 scenarios against `/dev/cu.usbmodem111401` after one clean retry of a pre-state identity disconnect.
- Android physical conformance passed all 17 scenarios on `1cfb7634120d7ece`. The ESP32 repeatedly reported `ADVERTISE_FAILED` on a second scenario after Android cleanup, so already-passed work was preserved and remaining scenarios ran in clean resumable shards; every scenario that reached the app passed. Background BLE/telemetry passed while location acquisition was explicitly deferred.
- iOS physical conformance passed the first 16 scenarios on `00008101-00195021268B001E`. In the final background scenario, `FloatViewLifecycleTests.testBackground` passed in 0.590 seconds with zero failures and BLE continued for 140 seconds with more than 310 telemetry responses; only the parent `xcodebuild` failed to exit within the harness's 120-second bound. This is recorded as a harness-only timeout, not an application regression. Location acquisition was explicitly deferred.
