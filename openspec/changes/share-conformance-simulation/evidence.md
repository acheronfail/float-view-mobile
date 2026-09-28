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
