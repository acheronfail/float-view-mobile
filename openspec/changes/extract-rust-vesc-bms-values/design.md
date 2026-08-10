## Context

Apple and Android now delegate VESC framing, firmware identity, and custom-configuration envelopes to safe Rust components, but each backend still implements `VescPayloadDecoder.bmsValues`. Those mirrored decoders parse the command-96 body into fixed electrical/environmental values, bounded cell and temperature arrays, balancing flags, a reported CAN ID, VESC auto-floats, and either an exact legacy end or an extended version/status tail.

Pinned `vesc_bms_fw`, `vesc_express`, and `vesc_tool` sources define the layout. The repository already contains immutable source-derived legacy and extended payloads, native minimum/maximum/adversarial tests, canonical malformed-response tables, and physical standalone/direct-controller/Express BMS scenarios. This change proves one safe Rust decoder against that evidence without selecting it from a host.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust component with an empty production dependency graph dedicated to bounded command-96 BMS values decoding.
- Match native fixed-point, auto-float, collection, flag, route, tail, UTF-8, remainder, numeric-finiteness, and failure behavior.
- Reuse the immutable source-derived payloads and cover exact minimum/maximum and every malformed structural class without adding a fixture parser dependency.
- Keep workspace layout, dependency, unsafe-code, privacy, bridge-symbol, packaging, and production-selection audits exact.
- Prove no repository or real-BLE regression through `just verify` and focused BMS physical scenarios on Android, iOS, and macOS.
- Leave a clean candidate for later, separate Apple and Android bridge/cutover changes.

**Non-Goals:**

- Adding or changing the Apple C ABI, Android JNI ABI, native packaging, or production selection.
- Removing or changing the Swift/Kotlin BMS values parser, `ByteReader`, BMS compatibility/discovery, request scheduling, polling, routing, lifecycle, or state publication.
- Moving firmware identity, CAN forwarding, realtime/controller values, Refloat protocols, application behavior, SQLite, the loopback server, Bluetooth, location, files, lifecycle, theme, permissions, or UI behavior.
- Adding a general BMS runtime, generated bindings, callbacks, or Rust-owned lifetime across a future host boundary.
- Changing the frontend contract, stored data, exports, diagnostics, or user-visible behavior.

## Decisions

### Add one focused safe decoder crate

Add `backends/core/vesc-bms-values` as workspace package `float-view-vesc-bms-values`. It will forbid unsafe code and have no normal or third-party development dependency. It will remain independent of framing, firmware identity, routing, host/application models, and every bridge crate.

A broader `vesc-protocol`, complete BMS subsystem, or `VescPayloadDecoder` extraction was rejected because each would combine unrelated routing, realtime/controller decoding, discovery, scheduling, or application lifecycle before those boundaries have independent parity evidence. Extracting only two-byte CAN wrapping was rejected as too small to retire meaningful duplicated protocol behavior.

### Return one bounded platform-neutral values model

The public safe API will accept a borrowed payload plus an optional expected reported CAN ID and return a platform-neutral decoded value. Cell voltages, balancing flags, and temperatures will use fixed-capacity 50-element arrays plus explicit counts so memory remains statically bounded and future bridges need no Rust allocator ownership. The optional status may borrow valid UTF-8 from the input because it is at most 40 bytes and the safe result never crosses a host boundary in this change.

Observation time, native `BmsSnapshot`, scheduler state, and availability counters remain outside the decoder. Returning host dates/models or application status was rejected because those are representation and lifecycle concerns. Designing an FFI/JNI result record now was rejected because copy policy, ABI mapping, packaging, and native error translation belong to later host cutovers.

### Preserve exact command-96 structure

The candidate will require command byte 96 and decode the same six signed 32-bit fixed-point leading fields, bounded variable cell/balancing/temperature sections, seven signed 16-bit fixed-point environmental/state fields, reported CAN byte, four VESC auto-floats, and pressure. Reported CAN IDs above 253 or different from an expected route fail closed.

Zero through 50 cells and temperatures are valid. Every balancing byte is exactly zero or one. A payload ending immediately after pressure is the legacy form. Otherwise exactly one data-version byte and one valid UTF-8 NUL-terminated status of at most 40 content bytes must consume the remainder. Every decoded number must be finite, and auto-floats outside finite `Float` range fail structurally as the selected native implementations do.

Errors remain a closed unsupported-command or malformed/truncated class and must not format the payload, arrays, status, or values. Command ID may remain in the bounded unsupported diagnostic, matching established protocol errors.

### Use existing source-derived evidence without dependencies

Tests will `include_str!` the committed `backends/fixtures/vesc-bms/source-derived.json` and use a tiny test-only named-hex extractor like the existing firmware-identity component. The crate will not add JSON, hex, fixture-generation, or numeric dependencies.

Tests will cover both committed values payloads, all expected fields, exact zero/50 collection limits, 40-byte status, every prefix truncation, excessive counts, invalid flags, wrong/empty commands, route mismatch, invalid and oversized UTF-8 status, unexpected remainder, non-finite/overflowing auto-floats, fixed-point signs/scales, redacted errors, and deterministic bounded arrays.

### Keep candidate and production evidence distinct

`just verify` will build, format, lint, and test the candidate and audit the expanded workspace. Existing Swift/Kotlin and canonical tests continue to prove production behavior. Focused `bms-direct-controller`, `bms-express`, and `bms-standalone` physical scenarios on the connected Android/iOS devices and macOS host verify no regression in production Bluetooth, direct/CAN routing, decoding, polling, state, and cleanup; they do not claim the candidate executed.

Later Apple and Android changes will introduce and select separate bridges. Each host cutover must exercise command 96 through Rust, preserve observed-time/application mapping, pass clean packaging and physical BMS checks, and delete that host's native structural parser before claiming ownership transfer.

## Risks / Trade-offs

- **The candidate can drift while native decoders remain live.** → Reuse the same source-derived fixtures and malformed classes, audit production call sites, and follow with per-host cutovers before expanding scope.
- **Fixed arrays make the safe model more verbose than native lists.** → They make the 50-element resource bound explicit and simplify later caller-owned bridge buffers without introducing an allocator contract.
- **Borrowed status text constrains the safe result lifetime.** → The lifetime is local and explicit; later bridges will copy into caller-owned buffers during the call rather than exposing it.
- **VESC auto-float behavior is unusual.** → Derive the exact algorithm from pinned source, compare all committed and boundary values, and reject non-finite or out-of-range results identically.
- **Physical runs do not execute the candidate.** → Label them regression evidence only; later cutovers must run BMS decoding through their selected Rust bridges before native deletion.

## Migration Plan

1. Add the focused safe crate and update exact workspace, ownership documentation, dependency checks, and production-isolation audits.
2. Implement strict bounded decoding and source-derived/adversarial tests.
3. Run focused Rust/native/conformance/audit checks and the complete `just verify` gate.
4. Run the three focused BMS physical scenarios through production BLE on Android, iOS, and macOS.
5. Leave both native decoders selected and follow with one host-cutover OpenSpec change at a time.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no production call-path or data migration to reverse.

## Open Questions

None. Bridge representation, host cutover order, BMS discovery/polling migration, and other VESC payload decoders are deliberately deferred.
