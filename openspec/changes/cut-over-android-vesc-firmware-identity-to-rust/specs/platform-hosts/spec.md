## ADDED Requirements

### Requirement: Android Rust firmware identity bridge ownership

The Android backend SHALL expose production Rust firmware identity decoding beneath `protocol/vesc` through a thin hand-written Kotlin adapter and one Android-specific JNI bridge crate under `backends/core`. The bridge SHALL depend only on the safe identity component, the Rust standard library, and the existing locked minimal JNI ABI binding. It SHALL use fixed version/status values, bounded copied input, caller-owned primitive arrays, a versioned byte record, explicit lengths, pending-exception checks, panic containment, and an exact two-symbol application allowlist.

The safe identity crate SHALL remain independent of Android, JNI, unsafe code, host frameworks, framing, Bluetooth, application coordination, persistence, transport, and frontend code. Rust allocations, layouts, references, strings, callbacks, opaque handles, generated bindings, payload logs, names, UUID diagnostics, and unrelated capabilities MUST NOT cross the boundary.

#### Scenario: Cross one bounded Android identity call

- **WHEN** Kotlin supplies one firmware payload and correctly shaped caller-owned output arrays
- **THEN** Rust validates the JNI regions, copies bounded input, decodes the identity, writes only the documented bounded record and metadata, returns a fixed status, and retains no Java reference or Rust allocation

#### Scenario: Map one successful Android record

- **WHEN** the bridge returns success
- **THEN** Kotlin validates ABI, status, flags, lengths, exact record extent and text before mechanically constructing the existing `FirmwareInfo` without interpreting VESC payload structure

#### Scenario: Contain Android bridge failure

- **WHEN** the bridge receives null, aliased, oversized, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, exception-pending, or internally failed input
- **THEN** it writes no unvalidated partial model, unwinds no panic across JNI, returns the documented bounded result, and logs no payload or private identity

#### Scenario: Package source-built Android identity

- **WHEN** Gradle builds host tests or Android debug and release applications
- **THEN** it builds the exact locked source-owned identity bridge for the host or all four supported Android ABIs, loads or packages only the expected library, and commits no native binary

#### Scenario: Keep host bridges independent

- **WHEN** the Android identity bridge is built and audited
- **THEN** the existing Android framing bridge and Apple identity bridge retain their separate crates, symbols, packages, dependencies, and production behavior

#### Scenario: Preserve OS capability ownership

- **WHEN** Android selects Rust firmware identity decoding
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport continue through their existing platform owners without a Rust callback
