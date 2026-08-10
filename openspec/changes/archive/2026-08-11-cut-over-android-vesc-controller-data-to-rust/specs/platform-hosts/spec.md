## MODIFIED Requirements

### Requirement: Rust VESC controller-data candidate ownership

The focused Rust VESC controller-data component SHALL live at `backends/core/vesc-controller-data` and SHALL remain independent of framing, firmware identity, custom configuration, BMS decoding, Refloat protocols, host frameworks, CAN routing, compatibility catalogues, scheduling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL select it through separate focused bridges and thin adapters beneath their respective `protocol/vesc` paths after focused, packaging, canonical, and physical verification deletes each host's superseded native parsing.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces command-4 realtime or command-14 current-limit decoding after both host cutovers
- **THEN** the safe component and both host bridges are plainly identified under `backends/core`, iOS and macOS share the Apple adapter, and Android uses its own thin adapter beneath `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused controller-data C bridge, Android packages only its focused controller-data JNI bridge, and neither host links, calls, generates bindings for, or packages the other host's artifact

#### Scenario: Preserve VESC application ownership

- **WHEN** either host decodes controller data through Rust
- **THEN** request construction, firmware compatibility, direct/CAN selection, framing, scheduling, telemetry polling, fault response, and state publication remain with that host's existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** both controller-data bridges are built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Android Rust controller-data bridge ownership

The Android backend SHALL expose the safe controller-data component through one dedicated, versioned, stateless JNI ABI. The ABI SHALL contain exactly two exported operations: version query and tagged decode. It SHALL use fixed operation, status, kind, flag, fault, and record values; JVM-owned bounded byte and integer arrays; alias and pending-exception validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, returned pointer, reference, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit result kind, option flags, raw and classified fault, optional series-cell metadata, reserved bytes, exact big-endian IEEE-754 scalar values, and signed big-endian ERPM. Kotlin SHALL validate the complete record and mechanically map it into existing models without interpreting command-4 or command-14 payload structure. The shared bridge catalog, locked source-build helper, and Gradle wiring SHALL build the same bridge for host JVM tests and all four supported Android ABIs in debug and release without committing a generated binding or native library.

#### Scenario: Exercise one bounded JNI tagged decode

- **WHEN** Kotlin supplies one valid decode operation, payload, and JVM-owned result arrays
- **THEN** Rust validates every operation, reference, shape, alias, primitive, and bound, decodes the payload, writes only the exact bounded tagged record and metadata, returns one fixed status, and retains no reference, allocation, or lifetime

#### Scenario: Map one successful controller-data record

- **WHEN** the bridge returns a valid realtime or current-limit success record
- **THEN** Kotlin validates its version, kind, flags, reserved and irrelevant fields, exact extent, finite numeric bits, fault classification, and optional-cell consistency before constructing one complete existing model

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives an invalid operation, null, oversized, aliased, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, pending-exception, invalid-record, or internally failed input
- **THEN** it writes no partial controller data, unwinds no panic across JNI, retains no Java reference, and logs no payload or decoded value

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications from repository-pinned source
- **THEN** the source-owned two-symbol bridge is loaded for JVM tests and packaged for `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` without a generated binding, committed library, or host-only substitution

#### Scenario: Preserve platform and Apple isolation

- **WHEN** Android production uses Rust controller-data decoding
- **THEN** Bluetooth, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain native and Apple contains no Android JNI artifact or dependency
