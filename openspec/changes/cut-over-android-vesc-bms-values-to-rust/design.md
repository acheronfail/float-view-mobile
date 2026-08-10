## Context

`backends/core/vesc-bms-values` is a safe, dependency-free, bounded structural decoder for `COMM_BMS_GET_VALUES = 96`. Apple already selects it through a focused two-operation C ABI. Android still selects the `VescPayloadDecoder.bmsValues` Kotlin implementation, which duplicates fixed-point fields, cell and temperature collections, balancing flags, reported-CAN validation, VESC auto-floats, finite-value checks, and the optional UTF-8 status tail even though its public `BmsSnapshot` result and errors already match the Rust component.

Android has an established source-built Rust JNI pattern for framing, firmware identity, and custom configuration: platform-specific bridge crates using the locked minimal `jni-sys` binding, pinned offline helpers, four Android ABI packages, explicit host libraries for JVM tests, caller-owned primitive arrays, exact symbol audits, and panic containment. This change can reuse that pattern while keeping the safe component and Apple's bridge unchanged.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of command-96 structural decoding.
- Keep Kotlin limited to loading the bridge, validating a bounded primitive record, mapping it to the existing `BmsSnapshot`, attaching the existing observation time, and mapping fixed statuses to established errors.
- Build and package the focused bridge from locked source for host JVM tests and all four supported Android ABIs in debug and release.
- Prove source-derived and adversarial parity through the compiled JNI boundary, then prove the selected production path through existing BMS physical scenarios on Android while rerunning Apple regression.
- Delete the superseded Kotlin structural parser and enforce its absence with repository audits.

**Non-Goals:**

- Moving BMS firmware compatibility, discovery, request construction, direct/CAN routing, scheduling, polling, observation time, availability, state publication, or live-only policy.
- Moving other VESC or Refloat protocols, application behavior, SQLite, loopback transport, BLE, background work, location, lifecycle, files, theme, permissions, persistence, WebViews, or the frontend contract.
- Sharing the Apple bridge artifact with Android, modifying any Apple bridge or adapter, merging BMS values into an existing Android JNI library, or creating generated bindings, callbacks, Rust-owned handles, or a general native runtime.

## Decisions

### Use a separate two-operation Android BMS JNI crate

Add `vesc-bms-values-android-jni` beside the safe decoder and existing host bridges. It depends only on the safe BMS-values crate, the Rust standard library, and the already locked minimal `jni-sys` binding. Its Kotlin bridge exposes exactly `abiVersion(): Int` and `decode(input, expectedCanId, output, result): Int`. Keeping a separate library makes BMS symbols, dependencies, build outputs, and future evolution independently auditable.

Adding methods to framing, identity, or custom-configuration libraries was rejected because it would make those bridges own unrelated protocol behavior and couple their allowlists. Calling Apple's C ABI from Android was rejected because it would blur platform representation and packaging. A general cross-platform runtime was rejected because it recreates the failed big-bang migration shape.

### Cross one bounded caller-owned record

`decode` accepts a nullable Java byte-array input, an expected-CAN integer using `255` as the no-expectation sentinel, a nullable byte-array output, and an exact two-element integer result. Rust rejects null, aliased, oversized, or wrongly shaped arrays; expected-CAN values outside `0...253` and the sentinel; pending exceptions; and output capacities above the fixed maximum. It initializes result metadata, copies the bounded input, invokes the safe decoder, and copies a successful record only after all Rust work completes. `result[0]` is the produced or required record length and `result[1]` is the unsupported command byte.

ABI version 1 uses statuses `ok`, `invalid argument`, `unsupported command`, `truncated payload`, `output too small`, and `internal failure`. The successful version-1 record uses the same explicit logical representation already proven on Apple: an eight-byte version/flags/counts/reported-CAN/optional-tail header, exact big-endian IEEE-754 bits for 17 fixed scalars, counted cell-voltage doubles, one-byte balancing entries, counted temperature doubles, and bounded status UTF-8 bytes. The maximum record is 1,034 bytes and the maximum payload is 350 bytes. Rust allocations, references, layouts, strings, callbacks, handles, and diagnostics never cross JNI.

