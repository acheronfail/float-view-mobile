## ADDED Requirements

### Requirement: Apple Rust Refloat configuration codec cutover

The shared Apple backend SHALL select the bounded Rust Refloat configuration codec as its sole owner of compressed schema decoding, normalized schema fingerprint and wire-signature construction, typed current/default value decoding, and typed write-value encoding, while retaining stateful configuration and application behavior in Swift until their separate migrations.

#### Scenario: Decode and retain one canonical schema

- **WHEN** the Apple configuration service completes a bounded compatible Refloat schema transfer or selects its cached compressed schema
- **THEN** the production Rust codec SHALL validate and decode it into a canonical schema record
- **AND** Swift SHALL mechanically project that record into its existing normalized schema model
- **AND** the service SHALL retain the exact record with that schema for the active connection generation

#### Scenario: Decode current and default values with the selected schema

- **WHEN** Apple receives current or default configuration value bytes
- **THEN** the production Rust codec SHALL validate the retained schema record and decode every typed field into a canonical value record
- **AND** Swift SHALL mechanically project that record without owning field layout, scalar decoding, scale conversion, string framing, or CRC semantics

#### Scenario: Encode a configuration write with the selected schema

- **WHEN** Apple schedules a validated configuration write
- **THEN** Swift SHALL mechanically describe the complete typed value set in the canonical value record
- **AND** Rust SHALL reject any stale, duplicate, incomplete, mistyped, non-canonical, or out-of-bounds record
- **AND** Rust alone SHALL encode the accepted Refloat wire payload and checksum

#### Scenario: Fail closed without a Swift codec fallback

- **WHEN** ABI validation, bridge validation, record validation, schema decode, value decode, or value encode fails
- **THEN** Apple SHALL map the bounded failure to its existing closed configuration error behavior
- **AND** it SHALL NOT retry any superseded Swift XML, fingerprint, signature, scalar, framing, or checksum implementation

#### Scenario: Preserve stateful Apple ownership

- **WHEN** the stateless codec is selected
- **THEN** Swift SHALL retain ownership of compatibility catalogue selection, schema chunk assembly, cache policy and storage, connection generations, revisions, conflicts, flywheel rules, scheduler interaction, write/readback verification, Tune persistence, application publication, and frontend contract handling
- **AND** the Rust codec and bridge SHALL retain no host or connection state

#### Scenario: Preserve independent Android ownership

- **WHEN** this Apple cutover is built and verified
- **THEN** Android SHALL continue selecting its existing Kotlin configuration codec
- **AND** no Android JNI bridge, Gradle task, package, or artifact SHALL be added for this Apple-only change

#### Scenario: Retire the migration bridge later

- **WHEN** a future change moves the configuration and application service behind the unified Rust runtime
- **THEN** it SHALL delete this temporary Apple bridge and canonical-record adapter rather than preserve them as an internal permanent API
