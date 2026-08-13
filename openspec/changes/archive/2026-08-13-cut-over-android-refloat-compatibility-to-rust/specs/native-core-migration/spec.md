## ADDED Requirements

### Requirement: Android Rust Refloat compatibility cutover

The Android backend SHALL use `refloat-compatibility` as its sole production implementation of the exact Refloat INFO request, bounded INFO identity decoding, supported-release/profile selection, and required dynamic-layout admission. Rust SHALL atomically decode INFO and select one closed compatibility result containing structural wire identity, telemetry profile/family, lighting profile, and fatal-clear availability. Kotlin SHALL perform only strict canonical-record validation, representation conversion into the existing models, established error mapping, and higher-level behavior.

The safe compatibility component SHALL remain the sole owner of its fixed canonical record and SHALL admit the existing safe dynamic-telemetry layout record without a second host serialization. The temporary Android bridge SHALL use JVM-owned primitive arrays and retain no Rust allocation, Java reference, callback, object, or lifetime across calls. The cutover MUST delete superseded Kotlin INFO offsets, catalogue entries, profile derivation, required-name constants, set admission, and runtime fallback. Apple, controller discovery effects, routing, scheduling, telemetry decoding, controls, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Construct and decode Android INFO through Rust

- **WHEN** Android production requests INFO or receives a valid direct or CAN-routed legacy or version-2 INFO response
- **THEN** Rust supplies the exact request and one complete accepted compatibility record while Kotlin executes no INFO byte-offset or catalogue/profile algorithm

#### Scenario: Admit Android dynamic layout through Rust

- **WHEN** Android production decodes a command-32 layout for the current connection generation
- **THEN** Rust compatibility validates the canonical Rust-owned layout record and decides required-field admission without Kotlin required-name constants, set logic, or a second layout serialization

#### Scenario: Fail closed without a Kotlin fallback

- **WHEN** the ABI, JVM arrays, payload, INFO protocol/release, compatibility record, profile codes, reserved bytes, canonical layout record, or required groups are invalid
- **THEN** Android returns its established bounded failure or unsupported state without partial compatibility, payload diagnostics, stale admission, or native protocol-policy fallback

#### Scenario: Preserve higher-level Android owners

- **WHEN** Rust returns a valid compatibility decision or layout admission
- **THEN** existing Android discovery effects, route attribution, framing, scheduling, connection lifetime, polling, state publication, sessions, controls, persistence, and application behavior remain unchanged

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their selected Apple bridge without linking, packaging, loading, or calling the Android JNI bridge

#### Scenario: Retire the temporary bridges with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the Apple compatibility FFI and Android compatibility JNI crates and host record adapters are deleted rather than retained as permanent APIs
