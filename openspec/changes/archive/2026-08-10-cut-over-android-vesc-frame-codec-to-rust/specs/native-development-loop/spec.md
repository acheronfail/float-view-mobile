## ADDED Requirements

### Requirement: Locked source-built Android Rust frame codec

The repository SHALL build the Android Rust frame-codec JNI library from the committed workspace and lockfile with the pinned Rust toolchain, locked dependency resolution, and the pinned Android NDK. It SHALL support the host JVM architecture used by Android local unit tests plus every Android ABI packaged by the application, without committing native libraries, generating bindings, adding a native Gradle module, or downloading toolchains during an Android build.

Root `just` and Gradle build, test, verification, run, and development paths that compile or execute the Android backend SHALL build the matching Rust target and configuration before Kotlin compilation, local JVM test execution, application packaging, or launch. Missing Rust targets, NDK/linker mismatches, stale or absent libraries, ABI-version drift, unexpected JNI symbols, unsupported architecture sets, and load/link failures SHALL stop with actionable diagnostics rather than selecting Kotlin behavior or an older artifact.

#### Scenario: Run Android local unit tests

- **WHEN** Gradle runs the Android host-JVM unit-test suite from a clean ignored-artifact state
- **THEN** it first builds and loads the matching host Rust JNI library and every Kotlin codec-dependent test executes the selected Rust encode and exact-decode path

#### Scenario: Build Android Debug and Release applications

- **WHEN** Gradle assembles either supported build type from a clean ignored-artifact state
- **THEN** the helper builds matching locked Rust libraries for every packaged Android ABI and the APK contains exactly one current codec library per ABI

#### Scenario: Missing native prerequisite

- **WHEN** the pinned Rust toolchain, selected Rust target, Android NDK, host JVM, or linker prerequisite is unavailable
- **THEN** the build fails before Kotlin execution or application packaging and reports the exact missing prerequisite without downloading it or falling back to Kotlin framing

#### Scenario: Audit Android native artifacts

- **WHEN** repository verification inspects tracked files, generated libraries, and packaged APK contents
- **THEN** no native binary or generated binding is tracked and the JNI library exports exactly its approved ABI-version, encode, and exact-decode application entry points for each architecture
