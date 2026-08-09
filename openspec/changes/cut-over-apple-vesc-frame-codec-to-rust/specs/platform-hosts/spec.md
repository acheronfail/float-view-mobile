## ADDED Requirements

### Requirement: Shared Apple Rust frame codec bridge ownership

The shared Apple backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through a thin hand-written Swift adapter and one Apple-specific C ABI crate under `backends/core`. The safe framing crate SHALL remain independent of Apple and unsafe code; the bridge crate SHALL contain only bounded representation conversion and SHALL depend only on the safe framing crate and the Rust standard library.

The bridge SHALL use fixed integer statuses, primitive pointer/length arguments, caller-owned bounded output buffers, panic containment, an ABI version check, and an explicit application-symbol allowlist. Rust allocator ownership, Rust layouts, callbacks, opaque runtime handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, and UI composition SHALL remain with the existing Apple owners.

#### Scenario: Locate Apple framing ownership

- **WHEN** a maintainer follows the Apple `protocol/vesc` codec adapter
- **THEN** the adapter identifies Rust as the production CRC/encode/exact-decode owner and Swift as the retained stream-boundary owner

#### Scenario: Share the cutover across Apple hosts

- **WHEN** iOS or macOS compiles the shared `FloatViewBackend`
- **THEN** both sibling hosts link the same versioned frame-codec ABI and use the same Swift adapter

#### Scenario: Cross one bounded buffer

- **WHEN** Swift encodes a payload or decodes an exact frame through the bridge
- **THEN** it supplies explicit bounded input and caller-owned output regions and receives a fixed status without transferring allocator ownership

#### Scenario: Contain a bridge failure

- **WHEN** the bridge receives a null, oversized, malformed, insufficient-capacity, or internally failed call
- **THEN** it writes nothing outside the supplied bound, unwinds no panic across C, returns the documented status, and logs no address, token, payload, or full frame

#### Scenario: Preserve OS capability ownership

- **WHEN** Apple selects the Rust stateless codec
- **THEN** CoreBluetooth and all other OS-owned capabilities continue through their existing shared or sibling-host adapters without a Rust callback or lifecycle handle
