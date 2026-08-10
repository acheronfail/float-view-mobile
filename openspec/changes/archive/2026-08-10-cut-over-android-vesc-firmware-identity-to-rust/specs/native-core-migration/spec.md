## ADDED Requirements

### Requirement: Android Rust VESC firmware identity cutover

The Android backend SHALL use the safe Rust firmware identity component as its sole production implementation of VESC `COMM_FW_VERSION` structural decoding. Rust SHALL decide command validity, common-prefix extent, both 64-byte terminated-name bounds, private UUID extraction, test/type/configuration fields, exact stock-BMS admission, custom-module capability/name extent, optional CRC extent and value, and every structural failure. Kotlin SHALL perform only bounded versioned bridge-record validation, primitive representation conversion into the existing `FirmwareInfo`, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin identity parsing, and SHALL delete the superseded Kotlin parser after required verification. Compatibility admission, request scheduling, route attribution, application behavior, Apple identity ownership, and all other protocol owners SHALL remain outside this change.

#### Scenario: Decode Android production identity

- **WHEN** Android production code receives a structurally valid direct or CAN-routed firmware payload
- **THEN** the thin Kotlin adapter obtains every identity field from Rust and executes no Kotlin VESC prefix, terminator, hardware-tail, or CRC-tail algorithm

#### Scenario: Map Android production failure

- **WHEN** Rust reports an unsupported command, structural failure, ABI failure, caller-contract failure, or invalid bounded result
- **THEN** Kotlin returns the established `VescProtocolException` class without exposing payload, UUID, names, or partial identity

#### Scenario: Preserve higher-level policy

- **WHEN** Rust returns a structurally valid motor, Express, stock-BMS, or VBMS32 identity
- **THEN** existing Kotlin compatibility, routing, discovery, application, and state owners apply unchanged policy after representation mapping

#### Scenario: Reject an Android identity fallback

- **WHEN** bridge decoding fails or the input is malformed
- **THEN** Android fails closed through the fixed status and validated record mapping without executing a Kotlin structural decoder

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android firmware identity cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking or calling the Android JNI artifact
