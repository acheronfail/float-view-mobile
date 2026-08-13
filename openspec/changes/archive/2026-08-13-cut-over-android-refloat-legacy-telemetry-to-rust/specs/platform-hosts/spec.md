## ADDED Requirements

### Requirement: Shared Android Rust Refloat legacy telemetry bridge ownership

The Android backend SHALL expose the safe Refloat legacy-telemetry component through one temporary, focused, versioned, stateless JNI bridge. The bridge SHALL export exactly an ABI-version query and one decode operation; use bounded JVM primitive arrays and call-local copies; validate environment, exceptions, nulls, array extents, reference distinctness, and capacity; publish fixed statuses and metadata; contain panics; retain no JVM reference; and depend only on the safe telemetry and protocol-neutral Android JNI-support crates.

The canonical version-1 record SHALL remain owned and serialized only by the safe telemetry component. Kotlin SHALL validate and mechanically map the record without interpreting command-10 bytes. Locked source builds, the shared shell catalogue, typed Gradle registration, host tests, and Android packaging SHALL select the correct library for the host JVM and arm64-v8a, armeabi-v7a, x86, and x86_64 without generated bindings or committed native binaries.

#### Scenario: Exercise one bounded Android bridge call
- **WHEN** Android passes a valid bounded legacy telemetry payload and distinct JVM-owned output arrays
- **THEN** the bridge publishes exactly the safe component's complete record and required length with the fixed success status and retains no caller memory

#### Scenario: Validate every Kotlin record form
- **WHEN** Kotlin receives a sample or active-fault record
- **THEN** it accepts only the exact version, kind, extent, closed codes, flags, option/detail consistency, finite values, canonical absent fields, reserved bytes, and complete cursor required by that form

#### Scenario: Contain bridge and record failure
- **WHEN** the bridge or adapter receives a null environment or reference, pending exception, oversized or aliased array, insufficient capacity, malformed payload, ABI-incompatible or impossible record, or internal failure
- **THEN** it fails closed through the established status or error without partial publication, payload data, retained references, generated fallback code, or Kotlin payload parsing

#### Scenario: Package every Android target
- **WHEN** host JVM, Debug, and Release products build from a clean artifact state
- **THEN** the host and all four Android ABIs contain the exact two-symbol JNI library built from locked source and every downstream merge and test consumer depends on its registered task

#### Scenario: Delete superseded Kotlin ownership
- **WHEN** production selection and regression evidence pass
- **THEN** Android source contains no command-10 byte reader, length table, fixed-point or float decoder, semantic-state mapping, payload enum or fault classification, field-weakening logic, or runtime fallback

#### Scenario: Keep Apple and OS capabilities isolated
- **WHEN** the Android bridge is built and selected
- **THEN** Apple retains its existing C archive and adapter while Bluetooth, location, background execution, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing owners

#### Scenario: Mark both host bridges as migration scaffolding
- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** the Android JNI and Apple FFI bridges are documented as temporary selection boundaries whose protocol-specific symbols disappear after all hosts consume Rust-owned application behavior
