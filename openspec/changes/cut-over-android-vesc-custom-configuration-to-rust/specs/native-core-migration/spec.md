## MODIFIED Requirements

### Requirement: Rust VESC custom-configuration candidate parity

The `backends/core` workspace SHALL contain one safe Rust component with an empty production dependency graph dedicated to encoding and decoding bounded VESC custom-configuration command envelopes. It SHALL encode command-92 schema requests, command-93 current requests, command-94 default requests, and command-95 writes; it SHALL decode bounded schema chunks, opaque current/default values, and exact write acknowledgements without interpreting compressed schema or typed Refloat values.

The component SHALL preserve the selected production command bytes, index byte, signed big-endian length/offset rules, 400-byte schema chunk bound, minimum four-byte values signature extent, existing write bound, response variant, and structural versus recognized-unsupported failure distinction. It SHALL borrow or return only bounded caller-owned bytes, forbid unsafe code, and log no configuration payload. Apple and Android SHALL select it through separate focused host bridges only after each cutover passes its own bridge, packaging, canonical, physical, and native-code deletion gates.

#### Scenario: Encode exact configuration requests

- **WHEN** the component receives valid schema, current/default, or write inputs at representative and boundary extents
- **THEN** it emits the exact command 92–95 payload bytes produced by the selected host adapters

#### Scenario: Decode exact configuration responses

- **WHEN** the component receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** it returns the matching typed envelope, exact index/extents/command, and unchanged opaque bytes without parsing Refloat semantics

#### Scenario: Reject malformed configuration envelopes

- **WHEN** input has invalid integer extent, truncation, excessive chunk data, inconsistent total/offset/chunk arithmetic, insufficient value bytes, an invalid acknowledgement tail, an unrecognized command, or a recognized non-configuration command
- **THEN** the component returns the matching bounded structural or unsupported-command failure and exposes no partial response or payload diagnostic

#### Scenario: Select both hosts independently

- **WHEN** the focused Apple and Android bridges and production adapters each pass their required verification
- **THEN** iOS, macOS, and Android select the same safe Rust component through separate host bridges without sharing platform artifacts

## ADDED Requirements

### Requirement: Android Rust VESC custom-configuration cutover

The Android backend SHALL use the safe Rust custom-configuration component as its sole production implementation of VESC command-92 schema requests and chunks, command-93 current requests and values, command-94 default requests and values, and command-95 writes and acknowledgements. Rust SHALL decide command bytes, index placement, signed big-endian request extents, schema chunk structure and arithmetic, opaque-values minimum extent, exact acknowledgement shape, and structural versus recognized-unsupported failures. Kotlin SHALL perform only bounded versioned primitive-result validation, representation conversion into the existing response types, caller-owned opaque-byte copying, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin envelope algorithms, and SHALL delete the superseded Kotlin implementation after required verification. Schema assembly, decompression, XML, typed Refloat values, signatures, compatibility and safety policy, mutation coordination, routing, scheduling, application behavior, Apple custom-configuration ownership, and every OS capability SHALL remain outside this change.

#### Scenario: Encode Android production configuration requests

- **WHEN** Android production requests schema chunks, current/default values, or a bounded write on either direct or CAN routing
- **THEN** the thin Kotlin adapter obtains the exact command 92–95 payload from Rust and executes no Kotlin command construction or signed-integer encoding

#### Scenario: Decode Android production configuration responses

- **WHEN** Android production receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** Rust returns the validated response kind, extents, index, command, and opaque-byte range and Kotlin executes no chunk, values-envelope, or acknowledgement parser

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an invalid argument, malformed envelope, recognized unsupported command, insufficient output, ABI mismatch, pending exception, or contained internal failure
- **THEN** the adapter maps it to the established bounded protocol exception without logging payload bytes or executing a Kotlin fallback

#### Scenario: Preserve higher-level owners

- **WHEN** Android opens, reads, writes, or applies a Refloat configuration through the Rust envelope codec
- **THEN** native schema/value services retain decompression, XML, signatures, typed values, safety, coordination, routing, scheduling, persistence, and application behavior unchanged

#### Scenario: Preserve Apple ownership

- **WHEN** Android completes its custom-configuration cutover
- **THEN** iOS and macOS continue using their existing safe Rust codec and Apple bridge without linking or calling the Android JNI artifact
