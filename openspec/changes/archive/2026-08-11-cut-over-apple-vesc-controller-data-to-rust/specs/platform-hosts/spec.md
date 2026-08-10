## ADDED Requirements

### Requirement: Shared Apple Rust controller-data bridge ownership

The Apple backend SHALL expose the safe controller-data component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly two exported operations: version query and tagged decode. It SHALL use fixed operation, status, kind, flag, fault, and record values; caller-owned bounded input and output regions; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, reference, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit result kind, option flags, raw and classified fault, optional series-cell metadata, reserved bytes, exact big-endian IEEE-754 scalar values, and signed big-endian ERPM. Swift SHALL validate the complete record and mechanically map it into existing models without interpreting command-4 or command-14 payload structure. The hand-written Apple C module, shared bridge catalog, locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator without committing a generated binding or native archive.

#### Scenario: Exercise one bounded tagged decode call

- **WHEN** Swift supplies one valid decode operation, payload, and caller-owned result regions
- **THEN** Rust validates every operation and region, decodes the payload, writes only the exact bounded tagged record, returns one fixed status, and retains no pointer, allocation, or lifetime

#### Scenario: Map one successful controller-data record

- **WHEN** the bridge returns a valid realtime or current-limit success record
- **THEN** Swift validates its version, kind, flags, reserved and irrelevant fields, exact extent, finite numeric bits, fault classification, and optional-cell consistency before constructing one complete existing model

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives an invalid operation, null, oversized, overlapping, insufficient-capacity, malformed, ABI-incompatible, invalid-record, or internally failed input
- **THEN** it writes nothing outside supplied bounds, unwinds no panic across C, exposes no partial controller data, and logs no payload or decoded value

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from repository-pinned source
- **THEN** each product links the correct two-symbol bridge architecture and shared adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Preserve platform and Android isolation

- **WHEN** shared Apple production uses Rust controller-data decoding
- **THEN** CoreBluetooth, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain native and Android contains no Apple bridge artifact or dependency
