## ADDED Requirements

### Requirement: Shared protocol-neutral Android JNI safety support

The Rust workspace SHALL contain one `android-jni-support` component whose only external dependency is the locked `jni-sys = 0.4.1` package and that owns reusable Android JNI panic containment, per-call environment/function-table validation, pending-exception checks, primitive-array length and pairwise-reference-alias validation, maximum-bounded byte-array copying, bounded byte-array publication, and exact integer-array publication. The component SHALL retain no environment or reference, create no global reference, attach no thread, invoke no callback, depend on no protocol or host framework, and export no JNI symbol.

#### Scenario: Use a valid primitive-array boundary

- **WHEN** a bridge supplies one valid call-local environment with non-null, correctly typed, pairwise-distinct primitive arrays within the bridge-provided limits
- **THEN** support copies and publishes the exact requested bytes and integers without retaining a reference or accessing outside an array extent

#### Scenario: Reject an invalid JNI boundary

- **WHEN** the environment/function table is null, an exception is pending, a required reference is null, an array length is invalid or exceeds its declared maximum, two references alias, or a VM operation raises an exception
- **THEN** support fails closed without unbounded allocation, partial later publication, retained VM state, or private reference/payload diagnostics

#### Scenario: Contain a panic

- **WHEN** a bridge operation panics inside the shared containment boundary
- **THEN** no unwind crosses the JNI ABI and the consumer receives its explicit fallback result

### Requirement: Independent Android JNI bridge ABI preservation

The six Android JNI crates SHALL use shared support while retaining their separate Cargo packages, one domain dependency, explicit exported functions, JNI ABI versions, status numbers, argument order and types, record formats, shared-library names, Kotlin declarations/adapters, host tests, four-ABI packages, and selected production call paths. Domain decoding, encoding, validation limits, status mapping, result construction, and publication order SHALL remain in the corresponding bridge or protocol crate.

#### Scenario: Call every retained JNI bridge

- **WHEN** the framing, firmware-identity, custom-configuration, BMS-values, controller-data, and Refloat-control host/JNI suites exercise success and failure paths after migration
- **THEN** every exported symbol returns byte-for-byte compatible statuses, lengths, metadata, and record output through its existing library and Kotlin declaration

#### Scenario: Inspect JNI ownership

- **WHEN** repository audits inspect the six Android bridge sources and dependency graphs
- **THEN** raw JNI table calls and shared containment occur only in `android-jni-support`, each consumer depends only on its domain crate and support, and no bridge ABI or domain logic has moved into support

### Requirement: Android JNI support regression evidence

The refactor SHALL pass warnings-denied Rust tests/linting, focused support and six-bridge host/JNI/adversarial suites, dependency/raw-JNI-ownership audits, clean host and four-ABI Android Debug/Release packaging, unchanged Apple packaging, the complete `just verify` gate, and physical ESP32 regressions on the connected Android device, connected iOS device, and macOS host. Apple SHALL remain a behaviorally unchanged regression peer, and only bounded-time location sample absence MAY be deferred.

#### Scenario: Verify software equivalence

- **WHEN** focused and complete software verification runs after all bridge-local JNI primitives are removed
- **THEN** support tests, all six Android JNI contracts, all six Apple ABIs, packages, symbols, architectures, canonical conformance, and repository audits pass without generated or committed native artifacts

#### Scenario: Verify physical equivalence

- **WHEN** identity, direct/CAN control, BMS, and mobile background scenarios run as applicable through production BLE against the ESP32 fixture on Android, iOS, and macOS
- **THEN** discovery, routing, framing, protocol decoding/encoding, controls, telemetry, state, sessions, background continuity, and cleanup remain successful, with only bounded location acquisition eligible for deferred manual observation
