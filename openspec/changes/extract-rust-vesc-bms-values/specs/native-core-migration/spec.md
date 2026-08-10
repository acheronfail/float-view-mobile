## ADDED Requirements

### Requirement: Rust VESC BMS values candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to structurally decoding bounded `COMM_BMS_GET_VALUES = 96` payloads. It SHALL decode all fixed electrical, environmental, state, lifetime, pressure, reported-CAN, bounded cell, balancing, temperature, and optional extended-tail fields without owning observation time, route scheduling, discovery, polling, availability, application state, or persistence.

The candidate SHALL preserve signed big-endian fixed-point and VESC auto-float rules; zero-through-50 cell and temperature counts; exact boolean flags; reported CAN IDs through 253 and optional route matching; exact legacy termination; one-byte data version and valid UTF-8 status through 40 content bytes for the extended form; exact remainder consumption; finite numeric results; and unsupported-command versus structural failure behavior. It SHALL use statically bounded collection storage, forbid unsafe code, log no payload or decoded values, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Decode exact source-derived BMS values

- **WHEN** the candidate receives either committed legacy or extended VBMS/VBMS32 command-96 payload
- **THEN** it returns every fixed value, array entry, flag, reported CAN ID, data version, and status with the same units and values as the selected native decoders

#### Scenario: Accept exact collection and tail bounds

- **WHEN** the candidate receives zero or 50 cells and temperatures, valid balancing flags, and absent or maximum 40-byte extended status
- **THEN** it returns complete bounded counts and values without allocation proportional to untrusted input

#### Scenario: Reject malformed BMS values

- **WHEN** input is empty, truncated, has another command, excessive counts, a non-boolean flag, a non-finite or excessive auto-float, invalid or mismatched reported CAN ID, malformed UTF-8, unterminated or oversized status, or unexpected remainder
- **THEN** the candidate returns the matching bounded structural or unsupported-command failure and exposes no partial values or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for command-96 values decoding and no supported-backend or native-code deletion claim is made
