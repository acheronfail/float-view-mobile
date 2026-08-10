## MODIFIED Requirements

### Requirement: Android Rust frame codec bridge ownership

The Android backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through a thin hand-written Kotlin adapter and one Android-specific JNI bridge crate under `backends/core`. Rust SHALL own CRC, encode, exact decode, and every stream-boundary decision while Kotlin MAY retain only bounded caller-owned stream storage and mechanical slicing and consumption. The safe framing crate SHALL remain independent of Android, JNI, and unsafe code; the bridge crate SHALL contain only bounded JNI representation conversion and SHALL depend only on the safe framing crate, the Rust standard library, and the existing locked minimal JNI ABI binding.

The versioned bridge SHALL use fixed integer statuses, JVM-owned primitive arrays, explicit lengths and capacities, panic containment, and an exact application-symbol allowlist. The stateless scanner operation SHALL return only validated frame offset, frame length, and consumed length values and SHALL retain no input or state after the call. Rust layouts, allocator ownership, callbacks, returned native pointers, opaque handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, storage, and UI composition SHALL remain with the existing Android owners in the single Gradle application module.

#### Scenario: Locate Android framing ownership

- **WHEN** a maintainer follows the Android `protocol/vesc` codec and stream owners
- **THEN** the Kotlin adapter identifies Rust as the production CRC, encode, exact-decode, and stream-boundary owner while Kotlin storage and mechanical consumption remain plainly identified

#### Scenario: Cross one bounded JNI codec call

- **WHEN** Kotlin requests frame encoding or exact decoding
- **THEN** it supplies bounded JVM-owned input, output, and output-length arrays and receives a fixed status without transferring native allocator ownership or retaining a native reference

#### Scenario: Cross one bounded JNI scanner call

- **WHEN** Kotlin supplies its current bounded notification bytes for stream scanning
- **THEN** Rust returns a fixed status and validated caller-owned primitive frame and consumption lengths without retaining the array, allocating returned memory, or creating native stream state

#### Scenario: Contain a JNI bridge failure

- **WHEN** the bridge receives a null, oversized, aliased, malformed, wrong-shaped, insufficient-capacity, pending-exception, or internally failed call
- **THEN** it writes nothing outside the supplied arrays, unwinds no panic across JNI, returns the documented fixed failure, and logs no address, token, payload, or full frame

#### Scenario: Keep one Android application module

- **WHEN** Gradle builds the Android host with complete Rust framing selected
- **THEN** the generated native libraries are packaged by the existing `app` module without adding a native API module, generated model layer, or general shared runtime

#### Scenario: Preserve OS capability ownership

- **WHEN** Android selects Rust stream boundary scanning
- **THEN** BLE, background logging, location, file presentation, theme, lifecycle, permissions, and WebView integration continue through their existing Kotlin and Android framework owners without a Rust callback
