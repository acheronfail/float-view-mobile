## Context

The safe dependency-free `backends/core/vesc-frame` crate now owns every Apple production framing decision through a versioned C ABI, but Android still implements CRC, frame construction, exact decoding, and stream boundary parsing in `VescFrame.kt`. The Rust behavior is already covered by immutable vectors, adversarial component tests, Apple software/packaging gates, and a physical iOS fragmented-background checkpoint. The unknown in the smallest Android cutover is therefore JNI, host-JVM loading, NDK cross-compilation, Gradle packaging, and production selection rather than framing semantics.

Android local unit tests use `VescFrameCodec` throughout the protocol and application suites, so a production-only `.so` is insufficient: the same bridge must build for the host JVM as well as Android. The application currently has one Gradle `app` module, minSdk 29, no native build, no ABI filters, and no generated API layer. The installed toolchain includes the four Android Rust targets and a usable NDK, but committed source must remain the authority and native outputs must stay ignored.

## Goals / Non-Goals

**Goals:**

- Make Rust the sole Android production owner of VESC CRC, frame encoding, and exact frame decoding while preserving Kotlin APIs and errors.
- Establish one small auditable JNI boundary and deterministic host-JVM plus Android-ABI source builds without adding another Gradle module.
- Keep the safe framing crate free of Android, JNI, unsafe code, and third-party dependencies.
- Exercise all existing Android codec consumers through the linked Rust implementation, then delete Kotlin CRC/encode/exact-decode code after complete evidence.
- Keep Kotlin stream accumulation and boundary interpretation unchanged and visibly owned for the next OpenSpec change.

**Non-Goals:**

- Rust stream scanning, Rust-owned stream state, opaque native handles, callbacks, or a generic cross-language runtime.
- Firmware identity, VESC command routing, BMS, Refloat, application behavior, SQLite, loopback transport, or frontend-contract changes.
- Apple ABI, Apple packaging, or Apple framing behavior changes.
- Moving Bluetooth, background work, location, permissions, file presentation, theme, lifecycle, or UI integration out of Android.
- Generated JNI bindings, bindgen, UniFFI, cargo-ndk, a CMake project, committed `.so`/`.dylib` files, or a native Gradle module.

## Decisions

### Cut over only the stateless Android codec

This change selects Rust for `encode(payload)` and `decode_exact(frame)` only. `VescFrameStreamDecoder` continues to own its Kotlin byte collection, noise disposal, header/declared-length parsing, incomplete suffix, complete-frame extraction, malformed progress, multiple-frame loop, and 8,192-byte aggregate bound. Each complete candidate still calls the public Kotlin `VescFrameCodec.decode`, whose implementation becomes the Rust adapter.

This mirrors the proven first Apple cutover and separates Android toolchain/bridge failures from stream-scanner integration. Cutting over encode, exact decode, and stream scanning together was rejected because one physical or lifecycle regression would not identify whether packaging, JNI conversion, or boundary-consumption behavior failed. Moving firmware identity first was rejected because it would deepen Apple-only protocol ownership without establishing the Android Rust path required by every later shared component.

### Use one Android-only Rust JNI bridge crate

Add `backends/core/vesc-frame-android-jni` as a `cdylib` workspace member. It depends on the safe framing crate and directly on pinned `jni-sys` 0.4.1 only; the latter contributes JNI C type/table definitions plus its macro crate and replaces handwritten platform-dependent JNI table layout. The safe crate remains dependency-free and forbids unsafe code. All JNI pointer/table access, array conversion, and exported functions remain isolated in the bridge crate.

The higher-level `jni` crate was rejected because its string, object, error, logging, and invocation dependencies are unnecessary for three primitive-array calls. A C/CMake shim was rejected because it would add a second native toolchain path or force a risky generalization/duplication of the Apple C bridge. Manually reproducing `JNINativeInterface` layout was rejected as a larger and less auditable unsafe contract. No JNI dependency may leak into the safe crate or future protocol APIs without another explicit review.

### Keep a flat versioned caller-owned contract

The private Kotlin bridge exposes exactly three native operations:

- `abiVersion(): Int`
- `encode(input: ByteArray?, output: ByteArray?, outputLength: IntArray?): Int`
- `decodeExact(input: ByteArray?, output: ByteArray?, outputLength: IntArray?): Int`

The nullable raw signatures make null-contract testing possible; the public Kotlin codec always supplies non-null arrays. Kotlin allocates the bounded output array and a one-element length array. The bridge validates nullability, exact length-array shape, input/output identity, array lengths, product bounds, and pending JNI exceptions before copying input into a call-local bounded Rust buffer. It invokes only safe `vesc-frame` operations, writes no partial payload/frame, returns the produced or required length according to the documented outcome, and retains no JNI reference or allocation after return.

Statuses reuse the Apple semantic numbering for success, invalid argument, input too large, invalid frame, invalid CRC, output too small, and contained internal failure, but the Android JNI ABI has its own version starting at 1. Kotlin maps semantic failures onto the established `PayloadTooLarge`, `InvalidFrame`, and `InvalidCrc` exceptions; impossible caller/ABI/internal failures fail closed as `InvalidFrame` with a bounded byte-free diagnostic. Every exported operation contains unwinding. The library exports exactly the three approved application-prefixed JNI symbols; no `JNI_OnLoad`, dynamic registration, callback, object construction, or returned native pointer is needed.

