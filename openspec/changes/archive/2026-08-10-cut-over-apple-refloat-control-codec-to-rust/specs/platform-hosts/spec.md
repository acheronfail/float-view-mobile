## ADDED Requirements

### Requirement: Shared Apple Rust Refloat control bridge ownership

The Apple backend SHALL expose the safe Refloat control component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly three exported operations: version query, control encode, and lighting-response decode. It SHALL use fixed operation, profile, target, status, and primitive-result values; caller-owned bounded output and scalar regions; borrowed bounded response input; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, reference, or opaque lifetime crossing the boundary.

The hand-written `CVescRefloatControl` module, shared bridge catalogue, generic locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator. No generated binding or native archive SHALL be committed. Existing Apple bridges SHALL remain independent, and Android SHALL neither link nor package this Apple bridge.

#### Scenario: Exercise one bounded encode call

- **WHEN** Swift supplies one canonical control operation and caller-owned output regions
- **THEN** Rust validates every primitive/region, encodes at most nine bytes, returns fixed status/length/unsupported-command values, and retains no pointer or allocation

#### Scenario: Exercise one bounded lighting decode call

- **WHEN** Swift supplies one exact lighting response and selected profile
- **THEN** Rust returns one validated two-bit primitive result or fixed failure without exposing a Rust model or retaining input

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives an invalid operation, profile, target, boolean, current state, unused parameter, null, oversized, overlapping, misaligned, insufficient-capacity, malformed, ABI-incompatible, or internally failed call
- **THEN** it writes nothing outside validated regions, unwinds no panic across C, returns the documented bounded result, and logs no payload, state, address, or token

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from repository-pinned source
- **THEN** each product links the correct three-symbol bridge architecture and shared adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Preserve platform and Android isolation

- **WHEN** shared Apple production uses Rust Refloat control encoding and decoding
- **THEN** CoreBluetooth, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain native and Android contains no Apple bridge artifact or dependency
