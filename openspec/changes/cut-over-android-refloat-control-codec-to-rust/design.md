## Context

`backends/core/refloat-control` is a safe, dependency-free codec for package, flywheel, legacy/stable lighting, zero-mask lighting queries, fatal clear, and lighting-response structural decoding. Apple now selects it through a focused three-operation C ABI. Android still selects `protocol/refloat/RefloatControlCodec.kt`, which duplicates the same command, parameter, mask, flag, query, availability, and response-shape decisions despite already sharing models, fixtures, conformance, and higher-level mutation behavior.

Android already builds four independent Rust JNI bridges through the shared catalogue and generic locked builder. Those bridges establish the required pattern: one platform-specific crate, the locked minimal `jni-sys` binding, host libraries for JVM tests, source-built libraries for four Android ABIs, exact symbol audits, caller-owned primitive arrays, pending-exception checks, and panic containment. The new control bridge can reuse this machinery without changing Apple or introducing generated bindings.

## Goals / Non-Goals

**Goals:**

- Make the safe Rust component the sole Android production owner of Refloat live-control payload encoding and lighting-response structural decoding.
- Keep Kotlin limited to loading/versioning the JNI bridge, converting existing models to fixed primitives, managing bounded caller-owned arrays, validating returned metadata, mapping fixed statuses to established exceptions, and constructing the existing lighting state.
- Build and package the bridge from locked source for host JVM tests and all four supported Android ABIs in debug and release.
- Delete the superseded Kotlin wire algorithm and enforce one Rust production path with focused and repository-wide audits.
- Verify unchanged direct/CAN/background behavior on Android and unchanged Apple behavior on iOS/macOS with the ESP32 fixture.

**Non-Goals:**

- Moving Refloat compatibility lookup, direct/CAN route selection, VESC framing, mutation authorization, scheduling, confirmation, telemetry, application state, SQLite, loopback transport, BLE, background work, location, lifecycle, files, theme, permissions, WebViews, or frontend behavior.
- Changing the safe codec, Apple C bridge, Apple adapter, frontend contract, persisted formats, exports, or database schema beyond changes strictly required to keep common tests compiling.
- Sharing an Apple artifact with Android, merging controls into another JNI library, generating bindings, returning Rust-owned memory, introducing callbacks/handles, or creating a general native runtime.

## Decisions

### Use one dedicated three-operation Android JNI crate

Add `refloat-control-android-jni` beside the safe component and Apple bridge. It depends only on `float-view-refloat-control`, the Rust standard library, and the already locked `jni-sys` crate. It exports exactly `abiVersion`, `encode`, and `decodeLighting` for `RustRefloatControlBridge`.

`encode` receives fixed operation, lighting-profile, fatal-availability, target, enabled, and current-flags integers plus a nine-byte-or-smaller caller-owned output and an exact two-integer result. The result contains produced/required length and unsupported-command byte. `decodeLighting` receives one bounded copied response, selected profile, and an exact one-integer result containing validated two-bit lighting flags. Statuses remain `ok`, `invalid argument`, `invalid payload`, `unsupported command`, `output too small`, and `internal failure`, matching the Apple bridge and safe component semantics.

Adding operations to an existing JNI library was rejected because it would couple unrelated protocol ownership, packages, and symbol allowlists. Calling the Apple C ABI from Android was rejected because JNI reference validation and Android packaging are distinct platform representation concerns. A shared FFI-record crate was rejected for this step because changing the verified Apple bridge would broaden regression scope and mix host representation into the safe protocol component.

### Keep JNI work mechanical and bounded

The bridge validates the JNI environment, pending exceptions, nulls, exact result shapes, pairwise reference aliasing, array lengths, integer-to-byte ranges, booleans, canonical unused values, and maximum response/output extents before copying. It initializes result metadata only after references are validated, copies successful payload bytes only after safe Rust completes, contains panics, retains no Java reference, and emits no payload/state/address/token diagnostic.

