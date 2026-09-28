# Implementation evidence

## Baseline (fae1ec75)

- Apple: SoftwareVescPeripheral 197 lines; SoftwareBluetoothAdapter 50; unused-after-deletion RefloatCommand/RefloatWire 37, VescCommand 14, VescProtocolError 7. No external references to these protocol helpers.
- Android: SoftwareVescPeripheral 149 lines; SoftwareBluetoothAdapter 56. The peripheral owns its framing/constants privately.
- Direct: conformance/fixture/software.ts and targets/direct.ts own protocol and discovery respectively.
- Selection: macOS environment toggle; iOS simulator detection; Android emulator detection. All currently ship simulator sources in ordinary builds.
- Apple SwiftPM depends on RefloatConfigurationFixtures; iOS copies schemas/values folders. Android main assets include all backends/fixtures (820 KiB filesystem allocation), not just simulator bytes. Retain canonical fixture files for Rust/TypeScript tests; delete only native packaging and unused Swift resource wrapper.
- Apple SharedServerTests depends on the old simulator and must move to the relay. Native production service, Bluetooth ports, Rust codec vectors and physical scenarios remain.

## Transcript reconciliation

All three replicas use equivalent synthetic THOR300 firmware (6.6, zero UUID, one custom configuration, VESC firmware name), CAN ID 42, immutable VBMS 6.00 and extended typical BMS values, legacy Refloat INFO and 55-byte riding telemetry. The THOR300 immutable firmware vector has different optional metadata; preserve the deliberate software configuration-count/name fields and test their exact bytes. BMS matches backends/fixtures/vesc-bms/source-derived.json. INFO matches refloat/protocol-1.0.0.json. Software telemetry uses the legacy riding nibble at offset 10 with deliberate constant values, not the richer immutable riding sample.

Source anchors: vendor/bldc/comm/commands.c COMM_FW_VERSION and custom configuration cases; vendor/vesc_bms_fw/commands.c COMM_BMS_GET_VALUES; vendor/refloat tagged v1.0.0 command and serialization layout; configuration manifest pins source commits and hashes.

Intentional convergence: discovery name becomes Software VESC (Android previously Float View Simulator); fragmented/multiple-frame traffic follows conformance FrameAssembler, malformed CRC/length fails the test session rather than silently timing out; configuration restoration retains the shared model's audited normalization (the old native replicas accepted only the golden write). INFO and telemetry accept only supported exact request shapes, removing TypeScript's permissive prefix matching.

## Shared model checkpoint

`node --strip-types --test test/software.test.ts`: 5/5 pass (golden bytes, configuration mutation/reset, framing, isolation/ordering, selected bounds/authentication). `npx tsc --noEmit`: pass. `npm run direct`: canonical conformance passed, revision 354. Additional relay stress and native checks remain before marking lifecycle/bounds tasks complete.

## Native switch and deletion checkpoint

- Canonical relay-backed conformance passed: macOS revision 358, iOS simulator 376, Android emulator 387. Each includes recording-time harness loss within five seconds; Android includes Activity recreation with the same active session. Direct conformance retains the production runtime and independent Rust codec vectors.
- Focused Swift tests: 9/9. Focused relay tests: 10/10, including exact/+1 byte, message, event and session bounds; 64 pending requests plus rejection; exhausted input budget with reserved close; stalled input expiry; ordered multi-reply draining; duplicate write rejection before mutation; stale generation and run/session isolation.
- Native replicas/helpers removed: 510 source lines. Replacement native transport/selection sources: 351 lines after deadline hardening (no firmware identity, VESC command parsing, framing, configuration model, or product assertion). No new dependency, Rust member or binding family: ownership audit reports 28 members and two host binding crates.
- Baseline packaged source assets: Android main asset wiring included 110 fixture files / 498,138 bytes; iOS schemas/values included 60 files / 424,549 bytes. SwiftPM included those folders plus manifest.json. Final ordinary packages contain zero fixture assets and no test-adapter/selection symbols; checked both Android four-ABI APKs, macOS Debug/Release, iOS ordinary/Release simulator and Debug device artifacts.
- Ordinary native source selection uses CoreBluetooth/AndroidBluetoothAdapter. Apple requires the explicit compile flag plus Debug and a supported software platform; Android compiles a distinct conformance source set. No production network configuration or frontend contract/routes changed.
- Initial `just verify`: passed, including clean native packaging, all software targets, canonical conformance, retained codec vectors and the new packaged simulation-exclusion audit. Final deadline/input-budget hardening receives focused retests and a final gate before archive.

## Real BLE checkpoint

ESP32 `/dev/cu.usbmodem111401`, iPhone `00008101-00195021268B001E`, Android `1cfb7634120d7ece`. Each platform passed `identity,direct,unexpected-disconnect`: discovery/identity, saved riding telemetry, and loss/reconnect continuity. Reports are in `evidence/*-physical.json`. Physical target launchers build ordinary hosts and start no software relay; ESP32 WRITE/NOTIFY traffic and OS adapter disconnect diagnostics establish the physical path. No physical coverage was deferred.

## Selection, formatting and validation

Injected unreachable software credentials were ignored while real ESP32 telemetry passed on macOS Release and ordinary physical iOS/Android. Reports: `evidence/macos-release-selection.json`, `evidence/ios-selection.json`, `evidence/android-selection.json`. The first Release activation probe overlapped an existing software host on the fixed backend port; it was rerun successfully after that host exited (no timeout increase).

`openspec validate share-conformance-simulation --strict` and `git diff --check`: pass. Scoped Prettier checks and native Swift formatting: pass. Whole-repository Prettier reports 40 pre-existing/generated files outside this change (archived specs, existing main specs, `conformance/src/client.ts`, one historical physical report and generated HTML); these were not reformatted. The Linux adapter contract is documented in `conformance/fixture/RELAY.md`.

## Final software gate

Final `just verify` passed on implementation commit `fae12cbf`, with no deferred software target: mock revision 31, direct Rust 354, macOS 355, iOS simulator 369, Android emulator 384. Clean Apple ordinary/Release/device builds and Android Debug/Release four-ABI builds passed, as did frontend/conformance/native/Rust/ESP32 firmware tests, ownership audit, diff checks, and packaged simulation exclusion. The final focused relay suite passed 10/10. Main spec sync and archival are recorded in the final commit.
