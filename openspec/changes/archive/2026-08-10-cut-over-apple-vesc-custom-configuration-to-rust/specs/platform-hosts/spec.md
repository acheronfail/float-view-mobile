## MODIFIED Requirements

### Requirement: Rust custom-configuration candidate ownership

The focused Rust VESC custom-configuration component SHALL live at `backends/core/vesc-custom-configuration` and SHALL remain independent of framing and identity bridges, host frameworks, schema/XML and typed-value codecs, Bluetooth, routing, scheduling, application coordination, persistence, transport, and frontend code. Apple SHALL select the component through its focused custom-configuration bridge and shared codec beneath `backends/apple/protocol/vesc` only after focused, packaging, canonical, and physical verification deletes the superseded Swift implementation. Android SHALL continue to locate its selected Kotlin production codec beneath `backends/android/protocol/vesc` until a separate Android-specific cutover passes equivalent verification and deletion gates.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces commands 92–95 after the Apple cutover
- **THEN** the safe component and focused Apple bridge are plainly identified under `backends/core`, shared iOS/macOS selection is beneath Apple's `protocol/vesc` path, and Android's Kotlin owner remains beneath Android's `protocol/vesc` path

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** Apple has only the focused audited C bridge while Android gains no C symbol, JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path from this cutover

#### Scenario: Preserve higher-level configuration ownership

- **WHEN** the Rust component encodes or decodes opaque configuration envelopes for Apple production
- **THEN** schema assembly, decompression, XML, signatures, typed values, safety policy, mutation coordination, scheduling, routing, storage, and contract serialization remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the Apple custom-configuration bridge is built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Shared Apple Rust custom-configuration bridge ownership

The Apple backend SHALL expose the safe custom-configuration component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly three exported operations: version query, request encode, and response decode. It SHALL use fixed operation, status, and result-record values; caller-owned bounded input and output regions; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, callback, or opaque lifetime crossing the boundary.

The hand-written Apple C module, locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator. No generated binding or native archive SHALL be committed. The existing Apple framing and firmware-identity bridges SHALL remain independent, and Android SHALL neither link nor package this Apple bridge.

#### Scenario: Exercise the bridge contract

- **WHEN** compiled Rust and C checks call every operation with valid and invalid buffers, capacities, regions, operations, payloads, and result records
- **THEN** the exact three-symbol ABI returns only fixed bounded statuses and records, contains panics, and transfers no allocation or lifetime ownership

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from the repository-pinned source
- **THEN** each product links the correct bridge architecture and shared Swift adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Keep bridges independent

- **WHEN** maintainers inspect dependencies and exported symbols
- **THEN** custom-configuration depends only on its safe component and standard library and neither changes nor expands framing or firmware-identity bridge ownership

#### Scenario: Keep platform services native

- **WHEN** shared Apple production uses Rust command envelopes
- **THEN** CoreBluetooth, background execution, location, native files, theme, lifecycle, permissions, WebViews, application services, SQLite, and loopback transport remain owned by the Apple host

#### Scenario: Isolate Android

- **WHEN** Android unit, packaging, canonical, and physical checks run during this Apple change
- **THEN** they continue through the unchanged Kotlin codec without resolving an Apple C module, library, or symbol
