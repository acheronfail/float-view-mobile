## ADDED Requirements

### Requirement: Apple Rust Refloat compatibility cutover

The shared Apple backend SHALL use `refloat-compatibility` as its sole production implementation of the exact Refloat INFO request, bounded INFO identity decoding, supported-release/profile selection, and required dynamic-layout admission on iOS and macOS. Rust SHALL atomically decode INFO and select one closed compatibility result containing structural wire identity, telemetry profile/family, lighting profile, and fatal-clear availability. Swift SHALL perform only strict canonical-record validation, representation conversion into the existing models, established error mapping, and higher-level behavior.

The safe compatibility component SHALL own one fixed dependency-free-of-third-parties, unsafe-free canonical record and SHALL admit the existing safe dynamic-telemetry layout record without a second host serialization. The temporary Apple bridge SHALL publish caller-owned records with no Rust allocation, reference, callback, object, or lifetime crossing C. The cutover MUST delete superseded Swift INFO offsets, catalogue entries, profile derivation, required-name constants, set admission, and runtime fallback. Android, controller discovery effects, routing, scheduling, telemetry decoding, controls, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Construct and decode Apple INFO through Rust

- **WHEN** Apple production requests INFO or receives a valid direct or CAN-routed legacy or version-2 INFO response
- **THEN** Rust supplies the exact request and one complete accepted compatibility record while Swift executes no INFO byte-offset or catalogue/profile algorithm

#### Scenario: Admit Apple dynamic layout through Rust

- **WHEN** Apple production decodes a command-32 layout for the current connection generation
- **THEN** Rust compatibility validates the canonical Rust-owned layout record and decides required-field admission without Swift required-name constants, set logic, or a second layout serialization

#### Scenario: Fail closed without a Swift fallback

- **WHEN** the ABI, caller regions, payload, INFO protocol/release, compatibility record, profile codes, reserved bytes, canonical layout record, or required groups are invalid
- **THEN** Apple returns its established bounded failure or unsupported state without partial compatibility, payload diagnostics, stale admission, or native protocol-policy fallback

#### Scenario: Preserve higher-level Apple owners

- **WHEN** Rust returns a valid compatibility decision or layout admission
- **THEN** existing Apple discovery effects, route attribution, framing, scheduling, connection lifetime, polling, state publication, sessions, controls, persistence, and application behavior remain unchanged

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple cutover builds, packages, and executes
- **THEN** Android continues using its Kotlin INFO and compatibility implementation without linking, packaging, loading, or calling the Apple bridge

#### Scenario: Retire the temporary bridge with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the Apple compatibility FFI and corresponding later Android JNI crate and host record adapters are deleted rather than retained as permanent APIs
