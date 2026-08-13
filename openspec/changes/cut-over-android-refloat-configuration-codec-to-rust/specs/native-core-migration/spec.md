## ADDED Requirements

### Requirement: Android Rust Refloat configuration codec cutover

The Android backend SHALL select the bounded Rust Refloat configuration codec as its sole owner of compressed schema decoding, normalized schema fingerprint and wire-signature construction, typed current/default value decoding, and typed write-value encoding, while retaining stateful configuration and application behavior in Kotlin until their separate migrations.

#### Scenario: Decode and retain one canonical schema

- **WHEN** the Android configuration service completes a bounded compatible Refloat schema transfer or selects its cached compressed schema
- **THEN** the production Rust codec SHALL validate and decode it into the canonical schema record
- **AND** Kotlin SHALL mechanically project that record into its existing normalized schema model
- **AND** the service SHALL retain the exact record with that schema for the active connection generation

#### Scenario: Decode current and default values with the selected schema

- **WHEN** Android receives current or default configuration value bytes
- **THEN** the production Rust codec SHALL validate the retained schema record and decode every typed field into the canonical value record
- **AND** Kotlin SHALL mechanically project that record without owning field layout, scalar decoding, scale conversion, string framing, or CRC semantics

#### Scenario: Encode a configuration write with the selected schema

- **WHEN** Android schedules a validated configuration write
- **THEN** Kotlin SHALL mechanically describe the complete typed value set in the canonical value record
- **AND** Rust SHALL reject any stale, duplicate, incomplete, mistyped, non-canonical, or out-of-bounds record
- **AND** Rust alone SHALL encode the accepted Refloat wire payload and checksum

#### Scenario: Fail closed without a Kotlin codec fallback

- **WHEN** ABI validation, JNI validation, record validation, schema decode, value decode, or value encode fails
- **THEN** Android SHALL map the bounded failure to its existing closed configuration error behavior
- **AND** it SHALL NOT retry any superseded Kotlin XML, fingerprint, signature, scalar, framing, or checksum implementation

#### Scenario: Preserve stateful Android ownership

- **WHEN** the stateless codec is selected
- **THEN** Kotlin SHALL retain ownership of compatibility catalogue selection, schema chunk assembly, cache policy and storage, connection generations, revisions, conflicts, flywheel rules, scheduler interaction, write/readback verification, Tune persistence, application publication, and frontend contract handling
- **AND** the Rust codec and bridge SHALL retain no host or connection state

#### Scenario: Preserve selected Apple ownership

- **WHEN** this Android cutover is built and verified
- **THEN** iOS and macOS SHALL continue selecting their existing Rust codec and Apple bridge
- **AND** no Apple C ABI, adapter, package, archive, symbol, or behavior SHALL change

#### Scenario: Retire both migration bridges later

- **WHEN** a future change moves configuration and application behavior behind the unified Rust runtime
- **THEN** it SHALL delete the temporary Android and Apple codec bridges and both canonical-record adapters rather than preserve them as permanent internal APIs
