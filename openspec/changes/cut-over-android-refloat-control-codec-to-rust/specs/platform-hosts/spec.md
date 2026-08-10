## ADDED Requirements

### Requirement: Android Rust Refloat control bridge ownership

The Android backend SHALL expose the safe Refloat control component through one dedicated, versioned, stateless JNI ABI. The ABI SHALL contain exactly three exported operations: version query, control encode, and lighting-response decode. It SHALL use fixed operation, profile, target, status, and primitive-result values; JVM-owned bounded input, output, and integer arrays; explicit lengths; alias and pending-exception validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, returned pointer, reference, or opaque lifetime crossing the boundary.

The shared bridge catalogue, generic locked Android build helper, Gradle host-test linkage, and Gradle application packaging SHALL build the same bridge for the host JVM and all four supported Android ABIs in debug and release. No generated binding or native library SHALL be committed. Existing Android bridges and all Apple bridges SHALL remain independent.

#### Scenario: Exercise one bounded encode call

- **WHEN** Kotlin supplies one canonical control operation and JVM-owned output/result arrays
- **THEN** Rust validates every primitive/reference, encodes at most nine bytes, returns fixed status/length/unsupported-command values, and retains no Java reference or Rust allocation

#### Scenario: Exercise one bounded lighting decode call

- **WHEN** Kotlin supplies one exact lighting response, selected profile, and JVM-owned result array
- **THEN** Rust returns one validated two-bit primitive result or fixed failure without exposing a Rust model or retaining input

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives an invalid operation, profile, target, boolean, current state, unused parameter, null, oversized, aliased, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, exception-pending, or internally failed call
- **THEN** it writes no unvalidated partial result, unwinds no panic across JNI, returns the documented bounded result, retains no Java reference, and logs no payload, state, address, or token

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications from repository-pinned source
- **THEN** the source-owned three-symbol bridge is loaded for JVM tests and packaged for `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` without a generated binding, committed library, or host-only substitution

#### Scenario: Preserve platform and Apple isolation

- **WHEN** Android production uses Rust Refloat control encoding and decoding
- **THEN** Bluetooth, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain native and Apple contains no Android JNI artifact or dependency
