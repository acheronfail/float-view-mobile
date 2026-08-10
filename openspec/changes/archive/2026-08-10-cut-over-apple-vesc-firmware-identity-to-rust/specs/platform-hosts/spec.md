## MODIFIED Requirements

### Requirement: Rust firmware identity candidate ownership

The safe Rust firmware identity component SHALL live at `backends/core/vesc-firmware-identity` and SHALL remain independent of host bridges, framing, host frameworks, Bluetooth, application coordination, persistence, transport, and frontend code. The shared Apple backend SHALL select it through one focused Apple bridge beneath `protocol/vesc`; Android SHALL continue to locate its selected Kotlin firmware identity decoder beneath its own `protocol/vesc` tree until a separate Android cutover passes focused, packaging, canonical, and physical verification and deletes that superseded parser.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces VESC firmware identity decoding after the Apple cutover
- **THEN** the safe component and Apple bridge are plainly identified under `backends/core`, iOS and macOS share a thin adapter under Apple `protocol/vesc`, and Android's selected Kotlin owner remains under Android `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused identity bridge, Android links no identity Rust artifact, and neither host gains generated bindings, callbacks, opaque handles, or unrelated production calls

#### Scenario: Preserve OS capability ownership

- **WHEN** Apple selects Rust firmware identity decoding
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners

## ADDED Requirements

### Requirement: Shared Apple Rust firmware identity bridge ownership

The shared Apple backend SHALL expose production Rust firmware identity decoding through a thin hand-written Swift adapter and one Apple-specific bridge crate under `backends/core`. The bridge SHALL depend only on the safe identity component and standard library and SHALL use a hand-written C module, fixed version/status values, bounded borrowed input, a caller-owned versioned byte record, explicit lengths, panic containment, and an exact two-symbol application allowlist. Rust allocations, layouts, references, callbacks, opaque handles, generated bindings, payload logs, names, UUID diagnostics, and unrelated capabilities MUST NOT cross the boundary.

The bridge SHALL build from locked offline source for macOS, iOS device, and active iOS simulator targets and SHALL be linked by both sibling Apple hosts without committing a native archive. The existing framing bridge SHALL retain its separate crate, header, four-symbol allowlist, and ownership unchanged.

#### Scenario: Cross one bounded identity call

- **WHEN** Swift supplies one firmware payload and caller-owned result region
- **THEN** Rust validates the regions, decodes the identity, writes only the documented bounded record, returns a fixed status, and retains no pointer or allocation

#### Scenario: Map one successful record

- **WHEN** the bridge returns success
- **THEN** Swift validates the exact record version/flags/lengths and mechanically constructs `FirmwareInfo` without interpreting VESC payload structure

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, overlapping, insufficient-capacity, malformed, ABI-incompatible, or internally failed input
- **THEN** it writes nothing outside supplied bounds, unwinds no panic across C, returns the documented bounded result, and logs no payload or private identity

#### Scenario: Share Apple production selection

- **WHEN** iOS or macOS compiles the shared `FloatViewBackend`
- **THEN** both link the same identity bridge and select the same Swift adapter and safe Rust decoder

#### Scenario: Keep framing independent

- **WHEN** the identity bridge is built and audited
- **THEN** the existing framing bridge and safe framing component retain their current symbols, package graph, and production behavior

#### Scenario: Preserve Android isolation

- **WHEN** Gradle builds Android during or after the Apple cutover
- **THEN** no Apple identity archive, C module, build helper, or Rust identity dependency enters the Android application graph
