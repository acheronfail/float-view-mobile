## Context

The safe `application-runtime` is selected by Apple through one coarse C ABI. Its Apple binding currently also contains platform-neutral JSON command/effect translation, handle registry, token mapping, interruption, and pending-result state. Android selects the same underlying Rust domain components through ten stateless protocol JNI libraries but keeps application, session, and protocol coordination in Kotlin. A sound second-host cutover must share the envelope implementation rather than copying 1,800 lines into a JNI crate, and it must delete the now-temporary protocol bridge/build matrix.

The measured Apple-cutover state is 31,724 Rust, 11,329 Apple, and 16,227 Android production-source lines, 34 workspace members, eleven bridge crates, one Apple artifact family, ten Android bridge crates, and eighty Android target artifacts. The final state must have two coarse host bindings, zero protocol bridge crates/artifacts, fewer workspace members, and fewer than 59,280 combined production-source lines.

## Goals / Non-Goals

**Goals:**

- Select the same process-owned application runtime on Android without copying its command/effect mapping.
- Preserve effect order, generation/token safety, persistence-before-publication, reconnect/reset behavior, contract envelopes, SQLite compatibility, and disposable-WebView semantics.
- Reduce Android to one JNI runtime library per ABI/profile and delete every protocol-specific bridge and native decision fallback.
- Simplify the final build matrix and make source/artifact regression limits permanent repository gates.
- Pass the complete software gate and repeat physical iOS/ESP32 conformance after changing shared boundary code.

**Non-Goals:**

- Moving Android Bluetooth, Handler/executor implementation, location acquisition, permissions, lifecycle/foreground service, file presentation, theme/UI, loopback transport/authentication, catalogue metadata, or import/export presentation into Rust.
- Changing the frontend contract, SQL schema, stored-session compatibility, or adding generated bindings/callbacks.
- Running Android physical conformance in this change.

## Decisions

### Share the safe host envelope, not an Apple implementation

Move strict JSON decoding/encoding, runtime-handle registry, token translation, interruption, result limits, and pending-result semantics into a safe `application-runtime-host` crate. It exposes slice-based Rust methods and contains no raw C/JNI pointer, JVM, callback, OS framework, or platform object. `application-runtime-apple-ffi` becomes a thin pointer-validation C ABI; a new `application-runtime-android-jni` becomes a thin call-local primitive-array JNI ABI. Both retain explicit hand-written bindings and the same ABI version/status meanings.

This avoids depending Android on an Apple-named crate or duplicating the evolving envelope. Putting JSON directly in the safe application domain would mix host representation with application behavior, while generated bindings remain outside repository policy.

### Android mirrors the proven effect-executor shape

`RustApplicationRuntime` owns the JNI handle and strict two-pass caller-owned result buffers. `ApplicationService` submits platform facts as closed commands, validates returned version/generation/effects, executes BLE/timer/cache/activation/catalogue/location/publication work in order, and projects retained public Kotlin records. Rust never retains JVM references or calls Kotlin. Handler tokens remain a generation-qualified Kotlin map and are reported back to Rust on firing.

### Session files are Rust-owned; catalogue and presentation stay native

The Android catalogue allocates identity/path and reconciles Rust commit markers. Active canonical session writes, bounded reads/reductions, interruption, and prune selection/commit run through the unified runtime. Kotlin retains catalogue/search metadata, archive validation, import/export packaging, and native share/picker presentation, but deletes duplicated session SQL/reduction/prune decisions.

### Deletion completes the migration

The cutover deletes Kotlin equivalents of the Apple-owned application algorithms, selected VESC/Refloat protocol decisions, their private tests, all ten `Rust*Bridge.kt` classes, all ten `*-android-jni` crates, and `android-jni-support`. Gradle retains one closed runtime build task factory for host, Debug, and Release outputs; the shell bridge catalog and `build-android-bridge.sh` are removed. Root/core audits reject every retired path, symbol, task, property, and artifact.

### Verification respects the requested hardware scope

Focused Rust tests exercise the shared host boundary plus both thin ABIs. JVM tests exercise handle/envelope/result validation and Android effect mapping. Clean four-ABI Debug/Release packaging must contain exactly one runtime library per ABI and no protocol libraries. `just verify` covers both software backends. Because shared boundary code changes after the Apple checkpoint, the connected iOS device reruns the canonical ESP32 matrix. Android device testing is skipped and recorded as such.

## Risks / Trade-offs

- **[Risk]** Extracting the live Apple envelope could regress a proven host. **Mitigation:** preserve byte/status vectors, keep the C ABI stable, run Apple package/tests/software conformance, then rerun the iOS physical matrix.
- **[Risk]** JNI arrays and handles can be malformed or concurrent. **Mitigation:** call-local checked copies, fixed bounds, no retained references, registry IDs, per-handle try-lock rejection, panic containment, and adversarial host-JNI tests.
- **[Risk]** A large Kotlin deletion can hide missing host work. **Mitigation:** map every Rust effect to an explicit exhaustive executor branch, forbid fallback imports/files, and use canonical conformance as the behavioral authority.
- **[Risk]** Skipping Android hardware leaves platform-BLE risk. **Mitigation:** Android BLE code remains unchanged, clean app/JVM/software conformance runs, and the omission is explicit rather than presented as physical proof.

## Migration Plan

1. Extract and test the safe host-envelope crate while preserving the Apple C ABI.
2. Add and exhaustively test one Android JNI runtime adapter and clean four-ABI packaging.
3. Add the Kotlin runtime/effect adapter, select it in the process service, and move session operations.
4. Delete superseded Kotlin decisions/tests plus all ten protocol JNI paths and support.
5. Remove the bridge catalog/build matrix, update docs/audits, and record final convergence counts.
6. Run strict OpenSpec validation, `just verify`, and connected iOS/ESP32 physical conformance; commit, sync, and archive with Android physical explicitly skipped.

Rollback before completion is `git revert` of the implementation commit. No feature flag or production fallback ships.

## Open Questions

None.
