## MODIFIED Requirements

### Requirement: Rust VESC controller-data candidate parity

The `backends/core` workspace SHALL contain one safe Rust component with an empty production dependency graph dedicated to decoding the consumed standard VESC `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset. It SHALL accept unframed payloads without owning request construction, firmware compatibility, VESC framing, CAN routing, scheduling, telemetry polling, application state, or persistence.

The component SHALL preserve command validation; big-endian signed fixed-point scales; the consumed 30-byte realtime prefix and permitted later fields; raw fault code and none, over-voltage, or unknown classification; VESC auto-float decoding; four finite nonzero current limits; the closed supported serialization-signature to battery-series-cell-offset table; and optional behavior for unknown signatures, absent offsets, and zero cell counts. It SHALL forbid unsafe code, expose only bounded payload-safe failures, and be selected by Apple and Android through separate audited host bridges.

#### Scenario: Decode exact realtime values

- **WHEN** the component receives each committed normal or faulted command-4 payload
- **THEN** it returns the exact temperatures, currents, duty, ERPM, voltage, raw fault code, and stable fault classification selected native decoders produce

#### Scenario: Preserve consumed-prefix behavior

- **WHEN** a command-4 payload contains exactly the consumed 30-byte prefix or additional standard fields
- **THEN** the component decodes the same selected values without requiring or interpreting the permitted tail

#### Scenario: Decode bounded current limits

- **WHEN** the component receives command 14 with valid VESC auto-float limits and any supported serialization signature
- **THEN** it returns the four exact finite nonzero limits and the positive series-cell byte when the signature-specific offset exists

#### Scenario: Preserve optional cell-count behavior

- **WHEN** the motor configuration has an unknown signature, ends before its recognized cell offset, or contains zero at that offset
- **THEN** the component returns valid limits with no series-cell value rather than inventing a value or rejecting the payload

#### Scenario: Reject malformed controller data

- **WHEN** either payload has the wrong command, truncates a consumed field, contains an invalid auto-float, or produces a zero or non-finite current limit
- **THEN** the component returns the matching bounded unsupported-command or truncated failure and exposes no partial values or payload diagnostic

#### Scenario: Select both host families independently

- **WHEN** the Android bridge and adapter pass focused, packaging, canonical, and physical verification
- **THEN** Android selects Rust controller-data decoding while iOS and macOS preserve their existing Rust selection through the independent Apple bridge

## ADDED Requirements

### Requirement: Android Rust VESC controller-data cutover

The Android backend SHALL use the safe Rust controller-data component as its sole production implementation of the consumed standard `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset. Rust SHALL decide command validity, every selected big-endian signed fixed-point scale, signed ERPM, raw fault and classification, consumed-prefix extent, permitted tail behavior, VESC auto-floats, finite nonzero limits, serialization signatures, series-cell offsets, optional cell-count behavior, and every structural or numeric failure. Kotlin MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `RealtimeValues`, `VescFault`, and `ControllerCurrentLimits`, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin controller-data parsing, and SHALL delete the superseded Kotlin algorithms after required verification. Request construction, firmware compatibility, route attribution, framing, scheduling, telemetry polling, fault response, application behavior, Apple controller-data ownership, and every OS capability SHALL remain outside this change.

#### Scenario: Decode Android production realtime values

- **WHEN** Android production receives a structurally valid direct or CAN-routed command-4 payload
- **THEN** the thin Kotlin adapter obtains every consumed scalar, ERPM, raw fault, and classification from Rust and executes no Kotlin byte-reading, scaling, or fault-classification algorithm

#### Scenario: Decode Android production current limits

- **WHEN** Android production receives a structurally valid command-14 motor configuration
- **THEN** Rust returns all four finite nonzero limits and the optional positive series-cell value while Kotlin executes no auto-float, signature, offset, or optional-cell algorithm

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an unsupported command, truncated or invalid data, invalid caller contract, insufficient output, ABI mismatch, invalid result record, pending exception, or contained internal failure
- **THEN** Kotlin returns the established bounded `VescProtocolException` class without publishing partial values, logging payload data, or executing a fallback parser

#### Scenario: Preserve higher-level controller owners

- **WHEN** Rust returns valid realtime values or current limits
- **THEN** existing Kotlin compatibility, direct/CAN routing, framing, scheduling, telemetry, fault-response, state-publication, and session owners behave unchanged after representation mapping

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android controller-data cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking, packaging, or calling the Android JNI artifact
