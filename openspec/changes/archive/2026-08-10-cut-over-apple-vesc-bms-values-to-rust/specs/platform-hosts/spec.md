## MODIFIED Requirements

### Requirement: Rust VESC BMS values candidate ownership

The focused Rust BMS values component SHALL live at `backends/core/vesc-bms-values` and SHALL remain independent of framing, firmware identity and custom-configuration bridges, host frameworks, CAN routing, compatibility, discovery, scheduling, polling, application coordination, persistence, transport, and frontend code. Apple SHALL select it through one focused bridge and shared adapter beneath Apple `protocol/vesc` after verification deletes the superseded Swift structural parser. Android SHALL continue to locate its selected production command-96 decoder beneath its own `protocol/vesc` tree until a separate host-specific cutover passes focused, packaging, canonical, and physical verification and deletes its Kotlin parser.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces command-96 BMS values decoding after the Apple cutover
- **THEN** the safe component and Apple bridge are plainly identified under `backends/core`, iOS and macOS share a thin adapter under Apple `protocol/vesc`, and Android's selected Kotlin owner remains under Android `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused BMS values bridge, Android gains no Apple artifact or BMS Rust call path, and neither host gains generated bindings, callbacks, or opaque handles

#### Scenario: Preserve BMS application ownership

- **WHEN** Apple decodes command-96 payloads through Rust
- **THEN** compatibility, discovery, direct/CAN selection, request scheduling, polling, observed-time assignment, availability, state publication, and live-only policy remain with their existing Apple owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the Apple BMS values bridge is built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Shared Apple Rust BMS values bridge ownership

The Apple backend SHALL expose the safe BMS values component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly two exported operations: version query and decode. It SHALL use fixed status and record values; caller-owned bounded input and output regions; explicit expected-CAN representation; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, reference, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit flags, counts, reported CAN ID, optional-tail metadata, exact big-endian IEEE-754 scalar and collection values, one-byte balancing entries, and bounded status bytes. Swift SHALL validate the complete record and mechanically map it into `BmsSnapshot` without interpreting command-96 payload structure. The hand-written Apple C module, locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator without committing a generated binding or native archive.

#### Scenario: Exercise one bounded bridge call

- **WHEN** Swift supplies one payload, optional expected CAN ID, and caller-owned result regions
- **THEN** Rust validates every region, decodes the payload, writes only the documented bounded record, returns one fixed status, and retains no pointer, allocation, or lifetime

#### Scenario: Map one successful record

- **WHEN** the bridge returns a valid success record
- **THEN** Swift validates its version, flags, reserved byte, counts, extents, finite numeric bits, boolean values, reported CAN ID, and optional-tail consistency before constructing one complete snapshot

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, overlapping, insufficient-capacity, malformed, ABI-incompatible, invalid-record, or internally failed input
- **THEN** it writes nothing outside supplied bounds, unwinds no panic across C, exposes no partial snapshot, and logs no payload or decoded value

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from repository-pinned source
- **THEN** each product links the correct two-symbol bridge architecture and shared adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Keep bridges and Android independent

- **WHEN** maintainers inspect dependencies, exported symbols, and Android packages
- **THEN** the BMS values bridge depends only on its safe component and standard library, existing Apple bridges retain their own symbols and behavior, and Android contains no Apple BMS bridge artifact or dependency
