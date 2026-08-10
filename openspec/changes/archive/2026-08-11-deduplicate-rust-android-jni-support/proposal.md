## Why

The six Android Rust bridge crates contain 3,334 lines and repeat the same panic containment, JNI environment lookup, exception checks, primitive-array bounds, alias rejection, and byte/integer publication. Centralizing those security-sensitive VM-boundary mechanics is the next narrow deduplication step after shared build tooling and Apple FFI support, and it makes later Rust protocol cutovers smaller without changing any JNI ABI or application behavior.

## What Changes

- Add one `backends/core/android-jni-support` crate over the already pinned `jni-sys` dependency for protocol-neutral panic containment, checked per-call environment access, exception detection, primitive-array length and pairwise-alias validation, bounded byte-array copying/publication, and integer-array publication.
- Rewire all six `-android-jni` crates to use the shared support API while retaining their explicit exported JNI symbols, ABI versions, statuses, argument shapes, record encoders, domain mappings, tests, library names, and Kotlin call sites.
- Remove direct `jni-sys` dependencies and superseded boundary helper implementations from the six consumers, and add audits/tests proving the support crate is the sole owner of raw JNI table calls while retaining no environment/reference and exporting no JNI symbol.
- Make every Android bridge Gradle task track the support manifest/source as an incremental input while preserving the shared allowlisted builder, package contents, four Android ABIs, and host-test libraries.
- Run focused JNI/packaging checks, the complete software gate, and ESP32 physical regressions on Android, iOS, and macOS; only bounded-time location sample absence remains deferred.
- Non-goals: generating JNI exports, adopting the higher-level `jni` crate, `cbindgen`, bindgen, UniFFI, macros, changing Apple FFI support, merging bridge crates/libraries, moving protocol logic, or changing BLE, background, location, files, theme, SQLite, loopback, frontend, or application behavior.

## Capabilities

### New Capabilities

- `rust-android-jni-support`: Defines the shared protocol-neutral Android JNI safety support and unchanged ownership/ABI requirements for its six consumers.

### Modified Capabilities

- `rust-host-bridge-build-tooling`: Requires shared Android JNI support source to participate in every Android bridge task's incremental build inputs without changing artifacts or symbols.
- `native-core-migration`: Defines the narrow shared JNI-support boundary while preserving domain bridge, Kotlin host, and OS-capability ownership.
- `backend-conformance`: Adds focused software and all-host physical regression evidence for the Android-only boundary refactor.

## Impact

- Affected code: one new Rust support crate, six Android JNI crates, Android Gradle inputs, core/Android documentation, workspace dependency and unsafe-ownership audits, and conformance requirements.
- Removed code: repeated bridge-local environment, exception, reference-alias, primitive-array copy/publication, and panic-containment helpers plus duplicate primitive-only tests.
- JNI exported names, ABI/status values, shared-library names, Kotlin adapters, Android package contents, Apple archives/bridges, frontend contract, persistence, and runtime behavior remain unchanged.
- No new external version, generated source, database migration, or cross-platform API is introduced; the existing locked `jni-sys = 0.4.1` dependency moves behind the support crate.
