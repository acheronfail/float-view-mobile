## 1. Android Identity Bridge

- [ ] 1.1 Add the Android JNI identity bridge crate to the locked Rust workspace with the fixed two-symbol ABI, bounded record/status encoding, panic containment, and focused safe/record tests.
- [ ] 1.2 Implement null, shape, alias, extent, pending-exception, copy, and metadata handling over `jni-sys`, with raw JNI tests covering success and every bounded failure class.
- [ ] 1.3 Add a pinned offline host/four-ABI build helper with exact symbol and artifact architecture audits and no committed native output.

## 2. Android Production Selection

- [ ] 2.1 Wire disjoint identity host/debug/release outputs into Gradle unit tests and all four Android ABI packages without changing the framing bridge.
- [ ] 2.2 Add the thin Kotlin loader and convert `VescFirmwareIdentityDecoder` to one-call ABI/status/record validation and `FirmwareInfo` mapping with no Kotlin structural fallback.
- [ ] 2.3 Extend focused JVM production-path and raw-bridge tests for source-derived identities, replacement UTF-8, malformed payloads, ABI/status/result corruption, established errors, bounds, and redaction.
- [ ] 2.4 Extend root dependency, symbol, packaging, ABI, ownership, and source-deletion audits to require the Android identity bridge and reject the superseded Kotlin parser.

## 3. Software Verification

- [ ] 3.1 Pass locked Rust formatting, warnings-denied linting, workspace tests, focused Android identity/JNI tests, exact audit, and clean debug/release four-ABI packaging.
- [ ] 3.2 Pass the complete `just verify` software gate after Android selects Rust.

## 4. Physical Cutover and Final Gate

- [ ] 4.1 Run focused ESP32 physical identity conformance through production BLE on the connected Android device, connected iOS device, and macOS host, deferring only a missing bounded-time location sample.
- [ ] 4.2 Confirm the Kotlin structural parser is absent and rerun the complete `just verify` gate after physical cutover evidence succeeds.
