# Native Core Migration Specification

## Purpose

Define the bounded, evidence-first process for incrementally moving portable backend behavior into Rust without conflating component parity with production host ownership.

## Requirements

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

### Requirement: Apple stateless VESC frame codec cutover

The shared Apple backend SHALL use the Rust `vesc-frame` component as the sole production implementation of VESC CRC-16, frame encoding, exact frame decoding, and stream boundary interpretation on both iOS and macOS. Rust SHALL decide leading-noise disposal, short/long header extent, declared payload length and bound, incomplete versus complete input, complete-frame extent, ordered multiple-frame progress, malformed-frame consumption, and aggregate stream overflow. Swift MAY retain only caller-owned bounded byte storage and mechanical consumption of the byte counts returned by Rust. Every complete frame SHALL be validated and decoded by Rust. The public Swift codec and stream-decoder surfaces and established protocol error and recovery behavior SHALL remain stable.

The Apple cutover SHALL use the existing narrow audited bridge, SHALL contain no runtime fallback to Swift framing algorithms, and SHALL delete the superseded Swift stream boundary parser after focused, packaging, canonical, and required physical checkpoints pass. Android SHALL retain its complete Kotlin framing implementation, and Rust-owned stream lifetime SHALL remain deferred until a later application-owner change can absorb it without introducing a temporary opaque FFI object.

#### Scenario: Encode an Apple production frame

- **WHEN** shared Apple production code encodes any valid payload at the short/long boundary or product size limit
- **THEN** its thin Swift adapter obtains the complete frame from the Rust implementation and no Swift CRC or frame-construction algorithm runs

#### Scenario: Decode an Apple production frame

- **WHEN** Rust stream scanning identifies one complete candidate frame
- **THEN** Rust validates its exact length, footer, CRC, and payload bound and returns either the payload or the established mapped protocol error

#### Scenario: Reassemble a fragmented Apple stream

- **WHEN** Apple receives valid fragmented, noisy, or multiple-frame notification bytes
- **THEN** Rust reports the exact noise and frame consumption boundaries in order, Swift retains only the incomplete caller-owned suffix, and every complete frame is submitted to Rust exact decode

#### Scenario: Preserve malformed-stream recovery

- **WHEN** a complete frame has an invalid footer or CRC, a declared payload exceeds the product bound, or aggregate input exceeds the stream bound
- **THEN** Rust returns the established error and consumption decision, Swift removes or clears exactly that indicated state before propagating the error, and a later valid frame can recover identically to the previous implementation

#### Scenario: Reject an Apple fallback

- **WHEN** the Apple Rust bridge returns an incomplete outcome, semantic failure, or contained internal failure
- **THEN** the adapter follows the fixed status and primitive lengths without executing Swift header, length, noise, CRC, encoder, or exact-decoder logic

#### Scenario: Preserve independent ownership

- **WHEN** the Apple stream-boundary cutover completes
- **THEN** Android continues using its Kotlin framing implementation and neither host transfers Rust allocator ownership or a Rust stream handle across FFI

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

### Requirement: Rust VESC firmware identity candidate parity

The `backends/core` workspace SHALL contain one safe Rust firmware identity component with an empty production dependency graph dedicated to structurally decoding bounded VESC `COMM_FW_VERSION` payloads. It SHALL expose major and minor versions, a hardware name of at most 64 input bytes, a private 12-byte UUID, test-version marker, hardware type, custom-configuration count, an optional firmware name of at most 64 input bytes, and an optional big-endian four-byte hardware-configuration CRC. It SHALL consume but not expose the established pairing and capability bytes and SHALL log no payload, UUID, name, address, token, or full identity.

The decoder SHALL accept the existing audited motor and custom-module tail, SHALL accept hardware type `1` only as exact stock VESC BMS 6.00 with no trailing bytes, and SHALL fail closed with typed unsupported-command or structural/truncation errors for every malformed prefix, missing or overlong terminator, incomplete capability tail, malformed BMS identity, invalid CRC-tail length, or unexpected trailing byte. Invalid UTF-8 in a bounded terminated name SHALL use replacement decoding. The shared Apple backend SHALL select this component for production firmware identity decoding through its audited bridge; Kotlin SHALL remain Android's production owner until a separate Android cutover deletes its superseded parser.

#### Scenario: Decode an audited motor identity

- **WHEN** the Rust component receives a source-derived THOR300 or VESC Express firmware payload with the established motor/custom-module tail
- **THEN** it returns the same version, bounded names, private UUID bytes, test marker, hardware type, custom-configuration count, and optional CRC as the established native behavior

#### Scenario: Decode an audited BMS identity

- **WHEN** the component receives exact stock VBMS 6.00 or VBMS32 6.06 source-derived payload evidence
- **THEN** it preserves the structural distinction between the exact stock-BMS tail and the custom-module tail without applying higher-level BMS compatibility policy

#### Scenario: Reject malformed identity input

- **WHEN** input has the wrong command, any truncated common field, no bounded terminator, an invalid stock-BMS version or tail, incomplete extended fields, an invalid optional CRC length, or trailing bytes
- **THEN** the component returns the matching bounded typed failure and exposes no partial identity

#### Scenario: Preserve private identity

- **WHEN** parity tests exercise meaningful UUID bytes and malformed private input
- **THEN** exact bytes remain available only in the returned in-process model or caller-owned bridge result and do not appear in ordinary output, formatted errors, or repository diagnostics

#### Scenario: Select Apple while preserving Android ownership

- **WHEN** the Apple bridge and production adapter pass focused, packaging, canonical, and physical verification
- **THEN** iOS and macOS select Rust firmware identity decoding while Android continues selecting Kotlin without depending on the Apple bridge

### Requirement: Apple Rust VESC firmware identity cutover

The shared Apple backend SHALL use the safe Rust firmware identity component as its sole production implementation of VESC `COMM_FW_VERSION` structural decoding on iOS and macOS. Rust SHALL decide command validity, common-prefix extent, both 64-byte terminated-name bounds, private UUID extraction, test/type/configuration fields, exact stock-BMS admission, custom-module capability/name extent, optional CRC extent and value, and every structural failure. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `FirmwareInfo`, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift identity parsing, and SHALL delete the superseded Swift parser after required verification. Compatibility admission, route attribution, application behavior, and Android identity ownership SHALL remain outside this change.

#### Scenario: Decode Apple production identity

- **WHEN** shared Apple production code receives a structurally valid direct or CAN-routed firmware payload
- **THEN** the thin Swift adapter obtains every identity field from Rust and executes no Swift VESC prefix, terminator, hardware-tail, or CRC-tail algorithm

#### Scenario: Map Apple production failure

- **WHEN** Rust reports an unsupported command, structural failure, ABI failure, or invalid bounded result
- **THEN** Swift returns the established `VescProtocolError` class without exposing payload, UUID, names, or partial identity

#### Scenario: Preserve higher-level policy

- **WHEN** Rust returns a structurally valid motor, Express, stock-BMS, or VBMS32 identity
- **THEN** existing Swift compatibility, routing, discovery, application, and state owners apply unchanged policy after representation mapping

#### Scenario: Reject an Apple fallback

- **WHEN** bridge decoding fails or the input is malformed
- **THEN** Apple fails closed through the fixed status mapping and does not execute a Swift structural decoder

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple firmware identity cutover completes
- **THEN** Android continues using its Kotlin decoder with unchanged behavior and without linking an Apple artifact
