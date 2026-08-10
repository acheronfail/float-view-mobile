## MODIFIED Requirements

### Requirement: Rust VESC BMS values candidate parity

The `backends/core` workspace SHALL contain one safe Rust component with an empty production dependency graph dedicated to structurally decoding bounded `COMM_BMS_GET_VALUES = 96` payloads. It SHALL decode all fixed electrical, environmental, state, lifetime, pressure, reported-CAN, bounded cell, balancing, temperature, and optional extended-tail fields without owning observation time, route scheduling, discovery, polling, availability, application state, or persistence.

The component SHALL preserve signed big-endian fixed-point and VESC auto-float rules; zero-through-50 cell and temperature counts; exact boolean flags; reported CAN IDs through 253 and optional route matching; exact legacy termination; one-byte data version and valid UTF-8 status through 40 content bytes for the extended form; exact remainder consumption; finite numeric results; and unsupported-command versus structural failure behavior. It SHALL use statically bounded collection storage, forbid unsafe code, log no payload or decoded values, and be selected by Apple and Android through separate audited host bridges.

#### Scenario: Decode exact source-derived BMS values

- **WHEN** the component receives either committed legacy or extended VBMS/VBMS32 command-96 payload
- **THEN** it returns every fixed value, array entry, flag, reported CAN ID, data version, and status with the same units and values as established production behavior

#### Scenario: Accept exact collection and tail bounds

- **WHEN** the component receives zero or 50 cells and temperatures, valid balancing flags, and absent or maximum 40-byte extended status
- **THEN** it returns complete bounded counts and values without allocation proportional to untrusted input

#### Scenario: Reject malformed BMS values

- **WHEN** input is empty, truncated, has another command, excessive counts, a non-boolean flag, a non-finite or excessive auto-float, invalid or mismatched reported CAN ID, malformed UTF-8, unterminated or oversized status, or unexpected remainder
- **THEN** the component returns the matching bounded structural or unsupported-command failure and exposes no partial values or payload diagnostic

#### Scenario: Select both host families independently

- **WHEN** the Android bridge and adapter pass focused, packaging, canonical, and physical verification
- **THEN** Android selects Rust command-96 structural decoding while iOS and macOS preserve their existing Rust selection through the independent Apple bridge

## ADDED Requirements

### Requirement: Android Rust VESC BMS values cutover

The Android backend SHALL use the safe Rust BMS-values component as its sole production implementation of `COMM_BMS_GET_VALUES = 96` structural decoding. Rust SHALL decide command validity, signed fixed-point fields, cell and temperature counts and entries, balancing flags, reported-CAN validity and route matching, VESC auto-floats, finite numeric values, pressure, exact legacy termination, extended data-version/status structure, UTF-8 validity and bound, remainder consumption, and every structural failure. Kotlin MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `BmsSnapshot`, observation-time attachment, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin BMS payload parsing, and SHALL delete the superseded Kotlin structural implementation after required verification. Firmware compatibility, BMS discovery, request construction, direct/CAN routing, scheduling, polling, observation time, availability, application state, frontend behavior, and Apple BMS ownership SHALL remain outside this change.

#### Scenario: Decode Android production BMS values

- **WHEN** Android production receives a valid legacy or extended command-96 payload from a direct or CAN-routed BMS
- **THEN** the thin Kotlin adapter obtains every structural field and collection from Rust and executes no Kotlin BMS payload, count, flag, auto-float, CAN, status-tail, or remainder algorithm

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an unsupported command, structural failure, invalid caller contract, insufficient output, ABI mismatch, invalid result record, pending exception, or contained internal failure
- **THEN** Kotlin returns the established bounded `VescProtocolException` class without publishing a partial snapshot, logging payload values, or executing a fallback parser

#### Scenario: Preserve higher-level BMS owners

- **WHEN** Rust returns structurally valid direct or CAN BMS values
- **THEN** existing Kotlin compatibility, discovery, routing, scheduling, polling, observation-time, availability, application-state, and live-only owners behave unchanged after representation mapping

#### Scenario: Preserve Apple BMS ownership

- **WHEN** Android completes its BMS-values cutover
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking, packaging, or calling the Android JNI artifact
