## Context

`backends/core/vesc-custom-configuration` is a safe, dependency-free, bounded codec for VESC commands 92–95. Apple already selects it through a focused three-operation C ABI. Android still selects `VescCustomConfigurationCodec.kt`, whose 66 lines duplicate command construction, signed big-endian extents, schema-chunk arithmetic, values-envelope validation, and acknowledgement parsing even though its public request/response surface already matches the Rust component.

Android already has a proven source-built Rust JNI pattern for framing and firmware identity: platform-specific bridge crates using the locked minimal `jni-sys` binding, pinned offline helpers, four Android ABI packages, explicit host libraries for JVM tests, caller-owned primitive arrays, exact symbol audits, and panic containment. This change can reuse that pattern while keeping the safe component and Apple's bridge unchanged.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of command-92 schema requests/chunks, command-93 current requests/values, command-94 default requests/values, and command-95 writes/acknowledgements.
- Keep Kotlin limited to loading the bridge, validating fixed primitive metadata and caller-owned opaque ranges, constructing the existing response variants, and mapping fixed statuses to existing errors.
- Build and package the focused bridge from locked source for host JVM tests and all four supported Android ABIs in debug and release.
- Prove fixture and adversarial parity through the compiled JNI boundary, then prove the selected production path through direct/CAN physical configuration operations on Android while rerunning Apple regression.
- Delete the superseded Kotlin envelope algorithms and enforce their absence with repository audits.

**Non-Goals:**

- Moving schema chunk assembly, decompression, XML, signatures, typed Refloat values, compatibility or safety policy, mutation coordination, routing, scheduling, or application behavior.
- Moving SQLite, loopback transport, BLE, background work, location, lifecycle, files, theme, permissions, persistence, WebViews, or the frontend contract.
- Sharing the Apple bridge artifact with Android, modifying any Apple source or bridge, merging custom configuration into the existing framing or identity JNI library, or creating generated bindings, callbacks, Rust-owned handles, or a general native runtime.

## Decisions

### Use a separate three-operation Android JNI bridge

Add `vesc-custom-configuration-android-jni` beside the safe codec and existing host bridges. It depends only on the safe custom-configuration crate, the Rust standard library, and the already locked minimal `jni-sys` binding. Its Kotlin bridge exposes exactly an ABI-version query, request encode, and response decode operation. Keeping a separate library makes custom-configuration symbols, dependencies, build outputs, and future evolution independently auditable.

Adding methods to the framing or identity JNI libraries was rejected because it would make those bridges own unrelated protocol behavior and couple their allowlists. Calling Apple's C ABI from Android was rejected because it would blur platform representation and packaging. A general cross-platform runtime was rejected because it recreates the failed big-bang migration shape.

### Cross only caller-owned primitive arrays

Request encode accepts a fixed operation selector, index, length/offset primitives or bounded values input, a caller-owned byte output, and exact result metadata. Response decode accepts the bounded payload plus a fixed-size integer result record describing response kind, command, index, total, offset, and the validated opaque-byte offset/length. Kotlin copies opaque bytes only from its original caller-owned input after checked range validation.

The bridge validates nulls, array shapes and lengths, aliasing, signed primitive ranges, output capacity, pending exceptions, and all copy results before invoking or publishing a result. It contains panics into a fixed internal-failure status and retains no Java reference or Rust allocation. Rust layouts, strings, callbacks, native pointers, handles, and payload diagnostics do not cross JNI.

Returning decoded opaque bytes through another native output buffer was rejected because the bytes already remain in Kotlin-owned input during the call. Returning Rust allocations or objects was rejected because this codec is stateless and needs no lifetime boundary.

### Retain `VescCustomConfigurationCodec` as the thin production adapter

The Kotlin object keeps `INDEX`, `MAXIMUM_CHUNK_BYTES`, its request methods, response variants, and existing `VescProtocolException` behavior. Each request verifies the linked ABI and invokes Rust once. Decode validates the fixed result shape, known response kind and command, representable extents, and checked opaque range before constructing the existing response. Recognized unsupported commands map to `UnsupportedCommand`; ABI, caller-contract, malformed, impossible-result, capacity, or internal outcomes map to `TruncatedPayload`.

No Kotlin command byte construction, signed-integer encoder/decoder, schema arithmetic, values minimum check, acknowledgement parser, or fallback remains. Higher-level services therefore retain their public and ownership boundaries unchanged.

### Build and verify the bridge independently

Use a dedicated pinned helper and disjoint generated output directories for host, debug, and release custom-configuration libraries. Gradle loads the exact host artifact for JVM tests and packages the bridge alongside the existing independent framing and identity libraries for all four ABIs. Root audits allowlist exactly the three JNI symbols, verify dependency depth and locked `jni-sys` version, inspect all packaged ABIs, reject committed binaries/generated bindings, and reject deleted Kotlin algorithm signatures.

Focused Rust/JNI/Kotlin tests cover every Refloat 1.0.0–1.2.3 request/response fixture, integer and chunk boundaries, malformed/unsupported commands, null/shape/alias/capacity failures, pending exceptions, result corruption, status mapping, redaction, and panic containment. `just verify` remains the complete software gate.

### Reuse the established physical configuration checkpoint

The direct/CAN control scenario already opens schema, reads current/default values, writes the audited disposable `kp` mutation, verifies readback, and completes no-motor-movement controls. Run it on Android through the new Rust JNI path and on iOS/macOS through the unchanged Apple Rust bridge. This proves commands 92–95 across real BLE without expanding into broad ride or location coverage.

Only absence of a location observation within a bounded automated dwell may remain deferred manual coverage. BLE, permission/capability, monotonic boundary, configuration, routing, controls, no-motor-movement, session, and cleanup failures remain blocking.

## Risks / Trade-offs

- **[A third Android JNI library increases packaging surface]** → Keep separate exact library/symbol allowlists and assert all three libraries for every supported ABI in debug and release packages.
- **[JNI array misuse or panic could cross the boundary]** → Validate null, shape, alias, extent, pending-exception, and copy outcomes; bound all regions; initialize metadata; and contain panics into a fixed status.
- **[Malformed native result metadata could select an invalid Kotlin range]** → Require the exact result shape, validate every kind/command/extent, and use checked offset-plus-length against the original input before copying.
- **[Kotlin and Apple adapters could map the same safe result differently]** → Exercise both against the same immutable every-release fixtures and canonical physical sequence while keeping the safe component as the sole envelope decision owner.
- **[Physical writes alter fixture state]** → Use only the audited disposable fixture mutation and verified readback; never run the write checkpoint against an uncontrolled VESC.

## Migration Plan

1. Add and audit the Android JNI crate/helper and focused raw-boundary tests without changing the Kotlin production call site.
2. Wire disjoint host/debug/release outputs into Gradle and prove clean four-ABI packaging.
3. Convert `VescCustomConfigurationCodec` into the thin bridge adapter, extend production-path tests, and delete the Kotlin envelope algorithms.
4. Pass focused Rust/JNI/Kotlin checks, source/dependency/symbol audits, clean packaging, and `just verify`.
5. Run direct and CAN physical configuration/control checkpoints on connected Android, iOS, and macOS against the ESP32 fixture.
6. Rerun the complete software gate after physical evidence, commit the implementation, sync canonical specs, and archive the change.

Rollback is a repository revert because no contract, database, persisted representation, or migration changes. Generated native artifacts remain disposable and untracked.

## Open Questions

None.