The safe crate remains the sole owner of VESC command/package prefix, Refloat commands and parameters, inverted package semantics, flywheel defaults and footpad-abort byte, lighting profiles/reserved bytes/masks/flags/query behavior, fatal-clear availability, response extent/prefix/defined bits, and semantic failures. The JNI crate owns only primitive and memory-contract validation.

### Keep the existing Kotlin codec surface as the production boundary

Retain `RefloatBooleanControl`, `RefloatLightingState`, and `RefloatControlCodec` at their current package and call sites. Replace its byte construction and parsing with primitive conversion, ABI/status/result validation, bounded array copying, and established `VescProtocolException` mapping. Package/flywheel bridge failures fail closed like other Kotlin exceptions; lighting and fatal-clear preserve their established failure classes. No fallback codec remains.

`RefloatWire` remains because non-control Refloat telemetry and identity code still uses it. Compatibility, service routing, coordinator authorization, pending mutation ownership, confirmation, telemetry, and state publication remain unchanged. Moving those layers in this change was rejected because each is a separately testable ownership transfer.

### Reuse shared build tooling and prove host isolation

Fill the Android metadata for the existing `refloat-control` catalogue entry and use `build-android-bridge.sh` for host, debug, and release outputs. Gradle adds disjoint Refloat-control output directories, host test properties, task dependencies, and generated JNI source directories while retaining its one application module. Root verification audits the exact three JNI symbols, ABI version, two-node production dependency graph, locked JNI dependency, all four architectures, no generated/committed artifacts, one Kotlin bridge path, deleted Kotlin algorithms, and unchanged Apple bridge selection.

After focused tests and `just verify`, run `control-direct`, `control-can`, and background on Android; the same applicable scenarios on iOS; and direct/CAN on macOS. Only absence of a location sample within the bounded dwell is deferred. BLE, routing, framing, control, confirmation, state, background/session, no-motor-movement, and cleanup failures remain blocking.

## Risks / Trade-offs

- **The JNI bridge adds more representation code than the deleted Kotlin algorithm.** → Keep exactly three operations, reuse the shared catalogue/builder, and constrain the bridge to mechanical JNI validation while the safe crate owns all protocol decisions.
- **Apple and Android bridge constants could drift.** → Use the same version, operations, statuses, primitive meanings, exact payload fixtures, and exhaustive state matrix, with independent exact-symbol and dependency audits.
- **Malformed arrays or primitive combinations could reach unsafe JNI operations.** → Validate environment, exceptions, nulls, shapes, aliasing, bounds, canonical unused values, and conversions before any copy or result write; contain every panic.
- **An impossible native result could be trusted by Kotlin.** → Validate ABI, status, result shape, produced/required length, unsupported byte, lighting flags, and exact bounds before returning an existing model.
- **A physical failure could be misclassified as slow location.** → Defer only the missing timed location observation; retain every BLE, route, control, state, session, monotonic, no-motor-movement, and cleanup assertion.

## Migration Plan

1. Add the Android JNI crate, catalogue metadata, focused boundary tests, symbol/dependency audits, and source-build wiring without selecting it from Kotlin.
2. Wire host JVM and four-ABI debug/release Gradle packaging, add the thin Kotlin bridge/adapter, and exercise exact fixtures and failures through JNI.
3. Delete the Kotlin control-wire and lighting-response algorithms, prove one production path and Apple isolation, and pass focused checks plus `just verify`.
4. Run direct/CAN/background physical regression on Android, iOS, and macOS as applicable against the ESP32 fixture.
5. Commit the verified implementation, sync the delta specs, and archive the change.

Rollback is a source revert of the coherent implementation commit. No contract, persistence, database, or runtime migration needs reversal, and generated native artifacts remain disposable.

## Open Questions

None. Broader Refloat application behavior, persistence, and loopback-server ownership remain separate later changes.
