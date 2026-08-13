## ADDED Requirements

### Requirement: Android Rust Refloat dynamic telemetry bridge ownership

The Android backend SHALL expose the safe Refloat dynamic-telemetry component through one temporary, focused, versioned, stateless JNI bridge. The bridge SHALL export exactly an ABI-version query, layout decode, and realtime decode; use bounded JVM-owned primitive arrays, explicit array extents, checked null/exception/reference/shape/distinctness/capacity, fixed statuses, deterministic output initialization, and panic containment; and depend only on the safe telemetry and protocol-neutral Android JNI-support crates.

Canonical layout and sample records SHALL be owned, parsed, and serialized only by the safe telemetry component. Kotlin SHALL validate and mechanically map them without interpreting command-31/32 bytes. Locked source builds and typed Gradle registration SHALL build the bridge for the host JVM and all four Android ABIs in Debug and Release without generated bindings or committed native binaries. The Apple builder and package catalogue SHALL retain their existing bridge and reject Android-only artifacts.

#### Scenario: Exercise bounded layout decode

- **WHEN** Android passes a valid command-32 payload and distinct JVM-owned output/result arrays
- **THEN** the bridge publishes exactly the safe component’s canonical bounded layout record and required length while retaining no JVM reference

#### Scenario: Exercise bounded realtime decode

- **WHEN** Android passes a valid canonical layout record, command-31 payload, and distinct JVM-owned output/result arrays
- **THEN** the bridge publishes exactly the safe component’s complete canonical sample record and required length while retaining no reference, allocation, or lifetime

#### Scenario: Validate every Android record form

- **WHEN** Kotlin receives a layout or sample record
- **THEN** it accepts only exact version, kind, extent, counts, UTF-8, ordering, closed codes, flags, options, finite values, canonical absent fields, alert/fault relations, extension entries, reserved bytes, and complete cursor consumption

#### Scenario: Contain bridge and record failure

- **WHEN** the bridge or adapter receives null, pending-exception, oversized, aliased, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, impossible, stale-layout, or internally failed input
- **THEN** it fails closed through the established status/error without partial publication, payload data, retained reference, generated fallback code, or native payload parsing

#### Scenario: Package every Android target

- **WHEN** host JVM, Debug, and Release products build from a clean artifact state
- **THEN** the host and each of `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` link the exact three-symbol bridge from source with no committed library or generated binding

#### Scenario: Delete superseded Kotlin ownership

- **WHEN** production selection and regression evidence pass
- **THEN** Android source contains no command-31/32 envelope reader, identifier parser, layout bound/uniqueness algorithm, extent calculator, source float16 decoder, semantic/alert/fault mapping, extension classifier, or runtime fallback

#### Scenario: Keep Apple and OS capabilities isolated

- **WHEN** the Android bridge is built and selected
- **THEN** Apple retains its selected C bridge while Bluetooth, location, background execution, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing owners

#### Scenario: Mark the bridges as migration scaffolding

- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** both platform bridges are documented as temporary selection boundaries whose protocol-specific symbols disappear after all hosts consume Rust-owned application behavior
