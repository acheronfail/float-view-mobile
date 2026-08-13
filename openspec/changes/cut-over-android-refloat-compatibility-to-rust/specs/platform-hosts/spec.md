## ADDED Requirements

### Requirement: Android Rust Refloat compatibility ownership

The Android protocol layer SHALL select the safe Rust compatibility component through one temporary stateless Android JNI bridge. The Kotlin adapter SHALL contain only bounded canonical-record validation and mechanical model mapping. Android SHALL retain controller-discovery transitions, request scheduling, direct/CAN route ownership, connection generations, state/session/control coordination, and all OS capabilities. Apple SHALL retain its selected compatibility implementation and unchanged C bridge build surface.

#### Scenario: Locate the selected Android owner

- **WHEN** a maintainer traces Android INFO identity, supported-release/profile selection, or dynamic-layout admission
- **THEN** each protocol decision terminates in `backends/core/refloat-compatibility` through the narrow Android bridge while higher-level discovery effects remain under the Android application owner

#### Scenario: Exercise the bounded JNI contract

- **WHEN** Android requests INFO bytes, decodes INFO, or admits a canonical layout through JVM-owned primitive arrays
- **THEN** the exact four-symbol bridge validates environment, exception, reference, shape, distinctness, and capacity bounds, publishes fixed statuses and canonical outputs, contains panics, and retains no JVM reference or Rust allocation

#### Scenario: Delete superseded Kotlin ownership

- **WHEN** Android production selection and regression evidence pass
- **THEN** Android source contains no INFO version-offset decoder, release catalogue, profile derivation, required identifier catalogue, set-based layout admission, or runtime fallback

#### Scenario: Preserve Android packaging from locked source

- **WHEN** host JVM, Android Debug, and Android Release products build from a clean artifact state
- **THEN** the host and each of `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` link the exact compatibility bridge symbols from locked repository source with no committed library or generated binding

#### Scenario: Keep Apple and OS capabilities isolated

- **WHEN** the Android bridge is built and selected
- **THEN** Apple retains its existing compatibility C archive and adapter while Bluetooth, background work, location, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing platform owners

#### Scenario: Mark both bridges as migration scaffolding

- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** the Android JNI and Apple FFI bridges are documented as temporary selection boundaries whose compatibility symbols disappear after all hosts consume Rust-owned application behavior
