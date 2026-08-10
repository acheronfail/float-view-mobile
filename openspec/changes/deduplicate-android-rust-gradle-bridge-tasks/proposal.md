## Why

Android Gradle still repeats the same Rust bridge command, source-input set, output declaration, host-library resolution, and clean ordering for six bridge families and eighteen tasks. That makes adding or changing one Rust bridge a broad copy/edit operation even though the locked shell catalog and builders already define the build boundary once.

## What Changes

- Replace the six bridge-specific Android package task functions and six hand-written host tasks with one typed, closed Gradle bridge catalog plus generic Android-package and host-JNI task factories.
- Preserve every existing Gradle task name, task group, profile, bridge identifier, source input, output path, host system property, packaged library, ABI, clean ordering, and downstream dependency.
- Drive JNI source-set registration, JVM-test dependencies/properties, and Debug/Release merge dependencies from the same typed entries so a later bridge needs one Gradle catalog entry instead of several synchronized blocks.
- Add structural audits that prove the catalog has exactly six unique entries and eighteen exact tasks while rejecting reintroduced bridge-specific task factories and duplicated per-bridge input blocks.
- Keep the shell bridge catalog/builders, Rust crates and ABIs, Apple/Xcode integration, Kotlin production behavior, native capabilities, and frontend/backend contract unchanged.

Non-goals are generating Gradle from the shell catalog, parsing shell as data, changing public task names, combining native libraries, changing JNI exports, modifying Apple build phases, moving application behavior to Rust, or adding a Gradle plugin/module/dependency.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rust-host-bridge-build-tooling`: Require one closed typed Android Gradle bridge catalog and generic task registration while retaining exact build identities and regression evidence.

## Impact

The implementation is limited primarily to `backends/android/app/build.gradle.kts`, the root build-tooling audit, and Android/core documentation. It deletes repeated Gradle Kotlin DSL without changing application APIs, persistence, native permissions, BLE/background/location/file/theme ownership, Cargo dependencies, shell helpers, package contents, or Apple integration. Verification covers Gradle task/input/output structure, linked host JNI tests, clean four-ABI Debug/Release packaging, the complete software gate, and physical Android/iOS/macOS fixture regression; only bounded-time location sample absence may be deferred.
