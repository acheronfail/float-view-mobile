## MODIFIED Requirements

### Requirement: Rust VESC firmware identity candidate parity

The `backends/core` workspace SHALL contain one safe Rust firmware identity component with an empty production dependency graph dedicated to structurally decoding bounded VESC `COMM_FW_VERSION` payloads. It SHALL expose major and minor versions, a hardware name of at most 64 input bytes, a private 12-byte UUID, test-version marker, hardware type, custom-configuration count, an optional firmware name of at most 64 input bytes, and an optional big-endian four-byte hardware-configuration CRC. It SHALL consume but not expose the established pairing and capability bytes and SHALL log no payload, UUID, name, address, token, or full identity.

The decoder SHALL accept the existing audited motor and custom-module tail, SHALL accept hardware type `1` only as exact stock VESC BMS 6.00 with no trailing bytes, and SHALL fail closed with typed unsupported-command or structural/truncation errors for every malformed prefix, missing or overlong terminator, incomplete capability tail, malformed BMS identity, invalid CRC-tail length, or unexpected trailing byte. Invalid UTF-8 in a bounded terminated name SHALL use replacement decoding. The shared Apple backend SHALL select this component for production firmware identity decoding through its audited bridge; Kotlin SHALL remain Android's production owner until a separate Android cutover deletes its superseded parser.

#### Scenario: Decode an audited motor identity

- **WHEN** the Rust component receives a source-derived THOR300 or VESC Express firmware payload with the established motor/custom-module tail
- **THEN** it returns the same version, bounded names, private UUID bytes, test marker, hardware type, custom-configuration count, and optional CRC as the established native behavior

#### Scenario: Decode an audited BMS identity

- **WHEN** the component receives exact stock VBMS 6.00 or VBMS32 6.06 source-derived payload evidence
- **THEN** it preserves the structural distinction between the exact stock-BMS tail and the custom-module tail without applying higher-level BMS compatibility policy

#### Scenario: Reject malformed identity input

- **WHEN** input has the wrong command, any truncated common field, no bounded terminator, an invalid stock-BMS version or tail, incomplete extended fields, an invalid optional CRC length, or trailing bytes
- **THEN** the component returns the matching bounded typed failure and exposes no partial identity

#### Scenario: Preserve private identity

- **WHEN** parity tests exercise meaningful UUID bytes and malformed private input
- **THEN** exact bytes remain available only in the returned in-process model or caller-owned bridge result and do not appear in ordinary output, formatted errors, or repository diagnostics

#### Scenario: Select Apple while preserving Android ownership

- **WHEN** the Apple bridge and production adapter pass focused, packaging, canonical, and physical verification
- **THEN** iOS and macOS select Rust firmware identity decoding while Android continues selecting Kotlin without depending on the Apple bridge

## ADDED Requirements

### Requirement: Apple Rust VESC firmware identity cutover

The shared Apple backend SHALL use the safe Rust firmware identity component as its sole production implementation of VESC `COMM_FW_VERSION` structural decoding on iOS and macOS. Rust SHALL decide command validity, common-prefix extent, both 64-byte terminated-name bounds, private UUID extraction, test/type/configuration fields, exact stock-BMS admission, custom-module capability/name extent, optional CRC extent and value, and every structural failure. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `FirmwareInfo`, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift identity parsing, and SHALL delete the superseded Swift parser after required verification. Compatibility admission, route attribution, application behavior, and Android identity ownership SHALL remain outside this change.

#### Scenario: Decode Apple production identity

- **WHEN** shared Apple production code receives a structurally valid direct or CAN-routed firmware payload
- **THEN** the thin Swift adapter obtains every identity field from Rust and executes no Swift VESC prefix, terminator, hardware-tail, or CRC-tail algorithm

#### Scenario: Map Apple production failure

- **WHEN** Rust reports an unsupported command, structural failure, ABI failure, or invalid bounded result
- **THEN** Swift returns the established `VescProtocolError` class without exposing payload, UUID, names, or partial identity

#### Scenario: Preserve higher-level policy

- **WHEN** Rust returns a structurally valid motor, Express, stock-BMS, or VBMS32 identity
- **THEN** existing Swift compatibility, routing, discovery, application, and state owners apply unchanged policy after representation mapping

#### Scenario: Reject an Apple fallback

- **WHEN** bridge decoding fails or the input is malformed
- **THEN** Apple fails closed through the fixed status mapping and does not execute a Swift structural decoder

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple firmware identity cutover completes
- **THEN** Android continues using its Kotlin decoder with unchanged behavior and without linking an Apple artifact
