## MODIFIED Requirements

### Requirement: Rust custom-configuration candidate ownership

The focused Rust VESC custom-configuration component SHALL live at `backends/core/vesc-custom-configuration` and SHALL remain independent of framing and identity bridges, host frameworks, schema/XML and typed-value codecs, Bluetooth, routing, scheduling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL select the component through separate focused bridges and thin adapters beneath each host's `protocol/vesc` path only after focused, packaging, canonical, and physical verification deletes that host's superseded native implementation.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces commands 92–95 after both host cutovers
- **THEN** the safe component and separate focused host bridges are plainly identified under `backends/core`, while the thin shared-Apple and Android adapters remain beneath each backend's `protocol/vesc` path

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused C bridge, Android packages only its focused JNI bridge, and neither host links, calls, generates bindings for, or packages the other host's bridge artifact

#### Scenario: Preserve higher-level configuration ownership

- **WHEN** the Rust component encodes or decodes opaque configuration envelopes for host production
- **THEN** schema assembly, decompression, XML, signatures, typed values, safety policy, mutation coordination, scheduling, routing, storage, and contract serialization remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** both custom-configuration bridges are built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Android Rust custom-configuration bridge ownership

The Android backend SHALL expose the safe custom-configuration component beneath `protocol/vesc` through one dedicated, versioned, stateless JNI bridge. The JNI surface SHALL contain exactly three exported operations: version query, request encode, and response decode. It SHALL use fixed operation, status, and primitive result values; JVM-owned bounded byte and integer arrays; explicit lengths; alias and pending-exception checks; panic containment; payload-safe diagnostics; and no Rust allocator, callback, returned pointer, or opaque lifetime crossing the boundary.

The locked source-build helper and Gradle wiring SHALL build the same bridge for host JVM tests and all four supported Android ABIs in debug and release. No generated binding or native library SHALL be committed. The existing Android framing and firmware-identity bridges and all Apple bridges SHALL remain independent.

#### Scenario: Exercise the Android bridge contract

- **WHEN** focused Rust and JVM checks call every operation with valid and invalid arrays, shapes, aliases, capacities, operations, payloads, exceptions, and result metadata
- **THEN** the exact three-symbol JNI ABI returns only fixed bounded statuses and primitive records, contains panics, and transfers no allocation or lifetime ownership

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications
- **THEN** the source-owned bridge is loaded for JVM tests and packaged for arm64-v8a, armeabi-v7a, x86, and x86_64 without a generated binding, committed library, or host-only substitution

#### Scenario: Keep Android bridges independent

- **WHEN** maintainers inspect dependencies, libraries, and exported symbols
- **THEN** custom configuration depends only on its safe component, standard library, and locked minimal JNI binding and neither changes nor expands framing or firmware-identity bridge ownership

#### Scenario: Keep Android platform services native

- **WHEN** Android production uses Rust command envelopes
- **THEN** Bluetooth, background execution, location, native files, theme, lifecycle, permissions, WebViews, application services, SQLite, and loopback transport remain owned by Android

#### Scenario: Isolate Apple

- **WHEN** Apple unit, packaging, canonical, and physical checks run during the Android cutover
- **THEN** iOS and macOS continue through the unchanged Apple bridge without resolving an Android JNI library or symbol
