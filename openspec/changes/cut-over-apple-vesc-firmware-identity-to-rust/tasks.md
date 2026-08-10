## 1. Add the bounded Apple identity bridge

- [x] 1.1 Add the `vesc-firmware-identity-apple-ffi` workspace crate with the versioned two-symbol caller-owned record ABI, exact safe-crate dependency, fixed status mapping, region validation, and panic containment.
- [x] 1.2 Add bridge unit tests plus a hand-written `CVescFirmwareIdentity` header/module and compiled C harness covering source-derived success, unsupported/truncated input, output sizing, null/overlap/capacity failures, record bounds, and private-data-safe diagnostics.
- [x] 1.3 Add the locked offline Apple build helper and extend workspace, dependency, unsafe-code, generated-artifact, and exact identity/framing symbol audits without changing Android packaging.

## 2. Integrate both sibling Apple hosts

- [x] 2.1 Add the identity C module/archive to SwiftPM and root Apple build, test, run, and packaging entry points while retaining the existing framing bridge independently.
- [x] 2.2 Add a separate declared Xcode build phase, inputs, output, include path, and link flag for clean iOS device and simulator Debug/Release builds.

## 3. Cut shared Apple decoding over to Rust

- [x] 3.1 Replace `VescFirmwareIdentityDecoder` internals with bounded ABI/result validation and mechanical `FirmwareInfo` mapping that preserves constants and established error classes.
- [x] 3.2 Extend shared Apple protocol tests to prove every audited motor, Express, stock/custom BMS, private field, replacement UTF-8, malformed input, ABI/status, and invalid result case executes through Rust.
- [x] 3.3 Delete the superseded Swift common-prefix, terminator, hardware-tail, and CRC-tail parser and add no-fallback/source-selection audits while preserving Kotlin production ownership.

## 4. Verify software and physical regression

- [x] 4.1 Run focused Rust formatting/lint/tests, the compiled C harness and symbol audit, shared Swift protocol tests, clean Apple packaging, Android unit/packaging regression, and canonical conformance.
- [x] 4.2 Run the complete `just verify` software gate after Swift parser deletion.
- [x] 4.3 Run focused ESP32 physical identity conformance on the connected Android device through the unchanged Kotlin decoder.
- [x] 4.4 Run focused ESP32 physical identity conformance on the connected iOS device through the selected Rust decoder.
- [x] 4.5 Run focused ESP32 physical identity conformance on the macOS host through the selected Rust decoder; defer only a missing bounded location observation.
