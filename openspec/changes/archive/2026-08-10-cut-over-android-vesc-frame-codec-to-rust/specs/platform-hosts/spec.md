## ADDED Requirements

### Requirement: Android Rust frame codec bridge ownership

The Android backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through a thin hand-written Kotlin adapter and one Android-specific JNI bridge crate under `backends/core`. Rust SHALL own CRC, encode, and exact decode while Kotlin SHALL retain bounded stream storage and boundary interpretation. The safe framing crate SHALL remain independent of Android, JNI, and unsafe code; the bridge crate SHALL contain only bounded JNI representation conversion and SHALL depend only on the safe framing crate, the Rust standard library, and the one locked minimal JNI ABI binding selected by the design.

The versioned bridge SHALL use fixed integer statuses, JVM-owned primitive arrays, explicit lengths and capacities, panic containment, and an exact application-symbol allowlist. Rust layouts, allocator ownership, callbacks, returned native pointers, opaque handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, storage, and UI composition SHALL remain with the existing Android owners in the single Gradle application module.

#### Scenario: Locate Android framing ownership

- **WHEN** a maintainer follows the Android `protocol/vesc` codec and stream owners
- **THEN** the Kotlin adapter identifies Rust as the production CRC, encode, and exact-decode owner while the retained Kotlin stream decoder remains plainly identified

#### Scenario: Cross one bounded JNI call

- **WHEN** Kotlin requests frame encoding or exact decoding
- **THEN** it supplies bounded JVM-owned input, output, and output-length arrays and receives a fixed status without transferring native allocator ownership or retaining a native reference

#### Scenario: Contain a JNI bridge failure

- **WHEN** the bridge receives a null, oversized, aliased, malformed, insufficient-capacity, pending-exception, or internally failed call
- **THEN** it writes nothing outside the supplied arrays, unwinds no panic across JNI, returns the documented fixed failure, and logs no address, token, payload, or full frame

#### Scenario: Keep one Android application module

- **WHEN** Gradle builds the Android host with the Rust codec selected
- **THEN** the generated native libraries are packaged by the existing `app` module without adding a native API module, generated model layer, or general shared runtime

#### Scenario: Preserve OS capability ownership

- **WHEN** Android selects Rust frame encoding and exact decoding
- **THEN** BLE, background logging, location, file presentation, theme, lifecycle, permissions, and WebView integration continue through their existing Kotlin and Android framework owners without a Rust callback
