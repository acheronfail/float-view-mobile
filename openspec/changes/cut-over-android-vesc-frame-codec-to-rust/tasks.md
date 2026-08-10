## 1. Build the bounded Android JNI bridge

- [ ] 1.1 Add the `vesc-frame-android-jni` `cdylib` workspace member with only the safe framing crate and pinned `jni-sys` dependency, update the lockfile and ownership documentation, and prove the safe crate and Apple bridge dependency graphs remain unchanged.
- [ ] 1.2 Define the Android JNI ABI version, shared fixed statuses, bounded internal encode/exact-decode operations, output-length rules, and contained-panic mapping independently of JNI array access.
- [ ] 1.3 Implement exactly the version, encode, and exact-decode JNI exports with pre-access null, array-shape, product-bound, identity/alias, pending-exception, capacity, and JNI-table validation; retain no JNI reference or Rust allocation after return.
- [ ] 1.4 Add Rust bridge tests for immutable short/long/empty/maximum vectors, every semantic status, required/produced lengths, unchanged output on failure, invalid caller shapes, contained panic, dependency isolation, and the exact application-symbol allowlist.

## 2. Add deterministic host and Android packaging

- [ ] 2.1 Add one source-build helper that maps host-JVM and Android `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` targets, pinned NDK/API linkers, Debug/Release profiles, and target-specific ignored outputs while using the pinned toolchain, lockfile, and offline resolution.
- [ ] 2.2 Pin the selected NDK and add Gradle tasks/inputs/outputs so local JVM tests build and load the exact host library while Android merge/package tasks build the exact four-ABI libraries in the existing `app` module.
- [ ] 2.3 Add clean packaging and negative-prerequisite checks that reject missing targets/NDK/linkers, stale or mismatched ABI versions, unexpected symbols, wrong architectures, duplicate libraries, and tracked native artifacts without narrowing the current ABI surface.
- [ ] 2.4 Integrate the Android Rust build, host-JVM load path, package inspection, dependency audit, and focused recipes into root `just` build/test/verify/run/development paths and backend documentation without changing Apple build selection.

## 3. Cut the Kotlin stateless codec over

- [ ] 3.1 Add the private raw JNI declarations and no-fallback Kotlin adapter that owns bounded JVM output arrays, verifies ABI version, slices only validated produced lengths, and maps fixed statuses onto the existing `VescProtocolException` cases.
- [ ] 3.2 Replace `VescFrameCodec` encode/decode internals with the adapter while preserving its public constants/signatures, every production call site, and `VescFrameStreamDecoder` storage, header/length/noise, fragmentation, recovery, multiple-frame, reset, and overflow behavior.
- [ ] 3.3 Extend host-JVM tests across native null/alias/shape/capacity/version failures, short/long/empty/maximum frames, malformed header/length/footer/CRC/trailing bytes, stable errors, and retained Kotlin stream fragmentation/noise/multiple/recovery behavior through Rust exact decode.
- [ ] 3.4 Run the complete Android local unit suite through the generated host JNI library and retain any Kotlin CRC/encode/exact-decode comparison only as a private test-target checkpoint that is absent from production packaging and source selection.
- [ ] 3.5 Update core and Android ownership documentation to identify Rust as Android's CRC/encode/exact-decode owner, Kotlin as the temporary stream-boundary owner, Apple as unchanged, and the Android stream cutover as the next independent change.

## 4. Verify the additive production cutover

- [ ] 4.1 Run Rust formatting, warnings-denied linting and tests, bridge dependency/status/symbol audits, linked host-JVM codec/stream tests, Android unit tests, and source/artifact ownership audits with the Rust adapter selected and no production fallback.
- [ ] 4.2 Run clean host-JVM, Android Debug/Release four-ABI packaging and APK inspection, canonical Android build/launch/conformance, and unchanged Apple packaging/canonical checks while proving every selected artifact has the current ABI version.
- [ ] 4.3 Run `just verify` with the additive Android stateless cutover selected and resolve every software, canonical, packaging, dependency, and ownership regression before the deletion checkpoint.

## 5. Complete the physical deletion checkpoint

- [ ] 5.1 Run and record the focused physical Android production-Bluetooth ride with fixture-supplied fragmented notifications and mobile background/foreground recovery, proving Rust encode/exact decode preserves the same session and continued samples.
- [ ] 5.2 Delete `VescCrc`, Kotlin frame construction/exact decoding, and any temporary test reference, then tighten repository audits so only the thin codec adapter and retained Kotlin stream parser remain and the deleted algorithms cannot return.
- [ ] 5.3 Rerun focused Rust/JNI/Kotlin checks, exact dependency/symbol/source audits, clean host/four-ABI packaging, canonical Android execution, and unchanged Apple packaging/canonical verification after deletion.
- [ ] 5.4 Run the final `just verify` software gate and resolve all regressions without changing the frontend contract, stream-boundary behavior, database, stored data, OS-capability ownership, or user-visible behavior.
