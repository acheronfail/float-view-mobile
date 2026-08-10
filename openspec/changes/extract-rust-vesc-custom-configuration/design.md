## Context

Apple and Android now delegate VESC framing and firmware identity decoding to safe Rust components, but each backend still has a separate `VescCustomConfigurationCodec`. Those mirrored codecs own the command-92 schema request and chunk envelope, command-93 current values, command-94 defaults, command-95 writes and acknowledgements, signed big-endian extents, index bytes, and structural rejection before the larger Refloat configuration system sees schema or value bytes.

The offline `vendor/bldc` and `vendor/vesc_tool` sources define commands 92–95. The repository already contains immutable compressed schema and current/default/write/readback values for every audited Refloat 1.0.0–1.2.3 release, plus native and conformance implementations of the same envelope. This change proves one Rust codec against that evidence without putting it on a production call path.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust component with an empty production dependency graph dedicated to bounded custom-configuration envelope encoding and decoding.
- Match the selected native request bytes, response variants, signed big-endian bounds, minimum value signature extent, acknowledgement extent, and stable error distinction.
- Reuse all nine immutable Refloat configuration releases and add complete boundary/adversarial coverage without parsing their schema or typed values.
- Keep workspace layout, dependency, unsafe-code, bridge-symbol, privacy, and production-selection audits exact.
- Prove no repository or real-BLE regression through `just verify` and focused direct/CAN Refloat physical scenarios on Android, iOS, and macOS.
- Leave a clean candidate for later, separate Apple and Android bridge/cutover changes.

**Non-Goals:**

- Adding or changing the Apple C ABI, Android JNI ABI, native packaging, or production selection.
- Removing or changing the Swift/Kotlin codec, Refloat schema parser, typed value codec, configuration service, compatibility policy, scheduler, routing, mutation authority, or persistence.
- Moving CAN forwarding, telemetry, BMS, application state, SQLite, the loopback server, Bluetooth, location, files, lifecycle, theme, permissions, or UI behavior.
- Sharing Rust allocation or object lifetime across a host boundary, adding generated bindings, or creating a general VESC runtime crate.
- Changing the frontend contract, stored data, exports, or user-visible behavior.

## Decisions

### Add one focused safe crate

Add `backends/core/vesc-custom-configuration` as a workspace member with package name `float-view-vesc-custom-configuration`. It will forbid unsafe code and have no normal or third-party development dependency. It will not depend on `vesc-frame`: frame validation and custom-configuration payload interpretation are adjacent but independent layers, and later bridges can compose both without coupling the safe components.

A broader `vesc-protocol` or `refloat-configuration` crate was rejected for this step because either would mix routing, schema decompression/XML, typed values, safety policy, or lifecycle before those boundaries have independent parity evidence. Moving only CAN wrapping was rejected as too small to retire meaningful duplicated protocol behavior.

### Expose typed requests and borrowed responses

The safe API will encode schema, current/default, and write requests from bounded primitive inputs. It will decode a borrowed payload into one of three variants: schema chunk with index/total/offset/borrowed bytes, current/default values with command/index/borrowed bytes, or write acknowledgement. Borrowing response bytes keeps the component allocation-free on decode and does not commit a future host ABI to a Rust layout or lifetime.

The codec will expose typed invalid-payload and unsupported-command errors without formatting payload bytes. It will preserve the selected native distinction: a recognized non-configuration VESC command is unsupported, while empty or unrecognized command input is structurally invalid. Request construction validates the complete index and integer domain before narrowing to wire bytes.

Returning host-specific `Data`, `ByteArray`, command enums, or Refloat models was rejected because those are representation and higher-layer concerns. Designing an FFI record now was rejected because bridge shape, copy policy, packaging, and host error mapping belong to later cutovers.

### Preserve exact command envelope behavior

Schema requests are exactly ten bytes: command 92, index, positive length no larger than 400, and nonnegative signed-32-bit offset. Schema responses require a positive signed-32-bit total, nonnegative signed-32-bit offset, at most 400 chunk bytes, and a chunk wholly within total extent; a zero-byte chunk at `offset == total` remains valid because the selected codecs admit it.

Current/default requests are exactly command 93/94 plus index. Their responses require index plus at least the four-byte configuration signature and preserve the remaining opaque bytes without interpreting them. Writes require at least one opaque value byte and retain the established scheduled-request bound; acknowledgement is exactly command 95 with no index or tail. The candidate does not validate schema compression, signatures, or serialized values.

The component keeps command constants and bounds local and audited rather than depending on host/application types. Tests cover actual production framing bounds as a composition constraint while preserving the existing codec's scheduled-request write bound for later host parity decisions.

### Use every committed release as the fixture oracle

Rust tests will use `include_bytes!` over the committed schema and value files for Refloat 1.0.0–1.2.3. For each release they will round-trip request bytes and decode schema chunks, current/default values, writes, readbacks, and acknowledgements without adding JSON, compression, XML, or hash dependencies. The envelope codec treats those bytes as opaque.

Adversarial tests cover zero/negative/overflowing lengths and offsets, every truncated fixed field, chunk size and total/offset arithmetic boundaries, minimum and excessive value extents, empty and oversized writes, all indexes, current/default command identity, exact acknowledgement length, recognized unsupported commands, unknown commands, and payload-safe errors.

### Keep candidate and production evidence distinct

`just verify` will build, format, lint, and test the candidate and audit the expanded workspace. Existing Swift/Kotlin and canonical tests continue to prove the selected production paths. Focused physical `identity`, `control-direct`, and `control-can` scenarios on the connected Android and iOS devices and macOS host verify that adding the candidate and checks did not regress BLE discovery, direct/CAN routing, Refloat readiness, safe controls, or cleanup; they do not claim the candidate executed.

Separate later changes will design and select an Apple bridge and an Android bridge one at a time. Each host cutover must exercise commands 92–95 through Rust in focused, packaging, canonical, and physical paths before deleting that host's native codec.

## Risks / Trade-offs

- **The candidate can drift while native codecs remain live.** → Reuse every immutable release, keep the slice small, audit production call sites, and follow with per-host cutovers rather than expanding the crate.
- **The native codecs inherit different type-level index validation.** → Define one explicit byte-domain safe API, prove all production index-0 behavior and full byte boundaries, and leave host argument/error mapping to each cutover design.
- **Scheduled-request and framing bounds are not identical.** → Preserve the current codec write bound as candidate parity, test the 4,096-byte frame composition boundary separately, and do not claim a production behavior change in this step.
- **Opaque configuration bytes may contain sensitive tuning values.** → Borrow or encode bytes without logging them and keep errors free of payload content.
- **Physical runs do not execute the candidate.** → Label them regression evidence only; later cutovers must run configuration operations through the selected Rust path before native deletion.

## Migration Plan

1. Add the one safe crate and update exact workspace/layout/dependency documentation and audits, including correction of completed Android identity ownership.
2. Implement bounded request encoding, borrowed response decoding, typed failures, and source-derived/adversarial tests.
3. Run focused Rust, native, conformance, ownership, dependency, and packaging checks followed by the complete `just verify` gate.
4. Run focused direct/CAN Refloat physical regression through production BLE on Android, iOS, and macOS with the ESP32 fixture.
5. Leave both native codecs in place and follow with one host-cutover OpenSpec change at a time.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no production call-path or data migration to reverse.

## Open Questions

None. Bridge representation, host cutover order, and later Refloat schema/value extraction are deliberately deferred.
