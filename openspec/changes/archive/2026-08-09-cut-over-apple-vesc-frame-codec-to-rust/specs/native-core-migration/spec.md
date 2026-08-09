## ADDED Requirements

### Requirement: Apple stateless VESC frame codec cutover

The shared Apple backend SHALL use the Rust `vesc-frame` component as the sole production implementation of VESC CRC-16, frame encoding, and exact frame decoding on both iOS and macOS. Swift MAY retain bounded stream boundary accumulation and complete-frame extraction, but each extracted frame SHALL be validated and decoded by Rust. The public Swift codec surface and established protocol error behavior SHALL remain stable. Android SHALL retain Kotlin framing ownership, and the Rust stream decoder SHALL remain a candidate, until separately scoped host-cutover changes complete.

The Apple cutover SHALL use one narrow audited bridge, SHALL contain no runtime fallback to Swift framing algorithms, and SHALL delete the superseded Swift CRC, encoder, and exact decoder after focused, packaging, canonical, and required physical checkpoints pass.

#### Scenario: Encode an Apple production frame

- **WHEN** shared Apple production code encodes any valid payload at the short/long boundary or product size limit
- **THEN** its thin Swift adapter obtains the complete frame from the Rust implementation and no Swift CRC or frame-construction algorithm runs

#### Scenario: Decode an Apple production frame

- **WHEN** the Apple stream decoder extracts one complete candidate frame
- **THEN** Rust validates its exact length, footer, CRC, and payload bound and returns either the payload or the established mapped protocol error

#### Scenario: Reassemble a fragmented Apple stream

- **WHEN** Apple receives valid fragmented, noisy, or multiple-frame notification bytes
- **THEN** Swift retains only bounded boundary/reassembly state and submits each complete frame to the Rust exact decoder in order

#### Scenario: Reject an Apple fallback

- **WHEN** the Apple Rust bridge returns a semantic or contained internal failure
- **THEN** the adapter fails closed through the established error surface without executing a Swift CRC, encoder, or exact decoder

#### Scenario: Preserve independent ownership

- **WHEN** the Apple stateless codec cutover completes
- **THEN** Android continues using its Kotlin framing implementation and neither host selects the Rust stream decoder