Returning a newly allocated byte array or throwing protocol exceptions from Rust was rejected because it would put object construction and exception-class lookup in the bridge. JVM-owned output arrays preserve the caller-owned pattern already proven by Apple and keep error mapping in Kotlin.

### Build host and Android libraries from one locked helper

A repository helper owns Rust target, NDK linker, Android ABI, API level, build-type, artifact name, and output-directory mapping. Gradle pins the selected NDK and adds narrowly scoped tasks that call this helper before local unit tests and before native-library merge/package work. Local tests build the current Cargo host `cdylib`, receive its exact absolute path through a private test system property, and load it through the same Kotlin bridge. Applications use `System.loadLibrary` and package generated libraries from an ignored directory beneath `backends/core/target/android`.

Android Debug and Release builds produce libraries for `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64`, matching the application's current unrestricted ABI surface. The helper uses the pinned Rust toolchain, lockfile, selected NDK clang linkers, minSdk 29, `--locked`, and offline Cargo resolution after normal repository setup. It never downloads Rust targets, an NDK, or dependencies during Gradle execution and never consumes a stale library from another target/configuration. Root `just` build, test, verify, conformance, run, and development paths continue to drive Gradle and therefore inherit the same prerequisites.

Adding cargo-ndk or a Gradle Rust plugin was rejected because direct target/linker mapping is small, testable, and avoids another tool/plugin lifecycle. Restricting the application to arm64 was rejected because it would silently narrow the existing package contract. Committed prebuilt libraries and an AAR were rejected because source, lockfile, and target selection must remain authoritative.

### Preserve the Kotlin surface and gate deletion

`VescProtocolException` and public `VescFrameCodec` constants/signatures remain stable. The Kotlin implementation becomes allocation, ABI check, native-call, exact-length slicing, and status mapping only. `VescCrc` and the Kotlin encode/exact-decode algorithms may temporarily remain only as private test-target parity references while the production adapter has no feature flag or fallback.

Focused bridge tests cover raw null/alias/shape/capacity failures, status/length initialization, contained panic, source vectors, and no retained JNI state. Host-JVM tests cover both native functions through the actual JVM, while the existing Android unit suite proves every codec consumer uses Rust. Repository audits check the exact JNI symbol set, dependency isolation, no tracked native artifacts, APK ABI/library contents, and no production Kotlin algorithm. Clean Debug/Release builds, canonical Android conformance, and the focused physical Android fragmented ride/background scenario must pass before the temporary Kotlin references are deleted. The audits are then tightened to reject their return and `just verify` runs again. Apple verification remains part of the unchanged complete gate.

## Risks / Trade-offs

- **Raw JNI expands the unsafe surface.** → Isolate it in one bridge crate, use only `jni-sys` table definitions, keep three primitive-array calls, validate before access, contain panics, and exercise the real host JVM plus Android runtime.
- **Host JVM and Android ART can differ.** → Build the same source for both, run all local tests through the host library, verify canonical Android execution, and require a real-device checkpoint before deletion.
- **Gradle can package stale or wrong-ABI libraries.** → Use target/configuration-specific ignored outputs, explicit task inputs/outputs and dependencies, clean packaging tests, APK inspection, ABI version checks, and load failure with no fallback.
- **One JNI dependency weakens the dependency-free workspace story.** → Pin and lock only `jni-sys` in the Android bridge; keep the safe algorithm crate and Apple bridge unchanged; audit the direct/transitive graph and forbid unused higher-level JNI crates.
- **Building four ABIs increases build time.** → Build the host library only for unit tests, retain Gradle incremental task outputs, and reserve clean four-ABI Debug/Release builds for packaging/verification while preserving the existing package surface.
- **Kotlin stream parsing remains duplicated temporarily.** → Keep its ownership explicit, retain immutable parity evidence, and schedule its deletion as the immediately following independent OpenSpec change.
- **The physical checkpoint can delay completion.** → Keep the temporary reference only in tests and do not claim or archive the ownership transfer until the physical gate and final deletion rerun pass.

## Migration Plan

1. Add the locked JNI dependency, bridge crate, internal status/result logic, host-JVM harness coverage, Android target helper, and symbol/dependency audits without changing production Kotlin selection.
2. Wire Gradle local tests and Debug/Release packaging to source-built host and four-ABI libraries; prove clean load/link/package behavior and no committed artifacts.
3. Replace production `VescFrameCodec` internals with the no-fallback JNI adapter, quarantine the old algorithms as test-only parity references, and run focused Rust/JNI/Kotlin plus canonical software gates.
4. Run the focused physical Android fragmented-notification ride with background/foreground recovery through production Bluetooth and the selected Rust codec.
5. Delete `VescCrc`, Kotlin encode/exact-decode bodies, and temporary references; tighten source/ABI/package audits; rerun clean packaging, canonical Android/unchanged Apple checks, and `just verify`.

Rollback before final deletion is a source revert to Kotlin `VescFrameCodec`; no contract, storage, database, or user-data migration is involved. After deletion, rollback requires reverting this complete OpenSpec implementation rather than enabling a dormant fallback.

## Open Questions

None. Android stream scanning is intentionally the next independent change; broader protocol and runtime boundaries remain deferred.
