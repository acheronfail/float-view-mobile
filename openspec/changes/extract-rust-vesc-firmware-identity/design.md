## Context

Apple and Android now delegate all VESC framing to the safe `vesc-frame` crate, but each production backend still has a separate strict `COMM_FW_VERSION` payload decoder. Those mirrored parsers are small, stateless, and security-relevant: they bound two names, retain a private 12-byte identity, classify motor, stock BMS, and custom-module tails, and reject malformed identities before controller/BMS discovery can proceed.

The offline `vendor/bldc`, `vendor/vesc_bms_fw`, `vendor/vesc_express`, and `vendor/vesc_tool` sources define the emitted fields. The repository already contains immutable THOR300 6.02/6.05/6.06/7.00, Express, stock VBMS 6.00, and VBMS32 6.06 payload evidence exercised by native and conformance tests. This change proves one Rust decoder against that evidence without putting it on a production call path.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust component with an empty production dependency graph dedicated to bounded VESC firmware identity decoding.
- Match the currently shipped structural acceptance, field values, typed failures, name decoding, optional CRC, and private-identity handling.
- Reuse committed source-derived evidence and add adversarial coverage without creating a new Rust-only compatibility policy.
- Keep workspace layout, dependency, unsafe-code, and verification audits exact.
- Prove no regression through `just verify` and focused physical identity scenarios on Android, iOS, and macOS.
- Leave a clean candidate for later, separate Apple and Android bridge/cutover changes.

**Non-Goals:**

- Adding or changing the Apple C ABI, Android JNI ABI, native packaging, or production selection.
- Removing or modifying the Swift/Kotlin decoder behavior or compatibility catalogs.
- Moving CAN routing, command scheduling, BMS values, Refloat, application state, persistence, the loopback server, Bluetooth, location, files, lifecycle, theme, permissions, or UI behavior.
- Sharing Rust allocation or object lifetime across a host boundary, adding generated bindings, or introducing a general VESC runtime crate.
- Changing the frontend contract, SQLite schema, stored data, exports, or user-visible behavior.

## Decisions

### Add one focused safe crate

Add `backends/core/vesc-firmware-identity` as a workspace member with package name `float-view-vesc-firmware-identity`. It will forbid unsafe code and have no normal dependency or third-party development dependency. Production code will not depend on `vesc-frame`: framing and firmware payload interpretation are adjacent but independent protocol layers, and making identity decoding depend on framing would obscure that boundary. Tests may use the existing workspace-local framing crate as a development dependency solely to unwrap committed framed fixture evidence.

A broader `vesc-protocol` crate was rejected because it would invite unrelated routing, telemetry, BMS, and Refloat work before their individual behavior and cutover boundaries are designed. Adding the decoder to `vesc-frame` was rejected because firmware identity is not framing.

### Expose one bounded platform-neutral decode model

The safe API accepts a borrowed payload slice and returns either a `FirmwareIdentity` value or a typed `FirmwareIdentityError`. The value contains major/minor version bytes, bounded hardware and optional firmware names, the private 12-byte UUID, test-version marker, hardware type, custom-configuration count, and optional big-endian hardware-configuration CRC. Pairing and capability bytes that current product behavior does not expose are consumed only to validate the established layout.

Errors distinguish an observed unsupported command byte from structurally invalid/truncated identity input, matching the two stable native error classes. Input and both NUL-terminated names remain bounded by the existing 64-byte name limit. Names use replacement decoding for invalid UTF-8 to preserve the current native behavior; the parser never logs the payload, UUID, or names.

Returning borrowed string slices was rejected because replacement decoding may be required and would complicate a later FFI model. Designing that FFI now was rejected because host representation and packaging belong to later cutover changes.

### Preserve exact hardware-tail behavior

The decoder consumes the common command/version/hardware-name/UUID/pairing/test/type/config prefix. Hardware type `1` is accepted only as exact stock VESC BMS firmware 6.00 with no trailing bytes. Motor type `0` and other custom-module types require the existing four capability bytes, a bounded terminated firmware name, and either no tail or exactly four big-endian CRC bytes. Compatibility admission—supported VESC releases, read-only test builds, VBMS32 name/type/version policy—remains outside this structural decoder.

The parser deliberately mirrors current product acceptance rather than broadening to every optional field decoded by VESC Tool. Any future layout support will be a separate behavior change with shared fixtures.

### Reuse the committed fixture oracle without dependencies

Rust tests will consume the existing committed frame and BMS identity JSON as compile-time text and use a small test-only extractor for the named hex fields. The tests will decode VESC frames with the existing `vesc-frame` crate only as a development dependency if necessary; the preferred shape is to extract the payload hex through a test helper so the identity crate's production dependency graph remains empty. No JSON library will be added solely for trusted fixed fixtures.

Parity covers all audited motor versions, Express, stock VBMS, VBMS32, nonzero test markers, empty firmware names, UUID and count fields, optional/absent CRC, maximum-length names, wrong command, every common-prefix truncation, missing terminators, overlong names, malformed BMS versions/tails, incomplete capability/name/CRC tails, and unexpected trailing bytes. Tests assert the UUID but diagnostics never print it.

### Keep candidate and production evidence distinct

`just verify` will build, format, lint, and test the candidate and audit the expanded workspace. Existing Swift/Kotlin/canonical tests continue to prove the selected production paths. Focused physical `identity` scenarios on the connected Android and iOS devices and macOS host verify that adding the candidate and workspace checks did not regress real BLE discovery and identification; they do not claim the candidate executed.

Separate later changes will design and select an Apple bridge, then an Android bridge, or vice versa. Each host cutover must exercise Rust in focused, packaging, canonical, and physical paths before deleting that host's native parser.

## Risks / Trade-offs

- **The candidate can drift while native parsers remain live.** → Reuse the same immutable source-derived identities, keep the slice small, and follow with per-host cutovers rather than expanding the candidate.
- **A component test cannot prove host representation or packaging.** → Make no production ownership claim; leave all FFI, mapping, and deletion to separate host changes.
- **Current identity layouts are intentionally stricter than generic VESC Tool decoding.** → Treat shipped native behavior and audited fixtures as the product oracle and document compatibility policy as out of scope.
- **Test-only fixture extraction could become an ad hoc JSON parser.** → Limit it to deterministic lookup of committed string fields, add no dependency, and keep it outside production code.
- **The private UUID is necessary output but sensitive evidence.** → Return a fixed array, prohibit logging and formatted full-identity diagnostics, and test redaction through source/audit rules.
- **All-host physical runs do not exercise the candidate.** → Label them regression evidence only; later cutovers must rerun physical identity through the selected Rust path.

## Migration Plan

1. Add the one safe crate and update exact workspace/layout/dependency documentation and audits.
2. Implement the bounded decoder and typed model from offline upstream layouts and current native behavior.
3. Add source-derived and adversarial component tests while preserving native production selection.
4. Run focused Rust/native/conformance checks and the complete `just verify` software gate.
5. Run focused physical identity scenarios through production BLE on Android, iOS, and macOS with the ESP32 fixture.
6. Leave both native decoders in place and follow with one host-cutover OpenSpec change at a time.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no production call-path or data migration to reverse.

## Open Questions

None. Bridge shape and host cutover order are deliberately deferred.
