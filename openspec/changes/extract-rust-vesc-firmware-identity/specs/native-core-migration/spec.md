## ADDED Requirements

### Requirement: Rust VESC firmware identity candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to structurally decoding bounded VESC `COMM_FW_VERSION` payloads. It SHALL expose major and minor versions, a hardware name of at most 64 bytes, a private 12-byte UUID, test-version marker, hardware type, custom-configuration count, an optional firmware name of at most 64 bytes, and an optional big-endian four-byte hardware-configuration CRC. It SHALL consume but not expose the established pairing and capability bytes and SHALL log no payload, UUID, name, address, token, or full identity.

The decoder SHALL accept the existing audited motor and custom-module tail, SHALL accept hardware type `1` only as exact stock VESC BMS 6.00 with no trailing bytes, and SHALL fail closed with typed unsupported-command or structural/truncation errors for every malformed prefix, missing or overlong terminator, incomplete capability tail, malformed BMS identity, invalid CRC-tail length, or unexpected trailing byte. Invalid UTF-8 in a bounded terminated name SHALL use replacement decoding to preserve established native behavior. Swift and Kotlin SHALL remain the only production owners until separate host-cutover changes select the candidate and delete their superseded parsers.

#### Scenario: Decode an audited motor identity

- **WHEN** the Rust candidate receives a source-derived THOR300 or VESC Express firmware payload with the established motor/custom-module tail
- **THEN** it returns the same version, bounded names, private UUID bytes, test marker, hardware type, custom-configuration count, and optional CRC as the current native decoders

#### Scenario: Decode an audited BMS identity

- **WHEN** the candidate receives exact stock VBMS 6.00 or VBMS32 6.06 source-derived payload evidence
- **THEN** it preserves the current structural distinction between the exact stock-BMS tail and the custom-module tail without applying higher-level BMS compatibility policy

#### Scenario: Reject malformed identity input

- **WHEN** input has the wrong command, any truncated common field, no bounded terminator, an invalid stock-BMS version or tail, incomplete extended fields, an invalid optional CRC length, or trailing bytes
- **THEN** the candidate returns the matching bounded typed failure and exposes no partial identity

#### Scenario: Preserve private identity

- **WHEN** parity tests exercise meaningful UUID bytes and malformed private input
- **THEN** exact bytes remain available only in the returned in-process model and do not appear in ordinary output, formatted errors, or repository diagnostics

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for firmware identity decoding and no supported-backend or native-code deletion claim is made
