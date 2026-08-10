## ADDED Requirements

### Requirement: Android stateless VESC frame codec cutover

The Android backend SHALL use the Rust `vesc-frame` component as the sole production implementation of VESC CRC-16, frame encoding, and exact frame decoding. Kotlin SHALL retain its existing bounded stream accumulation, leading-noise disposal, short/long header and declared-length interpretation, incomplete suffix retention, complete-frame extraction, ordered progress, and aggregate-overflow behavior until a separately scoped stream-boundary cutover completes. Every complete frame extracted by Kotlin SHALL be validated and decoded by Rust, and the public Kotlin codec, stream-decoder, and protocol-error behavior SHALL remain stable.

The Android cutover SHALL use one narrow audited JNI bridge, SHALL contain no runtime fallback to Kotlin CRC, encoding, or exact decoding, and SHALL delete those superseded Kotlin algorithms after focused, packaging, canonical, and required physical checkpoints pass. Apple SHALL retain its complete existing Rust framing path unchanged. No Rust allocator ownership, callback, opaque runtime handle, or stream lifetime SHALL cross JNI.

#### Scenario: Encode an Android production frame

- **WHEN** Android production code encodes a valid payload at either short/long boundary or the product size limit
- **THEN** the thin Kotlin adapter obtains the complete frame from Rust and no Kotlin CRC or frame-construction algorithm runs

#### Scenario: Decode an Android production frame

- **WHEN** the retained Kotlin stream decoder extracts one complete candidate frame
- **THEN** Rust validates its exact length, footer, CRC, and payload bound and returns either the payload or the established mapped protocol error

#### Scenario: Preserve Kotlin stream ownership

- **WHEN** Android receives fragmented, noisy, incomplete, multiple-frame, malformed, or aggregate-overflow input
- **THEN** Kotlin preserves the established bounded stream and recovery decisions while submitting every complete candidate to Rust exact decode in order

#### Scenario: Reject an Android fallback

- **WHEN** the Android Rust bridge returns a semantic, caller-contract, or contained internal failure
- **THEN** the adapter fails closed through the established Kotlin error surface without executing Kotlin CRC, encoder, or exact-decoder logic

#### Scenario: Preserve Apple framing ownership

- **WHEN** the Android stateless codec cutover completes
- **THEN** iOS and macOS continue using their existing Rust framing and Apple C ABI without depending on the Android bridge or changing their stream behavior