Duplicating the small host-specific record encoder is accepted because extracting it into the safe decoder would mix FFI representation with protocol ownership and changing Apple's verified bridge would broaden regression scope. Exact cross-host record vectors, bounds, and adapter assertions constrain the duplication.

### Retain `VescPayloadDecoder.bmsValues` as the production boundary

Add a thin `VescBmsValuesDecoder` beneath Android `protocol/vesc`; the existing `VescPayloadDecoder.bmsValues` delegates to it exactly once. The adapter allocates fixed bounded arrays, verifies the linked ABI, validates status and result metadata, then validates record version, flags, reserved byte, counts, exact checked extent, finite double bits, boolean entries, reported-CAN range, UTF-8, and optional-tail consistency before constructing `BmsSnapshot` and attaching `observedAt`.

Unsupported-command status maps to `UnsupportedCommand`; every structural, ABI, caller-contract, impossible-record, or internal bridge outcome maps to `TruncatedPayload`, matching established BMS behavior. The generic Kotlin byte reader and its auto-float implementation remain only where controller-current-limit decoding still needs them. BMS-only remainder and terminated-UTF-8 helpers and all command-96 algorithms are deleted; no source or runtime fallback remains.

### Build and verify BMS independently

Use a dedicated pinned build helper and disjoint generated output directories for host, debug, and release BMS libraries. Gradle loads all focused host JNI artifacts for JVM tests and packages each library for every supported Android ABI. Root audits allowlist exactly two BMS JNI symbols, verify dependency depth, locked JNI version, ABI agreement, all four package architectures, no committed binaries or generated bindings, one selected Kotlin call site, deleted structural signatures, and Apple isolation.

Focused Rust and JVM tests cover committed legacy and extended records, zero and maximum counts, maximum status, every fixed field and collection, malformed payload classes, JNI null/shape/alias/bounds behavior, expected-CAN validation, statuses, metadata, record validation, panic containment, and payload-safe diagnostics. `just verify` remains the complete software gate. Existing `bms-direct-controller`, `bms-express`, and `bms-standalone` physical scenarios run on connected Android, iOS, and macOS. Only a missing location observation within its bounded dwell is deferred; every BLE, BMS, routing, polling, state, session, monotonic, and cleanup failure remains blocking.

## Risks / Trade-offs

- **[A fourth Android JNI library increases packaging surface]** → Keep a separate exact library/symbol allowlist and assert every bridge for all four ABIs in debug and release APKs.
- **[Host-specific record encoders could drift]** → Use the same fixed version-1 bytes and maximums as Apple, retain the safe decoder as the sole payload source, and cover exact records through Rust and JVM tests.
- **[A malformed native record could be trusted accidentally]** → Kotlin validates ABI, status, metadata, version, flags, counts, exact extent, finite values, booleans, UTF-8, CAN range, and optional-tail consistency before constructing a snapshot.
- **[JNI misuse or panic could cross the boundary]** → Validate references, array lengths, aliasing, primitives and pending exceptions, bound every copy, and contain panics into a fixed internal-failure status.
- **[Physical regression could be mistaken for location delay]** → Defer only absence of a location sample; BLE, BMS, route, polling, state, session, monotonic, and cleanup failures remain mandatory.

## Migration Plan

1. Add and audit the Android JNI crate/helper and focused raw-boundary tests without changing the Kotlin production call site.
2. Wire disjoint host/debug/release outputs into Gradle and prove clean four-ABI packaging.
3. Add the thin BMS adapter, select it once from `VescPayloadDecoder.bmsValues`, extend production-path tests, and delete the Kotlin command-96 algorithms.
4. Pass focused Rust/JNI/Kotlin checks, source/dependency/symbol audits, clean packaging, and `just verify`.
5. Run the three focused BMS scenarios on connected Android, iOS, and macOS against the ESP32 fixture.
6. Rerun the complete software gate after physical evidence, commit the implementation, sync canonical specs, and archive the change.

Rollback is one repository revert because no contract, database, persisted representation, or migration changes. Generated native artifacts remain disposable and untracked.

## Open Questions

None.
