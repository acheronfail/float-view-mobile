## ADDED Requirements

### Requirement: Shared protocol-agnostic Apple FFI safety support

The Rust workspace SHALL contain one dependency-free `apple-ffi-support` component that owns reusable Apple C-boundary panic containment, checked address-region construction, alignment and pairwise-disjointness validation, validated input borrowing, bounded byte publication, and scalar output publication. The component SHALL retain no pointer or borrow, invoke no callback, allocate no unbounded collection, depend on no protocol or host framework, and expose no C symbol.

#### Scenario: Accept a valid disjoint boundary

- **WHEN** a bridge validates disjoint input, output, and metadata regions with supported alignment and capacity and then borrows or publishes within those exact bounds
- **THEN** the support component returns the same bytes and scalar values without reading or writing outside any validated region

#### Scenario: Reject an invalid boundary

- **WHEN** a boundary contains a required null pointer, a null pointer with nonzero length, misalignment, address wraparound, overlap, insufficient capacity, or an attempted publication beyond a validated region
- **THEN** the support component fails closed without constructing a slice, partially publishing a result, or exposing pointer or payload data in its error

#### Scenario: Contain a panic

- **WHEN** a bridge operation panics inside the support containment boundary
- **THEN** no unwind crosses the C ABI and the bridge can map the contained outcome to its existing internal-failure status

### Requirement: Independent Apple bridge ABI preservation

The six Apple FFI crates SHALL use the shared support component while retaining their separate Cargo packages, single domain dependencies, explicit exported functions, ABI versions, status numbers, argument order and types, record formats, public C headers, archive names, C ABI tests, Swift adapters, and selected production call paths. Domain decoding, encoding, validation limits, error mapping, and result construction SHALL remain in the corresponding bridge or protocol crate.

#### Scenario: Call every retained bridge

- **WHEN** the framing, firmware-identity, custom-configuration, BMS-values, controller-data, and Refloat-control C ABI suites call success and failure paths after migration
- **THEN** every exported symbol returns byte-for-byte compatible status, length, metadata, and record output through its existing archive and header

#### Scenario: Inspect bridge ownership

- **WHEN** repository audits inspect the six Apple bridge sources and dependency graphs
- **THEN** shared raw-memory primitives occur only in `apple-ffi-support`, each consumer depends only on its domain crate and the support crate, and no bridge ABI or domain logic has moved into support

### Requirement: Apple support regression evidence

The refactor SHALL pass warnings-denied Rust tests/linting, focused support and six-bridge ABI/adversarial suites, dependency and unsafe-ownership audits, all Apple architecture/package builds, the complete `just verify` software gate, and physical ESP32 regressions on the connected Android device, connected iOS device, and macOS host. Android remains a behaviorally unchanged regression peer, and only bounded-time location sample absence MAY be deferred.

#### Scenario: Verify software equivalence

- **WHEN** focused and complete software verification runs after all bridge-local primitives are removed
- **THEN** the support tests, all six Apple ABIs, every Android JNI bridge, packaging, symbols, architectures, canonical conformance, and repository audits pass without generated or committed native artifacts

#### Scenario: Verify physical equivalence

- **WHEN** identity, direct/CAN control, BMS, and mobile background scenarios run as applicable through production BLE against the ESP32 fixture on Android, iOS, and macOS
- **THEN** discovery, routing, framing, protocol decoding/encoding, controls, telemetry, state, sessions, background continuity, and cleanup remain successful, with only bounded location acquisition eligible for deferred manual observation
