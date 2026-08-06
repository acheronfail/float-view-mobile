## ADDED Requirements

### Requirement: Mirrored native Refloat configuration ownership

Apple and Android SHALL add matching owners for custom-schema chunk assembly, bounded Qt/zlib and XML decoding, normalized schema validation, CRC32C signature construction, typed value encoding/decoding, configuration operation lifecycle, and exact-version saved-configuration persistence. Protocol owners SHALL depend only on VESC framing/routing and configuration models; application coordination SHALL own scheduler submission and safety; storage SHALL own schema cache and Tunes; loopback serialization SHALL expose only normalized bounded contract values. Frontend code MUST NOT parse device XML, compute wire signatures, choose routes/commands, or persist authoritative Tunes.

#### Scenario: Locate schema codec

- **WHEN** a developer changes ConfigParams parsing or a VESC value encoding
- **THEN** corresponding focused Swift and Kotlin protocol owners and shared golden vectors are available under mirrored Refloat/VESC paths

#### Scenario: Disposable WebView

- **WHEN** the frontend is recreated during schema loading or after saved configurations exist
- **THEN** native connection, operation, cache, and durable Tune ownership remain intact and are recovered through authenticated contract methods

#### Scenario: Storage schema change

- **WHEN** exact-version saved configurations are introduced during test-stage development
- **THEN** canonical catalog SQL and both native stores change together, the storage version is bumped, and incompatible local storage is recreated rather than migrated

#### Scenario: No new parser dependency

- **WHEN** the platform implementations decode the compressed schema
- **THEN** they use bounded platform XML and zlib facilities unless pre-apply review demonstrates a smaller maintained dependency is necessary and records that decision
