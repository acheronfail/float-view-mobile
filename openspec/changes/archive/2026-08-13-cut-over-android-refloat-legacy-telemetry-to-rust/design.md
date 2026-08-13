## Context

`refloat-legacy-telemetry` is a dependency-free, unsafe-free Rust component that already owns complete command-10 decoding and the canonical version-1 telemetry record. Apple production consumes that record through a selected two-symbol C bridge, while Android still performs the same length checks, byte reads, scaling, semantic-state mapping, and fault classification in `RefloatLegacy.kt`. Existing Android protocol components are selected through independent primitive-array JNI bridges built from locked source by one typed Gradle catalogue.

The cutover must preserve the existing `RefloatLegacyTelemetryDecoder.decode` API and `RefloatTelemetryRecord`, keep Apple unchanged, package offline for the host JVM and every Android ABI, expose no Rust allocation or retained JVM reference, and leave compatibility, polling, routing, state, persistence, transport, and OS capabilities in Kotlin. The bridge is transitional and will be deleted when the complete application runtime moves to Rust.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of fixed Refloat command-10 structural and semantic decoding.
- Reuse the exact canonical record already consumed by Apple without adding another serializer or changing its version.
- Add one temporary two-symbol Android JNI bridge with fixed statuses, JVM-owned primitive arrays, bounded call-local copies, and panic containment.
- Preserve normalized telemetry models, established errors, and all higher-level Android behavior while deleting superseded Kotlin wire algorithms.
- Prove exact records, adversarial JNI boundaries, host/four-ABI packaging, Apple isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Changing the safe decoder, canonical record layout, or selected Apple C ABI and adapter.
- Moving dynamic Refloat telemetry, compatibility, request construction, direct/CAN routing, framing, scheduling, polling, cache merging, application behavior, SQLite, loopback transport, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract.
- Introducing the final Rust application runtime, callbacks, opaque handles, generated bindings, macros, committed native binaries, or a new dependency.
- Generalizing or merging existing per-protocol bridge crates; all such bridges remain temporary migration scaffolding.

## Decisions

### Publish the existing safe record through an independent JNI bridge

Add `refloat-legacy-telemetry-android-jni` with exactly an ABI-version query and decode operation. Decode accepts one bounded JVM byte-array payload, one JVM byte-array record output, and a fixed two-integer result array carrying record length and unsupported command. It maps fixed success, invalid-argument, unsupported-envelope, truncated-payload, invalid-payload, insufficient-output, and contained-internal-failure statuses. `android-jni-support` owns mechanical environment, exception, reference, array-length, distinctness, copying, publication, and panic safety; the safe telemetry crate owns every protocol and record decision.

Adding command-10 to another JNI library was rejected because it couples unrelated release surfaces. Re-encoding the record in the JNI crate was rejected because the safe component is already its canonical owner. Returning JVM objects, JSON, direct buffers, or generated bindings was rejected because primitive arrays keep the temporary boundary explicit, bounded, and consistent with existing host tests.

### Keep Kotlin mapping strict and mechanical

`RefloatLegacyTelemetryDecoder` becomes a bridge-backed adapter. It validates the linked ABI, exact record extent, version, result kind, closed enum codes, flags, option/detail consistency, finite numeric values, canonical absent fields, reserved and irrelevant bytes, and complete cursor consumption before constructing the existing `RefloatTelemetryRecord`. Ride state and the active-fault alert are derived only from the already-classified record kind and package state, not from command-10 bytes.

After selection, the Kotlin input-length table, signed/unsigned readers, float decoding, state/mode/footpad/stop/setpoint/alert/fault interpretation, scales, and field-weakening logic are deleted. Representation switches over stable record codes remain permitted and are covered by malformed-record rejection tests. A runtime Kotlin fallback is forbidden because it would retain duplicate production ownership.

### Extend the typed Android bridge catalogue by one entry

Give the existing `refloat-legacy-telemetry` build-catalog entry exact Android package, library, JNI prefix, and two-symbol metadata. Add a seventh typed Gradle bridge specification whose generated Debug/Release outputs are consumed by the matching merge tasks and whose host artifact is passed to JVM tests through one explicit property. Dependency inputs include the safe crate, JNI crate, shared JNI support, workspace lock/toolchain, and shared build metadata.

Apple's package, archive, C symbols, module, SwiftPM/Xcode inputs, and production call remain unchanged. Creating shared platform code above the safe record was rejected because the two VM boundaries have different safety and publication rules and will ultimately be removed.

### Require deletion and physical selection evidence

Focused Rust tests cover status mapping, exact safe records, null/exception/array/capacity/distinctness failures, output initialization, and contained panics. Host JVM tests cover ABI loading, all detail levels, active faults, every option and sentinel behavior, established error mapping, malformed record rejection, and production selection. Source and symbol audits prove that only the Rust crate interprets command-10 bytes and that the Android library exports exactly two bridge symbols.

After clean Android host and four-ABI Debug/Release packaging plus `just verify`, run applicable identity, direct/CAN control, BMS, ride, unexpected-disconnect, and mobile background/session scenarios on Android, iOS, and macOS against the ESP32 fixture. Android exercises the newly selected JNI path; Apple proves isolation through its unchanged selected Rust path. Only absence of a location sample within its bounded dwell is deferred.

## Risks / Trade-offs

- **[Risk] Kotlin record parsing can accept impossible combinations.** → Mirror the closed Apple validation rules, require exact extents and canonical absent values, and exhaust malformed records in JVM tests.
- **[Risk] A seventh JNI artifact can drift from the typed Gradle and shell catalogues.** → Derive every task from one typed entry and audit exact task names, inputs, outputs, package, symbols, and merge consumers.
- **[Risk] The cutover may accidentally change higher-level telemetry behavior.** → Preserve the decoder API and model, retain routing/polling/application owners, run canonical software conformance, and exercise production BLE on all three hosts.
- **[Trade-off] Another bridge temporarily increases source and packaging surface.** → Keep exactly two symbols, reuse audited support/build tooling and the canonical record, delete the larger Kotlin wire parser, and document removal with the unified Rust runtime.

## Migration Plan

1. Add the Android JNI crate, exact catalogue/Gradle metadata, focused boundary tests, and audits without selecting it from Kotlin.
2. Add the strict Kotlin record adapter and host-JVM tests, then select it and delete the superseded Kotlin wire algorithms and fallback surface.
3. Prove host and four-ABI packaging, Apple isolation, strict OpenSpec validation, and the complete `just verify` gate.
4. Run focused ESP32 regression on connected Android, iOS, and macOS, deferring only bounded location-sample absence.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no data migration, persisted-format change, dual runtime path, or remote deployment.

## Open Questions

None. The next protocol candidate and the later unified Rust application/runtime boundary remain separate changes.
