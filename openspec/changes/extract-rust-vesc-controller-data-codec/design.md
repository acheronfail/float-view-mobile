## Context

Apple and Android already delegate five focused protocol families to safe Rust, but both backends still carry equivalent `ByteReader` implementations and duplicate parsing for the standard VESC controller values used during discovery and live state. The duplicated boundary is smaller than Refloat telemetry: `COMM_GET_VALUES` contributes a fixed 30-byte consumed prefix while later standard fields remain intentionally unconsumed, and `COMM_GET_MCCONF` contributes a fixed command/signature/header/current-limit prefix plus one optional battery-series-cell byte at a firmware-layout-specific offset.

Pinned `bldc` and `vesc_tool` sources define the command-4 field order, signed fixed-point scales, command-14 serialization, and VESC auto-float representation. The repository already contains immutable normal and over-voltage realtime fixtures, a VESC 7.00 motor-configuration vector, native parity tests, direct/CAN simulation, and physical scenarios that verify controller discovery, current limits when published, telemetry, sessions, controls, and cleanup. This change proves a safe Rust codec against that evidence without selecting it from a host.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust component with an empty production dependency graph dedicated to the consumed `COMM_GET_VALUES` and `COMM_GET_MCCONF` subsets.
- Match native command validation, signed fixed-point scaling, VESC auto-float behavior, fault classification, supported serialization signatures, optional series-cell lookup, and finite/nonzero current-limit validation.
- Reuse immutable repository evidence and cover every consumed boundary and failure class without adding a fixture-parser dependency.
- Keep workspace layout, dependency, unsafe-code, privacy, bridge-symbol, packaging, and production-selection audits exact.
- Prove no repository or real-BLE regression through `just verify` and focused physical scenarios on Android, iOS, and macOS.
- Leave a clean candidate for later, separate Apple and Android bridge/cutover changes.

**Non-Goals:**

- Adding or changing the Apple C ABI, Android JNI ABI, native packaging, or production selection.
- Removing or changing the Swift/Kotlin byte reader, realtime decoder, current-limit decoder, request routing, framing, scheduler, or application models.
- Decoding unconsumed command-4 fields or the complete motor configuration.
- Moving firmware compatibility, Refloat telemetry/layouts, BMS, controller shutdown, application behavior, SQLite, the loopback server, Bluetooth, location, files, lifecycle, theme, permissions, or UI behavior.
- Adding a general VESC runtime, generated bindings, callbacks, or Rust-owned lifetime across a future host boundary.
- Changing the frontend contract, stored data, exports, diagnostics, or user-visible behavior.

## Decisions

### Add one focused safe controller-data crate

Add `backends/core/vesc-controller-data` as workspace package `float-view-vesc-controller-data`. It will forbid unsafe code and have no normal or third-party development dependency. It will remain independent of framing, CAN forwarding, firmware identity, host/application models, compatibility catalogues, scheduling, persistence, and every bridge crate.

Extracting only command 4 was rejected because the shared byte reader and auto-float implementation would remain duplicated for current limits. Extracting complete motor-configuration serialization was rejected because Float View consumes only four limits and an optional cell count, and complete schemas vary substantially between firmware releases. Refloat telemetry was deferred because its dynamic field lists, extension keys, and large application record require a wider model and future bridge.

### Model only the consumed protocol subsets

The safe API will expose `decode_realtime_values(&[u8])` and `decode_current_limits(&[u8])`, returning plain fixed-size Rust values. Realtime decoding requires the complete 30-byte consumed prefix, validates command 4, and ignores permitted later fields just as the native decoders do. It returns controller and motor temperature, motor and battery current, duty cycle, ERPM, pack voltage, raw fault code, and a closed `None`/`OverVoltage`/`Unknown` classification.

Current-limit decoding validates command 14, reads the four-byte serialization signature, skips the four enum bytes, and decodes four VESC auto-floats. It recognizes the existing 6.02, 6.05, 6.06, 7.00, and 7.01 serialization signatures and reads a positive battery-series-cell byte only when that signature's offset is present. Unknown signatures or absent/zero offset bytes yield `None`; they do not invalidate otherwise sound limits. All four limits must be finite and nonzero.

Returning framed or CAN-prefixed bytes was rejected because route ownership and framing already have independent components. Returning host-specific errors or models was rejected because representation mapping belongs to later bridge changes.

### Preserve VESC numeric behavior and closed errors

Signed fixed-point fields use exact big-endian two's-complement decoding and their established divisors. ERPM remains a signed 32-bit integer. VESC auto-float decoding follows the pinned upstream exponent/significand algorithm, accepts encoded zero, and rejects non-finite or out-of-`Float`-range results before current-limit validation.

The public error surface remains `UnsupportedCommand(u8)` or `TruncatedPayload`. Structural truncation, invalid numeric encodings, and zero limits use the bounded truncated failure to preserve existing native behavior. Errors and `Debug` output expose no payload bytes or decoded private values.

### Use existing evidence without production dependencies

Tests will `include_str!` the two committed realtime JSON fixtures and use a tiny test-only hex extractor. Source-derived motor-configuration vectors will cover all five recognized signatures and offsets, plus unknown, missing, zero, and positive series-cell cases. Tests will cover minimum consumed extents, accepted command-4 tails, every prefix truncation, wrong commands, signed extremes, scaling, fault classifications, auto-float zero/sign/range behavior, nonzero validation, deterministic output, and redacted errors.

The crate will not add JSON, code-generation, numeric, host, or bridge dependencies.

### Keep candidate and production evidence distinct

`just verify` will build, format, lint, and test the candidate and audit the expanded workspace. Existing Swift/Kotlin and canonical tests continue to prove production behavior. Focused physical identity, direct/CAN control, and mobile background scenarios verify no regression in production Bluetooth, controller reads, telemetry, state, sessions, and cleanup; they do not claim the candidate executed.

Later Apple and Android changes will introduce and select separate bridges. Each host cutover must exercise controller data through Rust, preserve routing/scheduling/application behavior, pass clean packaging and physical checks, and delete that host's native parsing before claiming ownership transfer.

## Risks / Trade-offs

- **The candidate can drift while native decoders remain live.** → Reuse the same immutable fixtures, cover every consumed byte and signature, audit production call sites, and follow with per-host cutovers before expanding scope.
- **The candidate intentionally ignores later command-4 fields.** → Document the 30-byte consumed prefix and test accepted tails so the extraction exactly matches current behavior instead of silently broadening the application model.
- **Firmware-layout offsets are a closed compatibility table.** → Test every recognized signature and make unknown or missing offsets return no cell count without rejecting otherwise valid limits.
- **Physical runs do not execute the candidate.** → Label them regression evidence only; later cutovers must run live reads through their selected Rust bridges before native deletion.

## Migration Plan

1. Add the focused safe crate and update exact workspace, ownership documentation, dependency checks, and production-isolation audits.
2. Implement strict bounded realtime and current-limit decoding with source-derived and adversarial tests.
3. Run focused Rust/native/conformance/audit checks and the complete `just verify` gate.
4. Run focused controller physical scenarios through production BLE on Android, iOS, and macOS.
5. Leave both native decoders selected and follow with one host-cutover OpenSpec change at a time.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no production call-path or data migration to reverse.

## Open Questions

None. Bridge representation, host cutover order, complete motor-configuration decoding, Refloat telemetry migration, and application behavior are deliberately deferred.
