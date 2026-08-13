## Context

`refloat-dynamic-telemetry` is a dependency-free, unsafe-free Rust component that already owns complete command-32 layout decoding, command-31 realtime decoding, and canonical version-1 layout/sample records. Apple production consumes those records through a selected three-symbol C bridge, while Android still performs envelope checks, UTF-8 layout parsing, mask-dependent extent calculation, source float16 decoding, semantic mapping, alert/fault classification, and extension selection in `RefloatDynamic.kt`. Existing Android protocol components are selected through independent primitive-array JNI bridges built from locked source by one typed Gradle catalogue.

The cutover must preserve the existing `RefloatDynamicTelemetryDecoder` API, its reset/replacement behavior, and `RefloatTelemetryRecord`; keep Apple unchanged; package offline for the host JVM and every Android ABI; expose no Rust allocation or retained JVM reference; and leave compatibility, polling, routing, state, persistence, transport, and OS capabilities in Kotlin. The bridge is transitional and will be deleted when the complete application runtime moves to Rust.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of Refloat command-31/32 structural and semantic decoding.
- Reuse the exact canonical records already consumed by Apple without adding another serializer or changing their version.
- Add one temporary three-symbol Android JNI bridge with fixed statuses, JVM-owned primitive arrays, bounded call-local copies, and panic containment.
- Preserve connection-scoped layout lifetime, normalized telemetry models, established errors, and all higher-level Android behavior while deleting superseded Kotlin wire algorithms.
- Prove exact records, adversarial JNI boundaries, host/four-ABI packaging, Apple isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Changing the safe decoder, canonical record layouts, or selected Apple C ABI and adapter.
- Moving compatibility, request construction, direct/CAN routing, framing, scheduling, polling, cache merging, application behavior, SQLite, loopback transport, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract.
- Introducing the final Rust application runtime, callbacks, opaque handles, generated bindings, macros, committed native binaries, or a new dependency.
- Generalizing or merging existing per-protocol bridge crates; all such bridges remain temporary migration scaffolding.

## Decisions

### Publish the existing safe records through an independent JNI bridge

Add `refloat-dynamic-telemetry-android-jni` with exactly an ABI-version query, layout decode, and realtime decode operation. Layout decode accepts one bounded JVM byte-array payload, one JVM byte-array record output, and a fixed integer result array carrying required/produced length. Realtime decode additionally accepts the canonical layout record. Both map fixed success, invalid-argument, unsupported-envelope, truncated-payload, invalid-layout, invalid-telemetry, insufficient-output, and contained-internal-failure statuses. `android-jni-support` owns mechanical environment, exception, reference, array-length, distinctness, copying, publication, and panic safety; the safe telemetry crate owns every protocol and record decision.

Adding command-31/32 to another JNI library was rejected because it couples unrelated release surfaces. Re-encoding records in the JNI crate was rejected because the safe component is already their canonical owner. Returning JVM objects, JSON, direct buffers, or generated bindings was rejected because primitive arrays keep the temporary boundary explicit, bounded, and consistent with existing host tests.

### Retain the canonical layout record in the existing decoder lifetime

`RefloatDynamicTelemetryDecoder` retains the exact bounded canonical layout record alongside its mechanically decoded field lists. `reset` clears all three values; a successful replacement publishes all three atomically; a rejected replacement preserves the prior accepted layout just as the current Kotlin decoder does. Each realtime call supplies the retained record back to stateless Rust. No Rust handle, allocation, callback, or cross-call reference is introduced.

Making the JNI bridge stateful was rejected because it would duplicate connection-generation ownership and complicate reset, cancellation, and lifetime safety. Reconstructing a record from Kotlin field lists was rejected because it would create a second serializer and weaken record identity.

### Keep Kotlin record mapping strict and mechanical

The decoder validates linked ABI, exact record extents, version/kind, counts, UTF-8 names, ordering, uniqueness, closed semantic codes, known flags/options, reserved bytes, presence bits, canonical absent values, finite numbers, charging pairs, alert/fault relations, extension uniqueness, and complete cursor consumption before constructing existing models. After selection, Kotlin envelope/layout parsing, extent calculation, source float16 conversion, semantic mapping from raw command bytes, and extension classification are deleted. Closed representation switches over canonical record codes remain permitted and are covered by malformed-record rejection tests. A runtime Kotlin fallback is forbidden because it would retain duplicate production ownership.

### Extend the typed Android bridge catalogue by one entry

Complete the existing `refloat-dynamic-telemetry` build-catalog entry with exact Android package, library, JNI prefix, and three-symbol metadata. Add an eighth typed Gradle bridge specification whose generated Debug/Release outputs are consumed by matching merge tasks and whose host artifact is passed to JVM tests through one explicit property. Dependency inputs include the safe crate, JNI crate, shared JNI support, workspace lock/toolchain, and shared build metadata.

Apple's package, archive, C symbols, module, SwiftPM/Xcode inputs, and production call remain unchanged. Creating shared platform code above the safe records was rejected because the two VM boundaries have different safety and publication rules and will ultimately be removed.

### Require deletion and physical selection evidence

Focused Rust tests cover status mapping, exact safe records, null/exception/reference/array/capacity/distinctness failures, initialized outputs, and contained panics. Host JVM tests cover ABI loading, source-derived layouts/realtime forms, complete record mapping, reset/replacement, established error mapping, and adversarial record rejection. Source and symbol audits prove that only the Rust crate interprets command-31/32 bytes and that the Android library exports exactly three bridge symbols.

After clean Android host and four-ABI Debug/Release packaging plus `just verify`, run applicable identity, direct/CAN control, BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile background/session scenarios on Android, iOS, and macOS against the ESP32 fixture. Android exercises the newly selected JNI path; Apple proves isolation through its unchanged selected Rust path. Only absence of a location sample within its bounded dwell is deferred.

## Risks / Trade-offs

- **[Risk] Kotlin record parsing can accept impossible combinations.** → Mirror the closed Apple validation rules, require exact extents and canonical absent values, and exhaust malformed records in JVM tests.
- **[Risk] Layout state can become stale or partially replaced.** → Store only the exact Rust record, reset it with the existing connection generation, replace fields and record atomically after full validation, and test failed replacement plus reset.
- **[Risk] An eighth JNI artifact can drift from typed Gradle and shell catalogues.** → Derive every task from one typed entry and audit exact task names, inputs, outputs, package, symbols, and merge consumers.
- **[Risk] The cutover may accidentally change higher-level telemetry behavior.** → Preserve the decoder API and model, retain routing/polling/application owners, run canonical software conformance, and exercise production BLE on all three hosts.
- **[Trade-off] Another bridge temporarily increases source and packaging surface.** → Keep exactly three symbols, reuse audited support/build tooling and canonical records, delete the larger Kotlin wire parser, and document removal with the unified Rust runtime.

## Migration Plan

1. Add the Android JNI crate, exact catalogue/Gradle metadata, focused boundary tests, and audits without selecting it from Kotlin.
2. Add the strict Kotlin record adapter and host-JVM tests, then select it and delete the superseded Kotlin wire algorithms and fallback surface.
3. Prove host and four-ABI packaging, Apple isolation, strict OpenSpec validation, and the complete `just verify` gate.
4. Run focused ESP32 regression on connected Android, iOS, and macOS, deferring only bounded location-sample absence.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no data migration, persisted-format change, dual runtime path, or remote deployment.

## Open Questions

None. The next higher-level Rust ownership transfer and the later unified Rust application/runtime boundary remain separate changes.
