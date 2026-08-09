## ADDED Requirements

### Requirement: Bounded incremental Rust extraction

Each OpenSpec change that moves backend behavior into Rust SHALL name one bounded behavioral slice, its existing production owner, its parity evidence, its integration state, and the checkpoint at which superseded production code may be deleted. A change MUST NOT combine an unproven component extraction with unrelated protocol, application, persistence, server, Bluetooth, or platform-lifecycle migrations.

#### Scenario: Start a Rust extraction

- **WHEN** a backend behavior is first implemented under `backends/core`
- **THEN** the change identifies the exact behavior and existing owner and supplies focused parity checks without claiming unrelated ownership

#### Scenario: Reject an umbrella migration

- **WHEN** a proposed step combines multiple independently cuttable backend layers or platform integrations
- **THEN** it is divided into separately verifiable OpenSpec changes before implementation

### Requirement: Candidate and production ownership remain distinct

A Rust implementation exercised only by component tests SHALL be described as a candidate and MUST NOT be treated as a supported backend or production owner. Production ownership SHALL transfer for one host only when a later change integrates that host, exercises its selected production path through the complete required gate, and records the superseded-code deletion decision.

#### Scenario: Component parity passes

- **WHEN** a Rust candidate passes all framing unit and parity tests without a host bridge
- **THEN** Swift and Kotlin remain the Apple and Android production owners and supported-backend claims remain unchanged

#### Scenario: One host cuts over

- **WHEN** a later Apple or Android change selects the Rust component through a narrow audited bridge and passes that host's complete gate
- **THEN** ownership changes only for that host and the other host retains its current production implementation

#### Scenario: Delete superseded code

- **WHEN** a host cutover has passed its focused, canonical, packaging, and required physical checkpoints
- **THEN** that change deletes the host's superseded implementation instead of retaining an indefinite fallback

### Requirement: Rust VESC framing parity

The initial Rust crate SHALL reproduce the shipped bounded VESC framing behavior: CRC-16 with polynomial `0x1021` and zero initial state; short frames for payloads through 255 bytes; long frames above 255 bytes; big-endian long length and CRC fields; footer byte `3`; a maximum payload of 4,096 bytes; and a maximum stream buffer of 8,192 bytes. Exact decoding and stream reassembly SHALL fail closed on invalid CRC, footer, header, declared length, trailing bytes, and aggregate bounds while accepting valid fragmentation, multiple frames, and leading noise according to current product behavior.

#### Scenario: Encode a short or long frame

- **WHEN** Rust encodes a payload at either side of the 255-byte boundary within the product limit
- **THEN** its complete bytes match the immutable source-derived vector and the shipped implementations

#### Scenario: Decode an exact frame

- **WHEN** Rust receives one complete valid short or long frame with no trailing bytes
- **THEN** it returns exactly the framed payload after validating length, footer, and CRC

#### Scenario: Reassemble a fragmented stream

- **WHEN** valid frames arrive across arbitrary fragments with leading noise or multiple frames in one fragment
- **THEN** Rust emits the same ordered payloads as the shipped bounded stream decoders

#### Scenario: Reject malformed or oversized input

- **WHEN** a frame or stream has an unsupported header, invalid CRC or footer, inconsistent length, payload above 4,096 bytes, trailing exact-frame bytes, or buffered data above 8,192 bytes
- **THEN** Rust returns a typed bounded error and retains no unbounded malformed state

### Requirement: Minimal locked Rust workspace

The `backends/core` Cargo workspace SHALL initially contain exactly the VESC framing crate, use the repository-pinned Rust toolchain, commit its lockfile, and have no third-party normal dependencies. Empty future crates, cross-language bindings, generated headers, mobile target packaging, and host callbacks MUST NOT be introduced by the framing-extraction change.

#### Scenario: Inspect the first workspace

- **WHEN** a maintainer reads its members and production dependency graph
- **THEN** only the VESC framing crate and its standard-library production dependencies are present

#### Scenario: Run the host software checks

- **WHEN** repository verification exercises the Rust workspace on the pinned host toolchain
- **THEN** formatting, warnings-denied linting, and all component tests pass without installing a mobile compilation target

### Requirement: Platform responsibilities stay native

Rust migration SHALL keep OS Bluetooth, location, background lifecycle, permissions, native file presentation, theme synchronization, and UI integration owned by each platform host. Portable layers MAY migrate below those adapters only through subsequent bounded changes that preserve the authenticated backend contract and dependency direction.

#### Scenario: Extract a portable protocol component

- **WHEN** VESC framing is added as a Rust candidate
- **THEN** no BLE, location, background, file-dialog, share-sheet, theme, WebView, or permission behavior moves from its platform owner
