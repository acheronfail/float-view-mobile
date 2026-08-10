## Context

`backends/core/vesc-firmware-identity` is a safe, dependency-free, bounded structural decoder. Apple already selects it through a narrow platform bridge. Android still selects `VescFirmwareIdentityDecoder.kt`, which duplicates command, terminated-name, UUID, hardware-tail, and optional-CRC parsing even though its public result and error types already match the Rust component.

Android has an established pattern for source-built Rust JNI: a platform-specific bridge crate with `jni-sys = 0.4.1`, a pinned offline helper, four Android ABI packages, one explicit host library for JVM tests, caller-owned primitive arrays, exact JNI symbol audits, and panic containment. The identity cutover can reuse that pattern without changing the existing framing bridge or introducing a general native runtime.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android owner of `COMM_FW_VERSION` structural decoding.
- Keep Kotlin limited to loading the bridge, validating a bounded primitive record, mapping it to the existing `FirmwareInfo`, and mapping fixed statuses to existing errors.
- Prove source-derived and adversarial parity through the compiled JNI boundary and real BLE on Android, while rerunning Apple physical identity regression.
- Delete the superseded Kotlin structural parser and enforce that deletion in repository audits.

**Non-Goals:**

- Moving firmware compatibility admission, controller routing/discovery, VESC request scheduling, BMS/Refloat protocols, application behavior, loopback transport, SQLite, or any OS capability.
- Changing `FirmwareInfo`, the frontend contract, persistence, fixture semantics, or public error behavior.
- Sharing an Apple bridge artifact with Android, changing the Apple bridge, merging identity into the framing JNI library, or creating generated bindings/general callbacks.

## Decisions

### Use a separate Android identity JNI crate

Add `vesc-firmware-identity-android-jni` beside the safe decoder and existing host bridges. It depends only on the safe identity crate, the Rust standard library, and the already locked minimal `jni-sys` binding. This keeps symbol and dependency ownership obvious and lets framing remain independently audited.

Alternatives rejected:

- Adding identity methods to `vesc-frame-android-jni` would make the framing bridge own an unrelated protocol model and couple future cutovers.
- Calling the Apple C ABI from Android would blur platform packaging and JNI representation ownership.
- A general cross-platform FFI/runtime layer would be much larger than this boundary and recreate the failed big-bang migration shape.

### Cross one bounded caller-owned record

The JNI class exposes exactly `abiVersion(): Int` and `decode(input, output, result): Int`. `decode` accepts a nullable Java byte-array input, byte-array output, and exact two-element integer result. Rust rejects null, aliased, oversized, or wrongly shaped regions; initializes result metadata; copies the bounded input; invokes the safe decoder; and copies a successful byte record back only after all Rust work completes. `result[0]` is produced or required record length and `result[1]` is the unsupported command byte.

ABI version 1 uses statuses `ok`, `invalid argument`, `unsupported command`, `truncated payload`, `output too small`, and `internal failure`. The record uses the same logical 26-byte fixed header and bounded UTF-8 name representation already proven by Apple: major, minor, test marker, hardware type, custom-configuration count, presence flags, two big-endian name lengths, 12 UUID bytes, optional CRC value storage, then hardware and firmware name bytes. The record is at most 410 bytes. Rust allocations, references, layouts, strings, callbacks, handles, and private diagnostics never cross JNI.

Duplicating the small host-specific record encoder is accepted because extracting it into the safe decoder would mix FFI representation into protocol ownership and changing Apple's verified bridge would broaden regression scope. Immutable cross-bridge vectors and equivalent bounds constrain the duplication.

### Retain the decoder object as the thin production adapter

Keep `VescFirmwareIdentityDecoder` and its public constants/call site so higher layers do not change. Its `decode` allocates fixed bounded arrays, verifies the linked ABI, invokes the bridge exactly once, validates status/result/record flags and lengths, strictly decodes the Rust-produced UTF-8, and constructs `FirmwareInfo`. Unsupported-command status maps to `UnsupportedCommand`; every structural, ABI, caller-contract, impossible-record, or internal bridge outcome maps to `TruncatedPayload`, matching existing identity behavior. There is no Kotlin payload parser or fallback.

### Build and verify identity independently

Use a dedicated pinned build helper and disjoint generated output directories for host, debug, and release identity libraries. Gradle loads both independently built JNI libraries in host tests and packages both for every supported Android ABI. Root audits allowlist exactly two identity JNI symbols, verify dependency depth/locked JNI version/ABI agreement, reject committed binaries and generated bindings, and reject the deleted Kotlin structural signatures.

Focused Rust and JVM tests cover source-derived motor, Express, stock-BMS, and VBMS32 input plus malformed payloads, replacement UTF-8, JNI null/shape/alias/bounds behavior, statuses, metadata, record validation, and redaction. `just verify` remains the complete software gate. Focused ESP32 identity scenarios run on the connected Android and iOS devices and macOS host; location sample timing is deferred without masking BLE, identity, route, session, or cleanup failures.

## Risks / Trade-offs

- **[Two JNI libraries increase Android packaging surface]** → Keep separate exact symbol/library allowlists and assert both libraries for all four ABIs in debug and release APKs.
- **[Host-specific record encoders could drift]** → Reuse the established logical record layout and bounds, cover both with the same immutable identity evidence, and keep the safe decoder as the sole structural source.
- **[A malformed native record could be trusted accidentally]** → Kotlin validates ABI, status, result shape, flags, exact computed extent, presence semantics, UTF-8, UUID extent, and CRC representation before constructing a model.
- **[JNI misuse or panic could cross the boundary]** → Validate references, array lengths, aliasing and pending exceptions, bound all copies, and contain panics into a fixed internal-failure status.
- **[Physical regression could be mistaken for location delay]** → Defer only absence of a location sample; any BLE, identity, routing, lifecycle, session, or cleanup failure remains blocking.

## Migration Plan

1. Add and audit the bridge crate/helper, Gradle wiring, and focused raw-boundary tests without changing higher-level APIs.
2. Convert the existing decoder object into the thin Rust record adapter and extend production-path parity tests.
3. Pass focused Rust/JVM checks, clean Android packaging, and the full software gate.
4. Run focused physical identity regression on Android, iOS, and macOS against the ESP32 fixture.
5. Enforce absence of the superseded Kotlin parser and rerun the full software gate before archiving.

Rollback is one repository revert because no contract, database, persisted representation, or migration changes. Generated native outputs remain disposable and untracked.

## Open Questions

None.
