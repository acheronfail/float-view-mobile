## MODIFIED Requirements

### Requirement: Rust VESC custom-configuration candidate parity

The `backends/core` workspace SHALL contain one safe Rust component with an empty production dependency graph dedicated to encoding and decoding bounded VESC custom-configuration command envelopes. It SHALL encode command-92 schema requests, command-93 current requests, command-94 default requests, and command-95 writes; it SHALL decode bounded schema chunks, opaque current/default values, and exact write acknowledgements without interpreting compressed schema or typed Refloat values.

The component SHALL preserve the selected production command bytes, index byte, signed big-endian length/offset rules, 400-byte schema chunk bound, minimum four-byte values signature extent, existing write bound, response variant, and structural versus recognized-unsupported failure distinction. It SHALL borrow or return only bounded caller-owned bytes, forbid unsafe code, and log no configuration payload. The shared Apple backend SHALL select it through the focused Apple bridge after cutover verification, while Kotlin SHALL remain Android's production owner until a separate Android change passes its own bridge, packaging, canonical, physical, and deletion gates.

#### Scenario: Encode exact configuration requests

- **WHEN** the component receives valid schema, current/default, or write inputs at representative and boundary extents
- **THEN** it emits the exact command 92–95 payload bytes produced by the selected native codecs

#### Scenario: Decode exact configuration responses

- **WHEN** the component receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** it returns the matching typed envelope, exact index/extents/command, and unchanged opaque bytes without parsing Refloat semantics

#### Scenario: Reject malformed configuration envelopes

- **WHEN** input has invalid integer extent, truncation, excessive chunk data, inconsistent total/offset/chunk arithmetic, insufficient value bytes, an invalid acknowledgement tail, an unrecognized command, or a recognized non-configuration command
- **THEN** the component returns the matching bounded structural or unsupported-command failure and exposes no partial response or payload diagnostic

#### Scenario: Select Apple while preserving Android ownership

- **WHEN** the focused Apple bridge and shared production adapter pass required verification
- **THEN** iOS and macOS select the safe Rust component for commands 92–95 while Android continues selecting Kotlin without depending on the Apple bridge

## ADDED Requirements

### Requirement: Apple Rust VESC custom-configuration cutover

The shared Apple backend SHALL use the safe Rust custom-configuration component as its sole production implementation of VESC command-92 schema requests and chunks, command-93 current requests and values, command-94 default requests and values, and command-95 writes and acknowledgements on iOS and macOS. Rust SHALL decide command bytes, index placement, signed big-endian request extents, schema chunk structure and arithmetic, opaque-values minimum extent, exact acknowledgement shape, and structural versus recognized-unsupported failures. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion, caller-owned opaque-byte copying, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift envelope algorithms, and SHALL delete the superseded Swift implementation after required verification. Schema assembly, decompression, XML, typed Refloat values, signatures, compatibility and safety policy, mutation coordination, routing, scheduling, application behavior, and Android custom-configuration ownership SHALL remain outside this change.

#### Scenario: Encode Apple production configuration requests

- **WHEN** shared Apple production requests schema chunks, current/default values, or a bounded write on either direct or CAN routing
- **THEN** the thin Swift adapter obtains the exact command 92–95 payload from Rust and executes no Swift command construction or signed-integer encoding

#### Scenario: Decode Apple production configuration responses

- **WHEN** shared Apple production receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** Rust returns the validated response kind, extents, index, command, and opaque-byte range and Swift executes no chunk, values-envelope, or acknowledgement parser

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an invalid argument, malformed envelope, recognized unsupported command, insufficient output, ABI mismatch, or contained internal failure
- **THEN** the adapter maps it to the established bounded protocol error without logging payload bytes or executing a Swift fallback

#### Scenario: Preserve higher-level owners

- **WHEN** Apple opens, reads, writes, or applies a Refloat configuration through the Rust envelope codec
- **THEN** native schema/value services retain decompression, XML, signatures, typed values, safety, coordination, routing, scheduling, and application behavior unchanged

#### Scenario: Preserve Android ownership

- **WHEN** Apple completes its custom-configuration cutover
- **THEN** Android continues to use its Kotlin command-92–95 codec and does not link, package, or call the Apple bridge
