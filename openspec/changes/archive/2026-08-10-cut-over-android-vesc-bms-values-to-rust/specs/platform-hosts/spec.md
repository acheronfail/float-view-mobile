## MODIFIED Requirements

### Requirement: Rust VESC BMS values candidate ownership

The focused Rust BMS-values component SHALL live at `backends/core/vesc-bms-values` and SHALL remain independent of framing, firmware identity and custom-configuration bridges, host frameworks, CAN routing, compatibility, discovery, scheduling, polling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL select it through separate focused bridges and thin adapters beneath their respective `protocol/vesc` trees after verification deletes each superseded native structural parser.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces command-96 BMS-values decoding after both host cutovers
- **THEN** the safe component and both host bridges are plainly identified under `backends/core`, iOS and macOS share the Apple adapter, and Android uses its own thin adapter beneath `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused BMS C bridge, Android packages only its focused BMS JNI bridge, and neither host links, calls, generates bindings for, or packages the other host's artifact

#### Scenario: Preserve BMS application ownership

- **WHEN** either host decodes command-96 payloads through Rust
- **THEN** compatibility, discovery, direct/CAN selection, request scheduling, polling, observed-time assignment, availability, state publication, and live-only policy remain with that host's existing native owners

#### Scenario: Preserve OS capability ownership

- **WHEN** both BMS-values bridges are built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Android Rust BMS values bridge ownership

The Android backend SHALL expose the safe BMS-values component through one dedicated, versioned, stateless JNI ABI. The ABI SHALL contain exactly two exported operations: version query and decode. It SHALL use fixed status and record values; JVM-owned bounded byte and integer arrays; explicit expected-CAN representation and lengths; alias and pending-exception validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, returned pointer, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit flags, counts, reported CAN ID, optional-tail metadata, exact big-endian IEEE-754 scalar and collection values, one-byte balancing entries, and bounded status bytes. Kotlin SHALL validate the complete record and mechanically map it into `BmsSnapshot` without interpreting command-96 payload structure. The locked source-build helper and Gradle wiring SHALL build the same bridge for host JVM tests and all four supported Android ABIs in debug and release without committing a generated binding or native library.

#### Scenario: Exercise one bounded JNI call

- **WHEN** Kotlin supplies one payload, optional expected CAN ID, and JVM-owned result arrays
- **THEN** Rust validates every reference, shape, alias, primitive and bound, decodes the payload, writes only the documented bounded record and metadata, returns one fixed status, and retains no reference, allocation, or lifetime

#### Scenario: Map one successful record

- **WHEN** the bridge returns a valid success record
- **THEN** Kotlin validates its version, flags, reserved byte, counts, extents, finite numeric bits, boolean values, reported CAN ID, and optional-tail consistency before constructing one complete snapshot

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, aliased, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, pending-exception, invalid-record, or internally failed input
- **THEN** it writes no partial snapshot, unwinds no panic across JNI, retains no Java reference, and logs no payload or decoded value

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications from repository-pinned source
- **THEN** the source-owned bridge is loaded for JVM tests and packaged for `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` without a generated binding, committed library, or host-only substitution

#### Scenario: Keep bridges and Apple independent

- **WHEN** maintainers inspect dependencies, libraries, exported symbols, and Apple packages
- **THEN** the Android BMS bridge depends only on its safe component, standard library, and locked minimal JNI binding; existing Android bridges retain their own symbols; and Apple contains no Android BMS JNI artifact or dependency
