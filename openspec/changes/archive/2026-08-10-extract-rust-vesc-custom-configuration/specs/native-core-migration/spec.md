## ADDED Requirements

### Requirement: Rust VESC custom-configuration candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to encoding and decoding bounded VESC custom-configuration command envelopes. It SHALL encode command-92 schema requests, command-93 current requests, command-94 default requests, and command-95 writes; it SHALL decode bounded schema chunks, opaque current/default values, and exact write acknowledgements without interpreting compressed schema or typed Refloat values.

The candidate SHALL preserve the selected production command bytes, index byte, signed big-endian length/offset rules, 400-byte schema chunk bound, minimum four-byte values signature extent, existing write bound, response variant, and structural versus recognized-unsupported failure distinction. It SHALL borrow or return only bounded caller-owned bytes, forbid unsafe code, log no configuration payload, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Encode exact configuration requests

- **WHEN** the candidate receives valid schema, current/default, or write inputs at representative and boundary extents
- **THEN** it emits the exact command 92–95 payload bytes produced by the selected native codecs

#### Scenario: Decode exact configuration responses

- **WHEN** the candidate receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** it returns the matching typed envelope, exact index/extents/command, and unchanged opaque bytes without parsing Refloat semantics

#### Scenario: Reject malformed configuration envelopes

- **WHEN** input has invalid integer extent, truncation, excessive chunk data, inconsistent total/offset/chunk arithmetic, insufficient value bytes, an invalid acknowledgement tail, an unrecognized command, or a recognized non-configuration command
- **THEN** the candidate returns the matching bounded structural or unsupported-command failure and exposes no partial response or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for commands 92–95 and no supported-backend or native-code deletion claim is made
