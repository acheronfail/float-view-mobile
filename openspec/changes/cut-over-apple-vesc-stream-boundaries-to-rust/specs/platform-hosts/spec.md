## MODIFIED Requirements

### Requirement: Shared Apple Rust frame codec bridge ownership

The shared Apple backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through thin hand-written Swift adapters and one Apple-specific C ABI crate under `backends/core`. Rust SHALL own CRC, encode, exact decode, and every VESC stream boundary decision. Swift SHALL own only bounded caller-side byte storage, mechanical consumption using Rust-returned lengths, and established Swift error mapping. The safe framing crate SHALL remain independent of Apple and unsafe code; the bridge crate SHALL contain only bounded representation conversion and SHALL depend only on the safe framing crate and the Rust standard library.

The versioned bridge SHALL use fixed integer statuses, primitive pointer/length arguments, caller-owned bounded regions, panic containment, and an explicit application-symbol allowlist. Its stream scanner SHALL return only the fixed outcome and validated primitive frame/consumption lengths for borrowed input; complete-frame payload output SHALL continue through the exact-decoder operation. Rust allocator ownership, Rust layouts, callbacks, opaque runtime handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, and UI composition SHALL remain with the existing Apple owners.

#### Scenario: Locate Apple framing ownership

- **WHEN** a maintainer follows the Apple `protocol/vesc` codec and stream adapters
- **THEN** the adapters identify Rust as the production CRC, encode, exact-decode, and stream-boundary owner while Swift contains only caller-owned bounded byte storage

#### Scenario: Share the cutover across Apple hosts

- **WHEN** iOS or macOS compiles the shared `FloatViewBackend`
- **THEN** both sibling hosts link the same versioned frame-codec ABI and use the same Swift adapters

#### Scenario: Scan one borrowed stream region

- **WHEN** Swift asks Rust to inspect its current bounded stream bytes
- **THEN** Rust reports incomplete input, a complete candidate frame, an oversized declaration, or aggregate overflow together with only validated primitive consumption and frame lengths and retains no pointer after the call

#### Scenario: Decode one scanned frame

- **WHEN** the scanner reports a complete candidate frame
- **THEN** Swift mechanically selects the reported region, Rust exact decode decides its validity and payload, and Swift consumes the reported prefix even when exact decoding returns the established malformed-frame error

#### Scenario: Contain a bridge failure

- **WHEN** the bridge receives a null, oversized, overlapping, misaligned, malformed, insufficient-capacity, or internally failed call
- **THEN** it writes nothing outside the supplied bounds, unwinds no panic across C, returns the documented fixed status, and logs no address, token, payload, or full frame

#### Scenario: Reset stream storage

- **WHEN** connection generation reset discards Apple stream state
- **THEN** Swift clears its caller-owned bytes without invoking a Rust destructor or retaining a cross-generation Rust handle

#### Scenario: Preserve OS capability ownership

- **WHEN** Apple selects Rust stream scanning
- **THEN** CoreBluetooth and all other OS-owned capabilities continue through their existing shared or sibling-host adapters without a Rust callback or lifecycle handle
