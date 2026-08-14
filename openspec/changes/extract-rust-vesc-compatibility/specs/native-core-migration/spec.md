## ADDED Requirements

### Requirement: Rust VESC compatibility candidate parity

The repository SHALL provide one safe bounded Rust candidate that consumes the existing safe firmware-identity model and owns the exact VESC motor-controller compatibility catalogue, access-mode decision, operation authorization, and shutdown availability while Apple and Android retain their selected native policy.

#### Scenario: Admit only exact audited motor firmware
- **WHEN** a structurally decoded identity has any hardware type and any major/minor pair, including the complete byte-domain boundaries
- **THEN** the candidate SHALL admit only motor hardware type `0` at exactly 6.02, 6.05, 6.06, or 7.00 and SHALL reject every adjacent, unknown, gateway, BMS, or custom-module identity

#### Scenario: Classify stable and test access
- **WHEN** an admitted identity reports test marker zero or any nonzero byte
- **THEN** the candidate SHALL classify zero as read/write and every nonzero value as read-only without using names, UUID, CRC, custom-configuration count, or other identity fields

#### Scenario: Authorize operations
- **WHEN** read or write authorization is requested for supported stable firmware, supported test firmware, or unsupported firmware
- **THEN** reads SHALL be permitted for both supported access modes, writes SHALL be permitted only for read/write mode, and unsupported firmware SHALL permit neither

#### Scenario: Authorize exact shutdown releases
- **WHEN** shutdown availability is requested
- **THEN** it SHALL be true only for stable motor firmware 6.05, 6.06, or 7.00 and false for 6.02, every test build, and every unsupported identity

#### Scenario: Remain total over identity bounds
- **WHEN** every compatibility input and unrelated bounded identity field reaches valid limits
- **THEN** all public decisions SHALL remain deterministic and panic-free and private identity material SHALL remain absent from diagnostics

### Requirement: VESC compatibility candidate remains below application execution

The Rust VESC compatibility candidate SHALL expose only safe immutable catalogue and decision APIs over the safe firmware-identity model. It MUST NOT own framing, payload decoding, routing, discovery state, clocks, scheduling, BLE, mutation execution, callbacks, foreign records, bridges, persistence, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, or OS frameworks.

#### Scenario: Compose controller discovery directly
- **WHEN** the next Rust controller-discovery candidate evaluates direct or forwarded identities
- **THEN** it SHALL call this safe API directly without copying the catalogue or introducing a VESC-compatibility Apple FFI or Android JNI crate

#### Scenario: Compose a unified runtime later
- **WHEN** a later unified Rust runtime owns discovery and application mutation policy
- **THEN** it SHALL retain this direct safe dependency while the temporary firmware-identity and protocol bridge crates are deleted
