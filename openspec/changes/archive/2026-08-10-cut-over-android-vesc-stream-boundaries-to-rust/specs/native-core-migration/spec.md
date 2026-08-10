## MODIFIED Requirements

### Requirement: Android stateless VESC frame codec cutover

The Android backend SHALL use the Rust `vesc-frame` component as the sole production implementation of VESC CRC-16, frame encoding, exact frame decoding, and stream boundary interpretation. Rust SHALL decide leading-noise disposal, short/long header extent, declared payload length and bound, incomplete versus complete input, complete-frame extent, ordered multiple-frame progress, malformed-frame consumption, and aggregate stream overflow. Kotlin MAY retain only caller-owned bounded byte storage and mechanical slicing and consumption of the primitive offsets and lengths returned by Rust. Every complete frame SHALL be validated and decoded by Rust, and the public Kotlin codec, stream-decoder, and protocol-error behavior SHALL remain stable.

The Android cutover SHALL use the existing narrow audited JNI bridge, SHALL contain no runtime fallback to Kotlin framing algorithms, and SHALL delete the superseded Kotlin stream boundary parser after focused, packaging, canonical, and required physical checkpoints pass. Apple SHALL retain its complete existing Rust framing path unchanged. No Rust allocator ownership, callback, opaque runtime handle, or stream lifetime SHALL cross JNI.

#### Scenario: Encode an Android production frame

- **WHEN** Android production code encodes a valid payload at either short/long boundary or the product size limit
- **THEN** the thin Kotlin adapter obtains the complete frame from Rust and no Kotlin CRC or frame-construction algorithm runs

#### Scenario: Decode an Android production frame

- **WHEN** Rust stream scanning identifies one complete candidate frame
- **THEN** Rust validates its exact length, footer, CRC, and payload bound and returns either the payload or the established mapped protocol error

#### Scenario: Reassemble a fragmented Android stream

- **WHEN** Android receives valid fragmented, noisy, incomplete, or multiple-frame notification bytes
- **THEN** Rust reports the exact noise and frame consumption boundaries in order, Kotlin retains only the incomplete caller-owned suffix, and every complete frame is submitted to Rust exact decode

#### Scenario: Preserve malformed-stream recovery

- **WHEN** a complete frame has an invalid footer or CRC, a declared payload exceeds the product bound, or aggregate input exceeds the stream bound
- **THEN** Rust returns the established error and consumption decision, Kotlin removes or clears exactly that indicated state before propagating the error, and a later valid frame can recover identically to the previous implementation

#### Scenario: Reject an Android fallback

- **WHEN** the Android Rust bridge returns an incomplete outcome, semantic failure, caller-contract failure, or contained internal failure
- **THEN** the adapter follows the fixed status and validated primitive results without executing Kotlin header, length, noise, CRC, encoder, exact-decoder, or alternative recovery logic

#### Scenario: Preserve Apple framing ownership

- **WHEN** the Android stream-boundary cutover completes
- **THEN** iOS and macOS continue using their existing Rust framing and Apple C ABI without depending on the Android bridge or changing their stream behavior
