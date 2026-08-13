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

### Requirement: Android Rust VESC firmware identity cutover

The Android backend SHALL use the safe Rust firmware identity component as its sole production implementation of VESC `COMM_FW_VERSION` structural decoding. Rust SHALL decide command validity, common-prefix extent, both 64-byte terminated-name bounds, private UUID extraction, test/type/configuration fields, exact stock-BMS admission, custom-module capability/name extent, optional CRC extent and value, and every structural failure. Kotlin SHALL perform only bounded versioned bridge-record validation, primitive representation conversion into the existing `FirmwareInfo`, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin identity parsing, and SHALL delete the superseded Kotlin parser after required verification. Compatibility admission, request scheduling, route attribution, application behavior, Apple identity ownership, and all other protocol owners SHALL remain outside this change.

#### Scenario: Decode Android production identity

- **WHEN** Android production code receives a structurally valid direct or CAN-routed firmware payload
- **THEN** the thin Kotlin adapter obtains every identity field from Rust and executes no Kotlin VESC prefix, terminator, hardware-tail, or CRC-tail algorithm

#### Scenario: Map Android production failure

- **WHEN** Rust reports an unsupported command, structural failure, ABI failure, caller-contract failure, or invalid bounded result
- **THEN** Kotlin returns the established `VescProtocolException` class without exposing payload, UUID, names, or partial identity

#### Scenario: Preserve higher-level policy

- **WHEN** Rust returns a structurally valid motor, Express, stock-BMS, or VBMS32 identity
- **THEN** existing Kotlin compatibility, routing, discovery, application, and state owners apply unchanged policy after representation mapping

#### Scenario: Reject an Android identity fallback

- **WHEN** bridge decoding fails or the input is malformed
- **THEN** Android fails closed through the fixed status and validated record mapping without executing a Kotlin structural decoder

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android firmware identity cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking or calling the Android JNI artifact

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

### Requirement: Apple Rust VESC BMS values cutover

The shared Apple backend SHALL use the safe Rust BMS values component as its sole production implementation of `COMM_BMS_GET_VALUES = 96` structural decoding on iOS and macOS. Rust SHALL decide command validity, signed fixed-point fields, cell and temperature counts and entries, balancing flags, reported-CAN validity and route matching, VESC auto-floats, finite numeric values, pressure, exact legacy termination, extended data-version/status structure, UTF-8 validity and bound, remainder consumption, and every structural failure. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `BmsSnapshot`, observation-time attachment, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift BMS payload parsing, and SHALL delete the superseded Swift structural implementation after required verification. Firmware compatibility, BMS discovery, request construction, direct/CAN routing, scheduling, polling, observation time, availability, application state, frontend behavior, and Android BMS ownership SHALL remain outside this change.

#### Scenario: Decode Apple production BMS values

- **WHEN** shared Apple production receives a valid legacy or extended command-96 payload from a direct or CAN-routed BMS
- **THEN** the thin Swift adapter obtains every structural field and collection from Rust and executes no Swift BMS payload, count, flag, auto-float, CAN, status-tail, or remainder algorithm

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an unsupported command, structural failure, invalid caller contract, insufficient output, ABI mismatch, invalid result record, or contained internal failure
- **THEN** Swift returns the established bounded `VescProtocolError` class without publishing a partial snapshot, logging payload values, or executing a fallback parser

#### Scenario: Preserve higher-level BMS owners

- **WHEN** Rust returns structurally valid direct or CAN BMS values
- **THEN** existing Swift compatibility, discovery, routing, scheduling, polling, observation-time, availability, application-state, and live-only owners behave unchanged after representation mapping

#### Scenario: Preserve Android BMS ownership

- **WHEN** Apple completes its BMS values cutover
- **THEN** Android continues using its Kotlin command-96 decoder with unchanged behavior and without linking, packaging, or calling the Apple bridge

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

### Requirement: Rust Refloat control codec candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to encoding bounded Refloat package, flywheel, legacy/stable lighting, and fatal-clear custom-app-data payloads and decoding bounded lighting responses. It SHALL accept typed caller-provided control profile and lighting state values without owning compatibility lookup, VESC framing, CAN routing, mutation authorization, scheduling, confirmation, telemetry, application state, or persistence.

The candidate SHALL preserve VESC command `36`, Refloat package ID `101`, exact Refloat command and parameter bytes, inverted package enable semantics, flywheel mode/default/footpad-abort bytes, lighting profile and mask/flag behavior, zero-mask query behavior, fatal-clear availability, exact four-byte response extent, defined lighting bits, and unsupported-command versus malformed failure behavior. It SHALL return statically bounded payloads, forbid unsafe code, log no payload or state, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Encode exact source-derived controls

- **WHEN** the candidate receives each committed package, flywheel, legacy/stable lighting, or fatal-clear input
- **THEN** it returns the exact source-derived custom-app-data payload selected native codecs produce

#### Scenario: Preserve lighting state and query semantics

- **WHEN** the candidate encodes either lighting profile for every current-state, target, and enabled combination or for no target
- **THEN** it changes only the selected bit, preserves the other bit, uses the exact mask, and emits a zero-mask query without changing the supplied flags

#### Scenario: Decode exact lighting responses

- **WHEN** the candidate receives an exact four-byte response for the supplied legacy or stable profile with any defined two-bit lighting state
- **THEN** it returns the matching LED and headlight booleans

#### Scenario: Reject unavailable or malformed controls

- **WHEN** fatal clear is unavailable or a lighting response has an empty, short, long, wrong-prefix, wrong-package, wrong-command, wrong-profile, or undefined-flag form
- **THEN** the candidate returns the matching bounded unsupported-command or malformed failure and exposes no partial state or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for Refloat live-control encoding and response decoding and no supported-backend or native-code deletion claim is made

### Requirement: Apple Rust Refloat control codec cutover

The shared Apple backend SHALL use the safe Rust Refloat control component as its sole production implementation of package, flywheel, legacy/stable lighting, zero-mask lighting-query, and fatal-clear custom-app-data payload encoding and lighting-response structural decoding on iOS and macOS. Rust SHALL decide the VESC command and Refloat package prefix, control command and parameter bytes, inverted package semantics, flywheel defaults and footpad-abort byte, lighting profile/reserved bytes/mask/flags/query behavior, fatal-clear availability, exact response extent and prefix, defined response bits, and every structural or unsupported failure. Swift MAY perform only bounded versioned bridge validation, primitive model conversion, caller-owned byte copying, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift control-wire algorithms, and SHALL delete the superseded Swift implementation after required verification. Compatibility lookup, route selection, VESC framing, mutation authorization, scheduling, confirmation, telemetry, application state, and Android Refloat control ownership SHALL remain outside this change.

#### Scenario: Encode Apple production controls

- **WHEN** shared Apple production requests any supported package, flywheel, lighting mutation/query, or fatal-clear control
- **THEN** the thin Swift adapter obtains the exact bounded payload from Rust and executes no Swift command, parameter, mask, flag, reserved-byte, query, or fatal-availability algorithm

#### Scenario: Decode Apple production lighting

- **WHEN** shared Apple production receives a valid legacy or stable lighting response
- **THEN** Rust validates its complete structure and Swift mechanically maps the returned two-bit result into the existing lighting state

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an invalid caller contract, malformed response, unsupported command, insufficient output, ABI mismatch, invalid primitive result, or contained internal failure
- **THEN** Swift returns the established bounded protocol error without logging payload/state, publishing a partial result, or executing a fallback codec

#### Scenario: Preserve higher-level Refloat owners

- **WHEN** Rust returns a control payload or decoded lighting state
- **THEN** existing Swift compatibility, routing, framing, authorization, scheduling, confirmation, telemetry, and application-state owners behave unchanged after representation mapping

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple Refloat control cutover completes
- **THEN** Android continues using its Kotlin codec with unchanged behavior and without linking, packaging, or calling the Apple bridge

### Requirement: Android Rust Refloat control codec cutover

The Android backend SHALL use the safe Rust Refloat control component as its sole production implementation of package, flywheel, legacy/stable lighting, zero-mask lighting-query, and fatal-clear custom-app-data payload encoding and lighting-response structural decoding. Rust SHALL decide the VESC command and Refloat package prefix, control command and parameter bytes, inverted package semantics, flywheel defaults and footpad-abort byte, lighting profile/reserved bytes/mask/flags/query behavior, fatal-clear availability, exact response extent and prefix, defined response bits, and every structural or unsupported failure. Kotlin MAY perform only bounded versioned bridge validation, primitive model conversion, caller-owned array copying, and established error mapping.

The cutover SHALL use a narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin control-wire algorithms, and SHALL delete the superseded Kotlin implementation after required verification. Compatibility lookup, route selection, VESC framing, mutation authorization, scheduling, confirmation, telemetry, application state, and Apple Refloat control ownership SHALL remain outside this change.

#### Scenario: Encode Android production controls

- **WHEN** Android production requests any supported package, flywheel, lighting mutation/query, or fatal-clear control
- **THEN** the thin Kotlin adapter obtains the exact bounded payload from Rust and executes no Kotlin command, parameter, mask, flag, reserved-byte, query, or fatal-availability algorithm

#### Scenario: Decode Android production lighting

- **WHEN** Android production receives a valid legacy or stable lighting response
- **THEN** Rust validates its complete structure and Kotlin mechanically maps the returned two-bit result into the existing lighting state

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an invalid caller contract, malformed response, unsupported command, insufficient output, ABI mismatch, invalid primitive result, pending exception, or contained internal failure
- **THEN** Kotlin returns the established bounded protocol exception without logging payload/state, publishing a partial result, or executing a fallback codec

#### Scenario: Preserve higher-level Refloat owners

- **WHEN** Rust returns a control payload or decoded lighting state
- **THEN** existing Kotlin compatibility, routing, framing, authorization, scheduling, confirmation, telemetry, and application-state owners behave unchanged after representation mapping

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android Refloat control cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust codec and Apple bridge without linking, packaging, or calling the Android JNI artifact

### Requirement: Rust VESC controller-data candidate parity

The `backends/core` workspace SHALL contain one safe Rust component with an empty production dependency graph dedicated to decoding the consumed standard VESC `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset. It SHALL accept unframed payloads without owning request construction, firmware compatibility, VESC framing, CAN routing, scheduling, telemetry polling, application state, or persistence.

The component SHALL preserve command validation; big-endian signed fixed-point scales; the consumed 30-byte realtime prefix and permitted later fields; raw fault code and none, over-voltage, or unknown classification; VESC auto-float decoding; four finite nonzero current limits; the closed supported serialization-signature to battery-series-cell-offset table; and optional behavior for unknown signatures, absent offsets, and zero cell counts. It SHALL forbid unsafe code, expose only bounded payload-safe failures, and be selected by Apple and Android through separate audited host bridges.

#### Scenario: Decode exact realtime values

- **WHEN** the component receives each committed normal or faulted command-4 payload
- **THEN** it returns the exact temperatures, currents, duty, ERPM, voltage, raw fault code, and stable fault classification selected native decoders produce

#### Scenario: Preserve consumed-prefix behavior

- **WHEN** a command-4 payload contains exactly the consumed 30-byte prefix or additional standard fields
- **THEN** the component decodes the same selected values without requiring or interpreting the permitted tail

#### Scenario: Decode bounded current limits

- **WHEN** the component receives command 14 with valid VESC auto-float limits and any supported serialization signature
- **THEN** it returns the four exact finite nonzero limits and the positive series-cell byte when the signature-specific offset exists

#### Scenario: Preserve optional cell-count behavior

- **WHEN** the motor configuration has an unknown signature, ends before its recognized cell offset, or contains zero at that offset
- **THEN** the component returns valid limits with no series-cell value rather than inventing a value or rejecting the payload

#### Scenario: Reject malformed controller data

- **WHEN** either payload has the wrong command, truncates a consumed field, contains an invalid auto-float, or produces a zero or non-finite current limit
- **THEN** the component returns the matching bounded unsupported-command or truncated failure and exposes no partial values or payload diagnostic

#### Scenario: Select both host families independently

- **WHEN** the Android bridge and adapter pass focused, packaging, canonical, and physical verification
- **THEN** Android selects Rust controller-data decoding while iOS and macOS preserve their existing Rust selection through the independent Apple bridge

### Requirement: Apple Rust VESC controller-data cutover

The shared Apple backend SHALL use the safe Rust controller-data component as its sole production implementation of the consumed standard `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset on iOS and macOS. Rust SHALL decide command validity, every selected big-endian signed fixed-point scale, signed ERPM, raw fault and classification, consumed-prefix extent, permitted tail behavior, VESC auto-floats, finite nonzero limits, serialization signatures, series-cell offsets, optional cell-count behavior, and every structural or numeric failure. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `RealtimeValues`, `VescFault`, and `ControllerCurrentLimits`, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift controller-data parsing, and SHALL delete the superseded Swift algorithms after required verification. Request construction, firmware compatibility, route attribution, framing, scheduling, telemetry polling, fault response, application behavior, Android controller-data ownership, and every OS capability SHALL remain outside this change.

#### Scenario: Decode Apple production realtime values

- **WHEN** shared Apple production receives a structurally valid direct or CAN-routed command-4 payload
- **THEN** the thin Swift adapter obtains every consumed scalar, ERPM, raw fault, and classification from Rust and executes no Swift byte-reading, scaling, or fault-classification algorithm

#### Scenario: Decode Apple production current limits

- **WHEN** shared Apple production receives a structurally valid command-14 motor configuration
- **THEN** Rust returns all four finite nonzero limits and the optional positive series-cell value while Swift executes no auto-float, signature, offset, or optional-cell algorithm

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an unsupported command, truncated or invalid data, invalid caller contract, insufficient output, ABI mismatch, invalid result record, or contained internal failure
- **THEN** Swift returns the established bounded `VescProtocolError` class without publishing partial values, logging payload data, or executing a fallback parser

#### Scenario: Preserve higher-level controller owners

- **WHEN** Rust returns valid realtime values or current limits
- **THEN** existing Swift compatibility, direct/CAN routing, framing, scheduling, telemetry, fault-response, state-publication, and session owners behave unchanged after representation mapping

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple controller-data cutover completes
- **THEN** Android continues using its Kotlin decoder with unchanged behavior and without linking, packaging, or calling the Apple bridge

### Requirement: Android Rust VESC controller-data cutover

The Android backend SHALL use the safe Rust controller-data component as its sole production implementation of the consumed standard `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset. Rust SHALL decide command validity, every selected big-endian signed fixed-point scale, signed ERPM, raw fault and classification, consumed-prefix extent, permitted tail behavior, VESC auto-floats, finite nonzero limits, serialization signatures, series-cell offsets, optional cell-count behavior, and every structural or numeric failure. Kotlin MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `RealtimeValues`, `VescFault`, and `ControllerCurrentLimits`, and established error mapping.

The cutover SHALL use one narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin controller-data parsing, and SHALL delete the superseded Kotlin algorithms after required verification. Request construction, firmware compatibility, route attribution, framing, scheduling, telemetry polling, fault response, application behavior, Apple controller-data ownership, and every OS capability SHALL remain outside this change.

#### Scenario: Decode Android production realtime values

- **WHEN** Android production receives a structurally valid direct or CAN-routed command-4 payload
- **THEN** the thin Kotlin adapter obtains every consumed scalar, ERPM, raw fault, and classification from Rust and executes no Kotlin byte-reading, scaling, or fault-classification algorithm

#### Scenario: Decode Android production current limits

- **WHEN** Android production receives a structurally valid command-14 motor configuration
- **THEN** Rust returns all four finite nonzero limits and the optional positive series-cell value while Kotlin executes no auto-float, signature, offset, or optional-cell algorithm

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an unsupported command, truncated or invalid data, invalid caller contract, insufficient output, ABI mismatch, invalid result record, pending exception, or contained internal failure
- **THEN** Kotlin returns the established bounded `VescProtocolException` class without publishing partial values, logging payload data, or executing a fallback parser

#### Scenario: Preserve higher-level controller owners

- **WHEN** Rust returns valid realtime values or current limits
- **THEN** existing Kotlin compatibility, direct/CAN routing, framing, scheduling, telemetry, fault-response, state-publication, and session owners behave unchanged after representation mapping

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android controller-data cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking, packaging, or calling the Android JNI artifact

### Requirement: Rust Refloat legacy telemetry candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to decoding complete unframed Refloat custom-app-data command-10 compact telemetry. It SHALL accept the exact active-firmware-fault form and detail levels 1 through 4 without owning VESC framing, direct/CAN routing, compatibility, polling, dynamic field negotiation, application state, persistence, or transport.

The candidate SHALL preserve the VESC custom-app-data command and Refloat package/command identifiers; exact form and detail-level lengths; big-endian signed scales; one metres/second-to-kilometres/hour conversion; finite VESC auto-float distance; legacy package state, mode, footpad, stop, adjustment, alert, and firmware-fault mappings; all fixed and optional detail fields; and raw unknown fault classification. It SHALL expose the source `222` field-weakening sentinel as no value, forbid unsafe code, return allocation-free fixed-size output, expose only bounded payload-safe failures, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Decode every compact detail level

- **WHEN** the candidate receives each committed source-derived command-10 detail-level 1 through 4 payload
- **THEN** it returns every field present at that level with exact units and semantics while fields belonging only to later levels remain absent

#### Scenario: Decode an active firmware fault

- **WHEN** the candidate receives the exact five-byte command-10 fault form with a nonzero known or unknown fault code
- **THEN** it returns the raw code and stable firmware-fault classification without fabricating a normal telemetry sample

#### Scenario: Preserve unavailable field weakening

- **WHEN** a valid compact response carries upstream sentinel byte `222` in the field-weakening position
- **THEN** the candidate returns no field-weakening current rather than reporting the sentinel as a measured value

#### Scenario: Reject malformed legacy telemetry

- **WHEN** a payload has the wrong envelope, package, or command; an unsupported detail level; a zero fault code; an inexact form length; a truncated field; or a non-finite distance value
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial values or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for legacy telemetry decoding and no supported-backend or native-code deletion claim is made

### Requirement: Narrow Apple FFI support ownership

Shared Rust Apple FFI support SHALL own only protocol-neutral C-boundary mechanics common to every selected Apple bridge. VESC and Refloat framing, commands, decoding, encoding, record schemas, status mappings, compatibility, routing, scheduling, application behavior, persistence, transport, and OS capabilities SHALL remain with their existing owners, and Android JNI support SHALL remain independent until a separate change.

#### Scenario: Maintain downward dependencies

- **WHEN** an Apple bridge uses shared boundary support
- **THEN** the dependency graph remains host bridge to support and domain crate, support depends on neither, and no domain or host framework dependency points upward into a bridge

#### Scenario: Add a later Apple bridge

- **WHEN** a later protocol candidate receives an Apple C bridge
- **THEN** it can reuse the bounded support primitives without adding protocol knowledge, exported symbols, headers, generated bindings, or platform behavior to the support crate

#### Scenario: Preserve Android isolation

- **WHEN** Apple FFI support is built, tested, packaged, and executed
- **THEN** Android JNI crates, libraries, symbols, loaders, Gradle inputs, and production calls remain unchanged and do not link the Apple support component

### Requirement: Narrow Android JNI support ownership

Shared Rust Android JNI support SHALL own only protocol-neutral, primitive-array VM-boundary mechanics common to every selected Android bridge. VESC and Refloat framing, commands, decoding, encoding, record schemas, status mappings, compatibility, routing, scheduling, application behavior, persistence, transport, Kotlin host policy, and OS capabilities SHALL remain with their existing owners, and Apple FFI support SHALL remain independent.

#### Scenario: Keep domain and JNI ownership separate

- **WHEN** a selected Android bridge validates a call, invokes a domain component, maps its result, and publishes primitive-array output
- **THEN** shared support performs only call-local JNI safety mechanics while the explicit bridge owns its symbol, ABI, limits, status mapping, record, and publication policy

#### Scenario: Add a later Android bridge

- **WHEN** a later Rust protocol candidate receives an Android JNI bridge
- **THEN** it may reuse the same environment/reference/primitive-array support without moving domain behavior, host callbacks, JVM object lifetime, or application ownership into support

#### Scenario: Preserve OS capability ownership

- **WHEN** Android or Apple performs BLE, background, location, permission, file, theme, lifecycle, WebView, loopback, or persistence work
- **THEN** the retained native host remains the sole owner and neither JNI nor FFI support participates

### Requirement: Shared Rust firmware identity bridge record contract

The safe `vesc-firmware-identity` component SHALL be the single owner of the existing bounded platform-neutral firmware-identity bridge record, including its version, maximum payload and record extents, field/flag layout, replacement-decoded name serialization, optional firmware-name and hardware-configuration-CRC representation, and closed decoder-to-record outcome. The component SHALL remain dependency-free and unsafe-free and MUST expose no C/JNI symbol, pointer, host status, callback, retained reference, platform framework, or generated binding.

The independent Apple FFI and Android JNI bridges SHALL consume that shared result while retaining their existing public symbols, argument shapes, caller validation, fixed status mapping, caller-owned publication, artifacts, and host isolation. Neither bridge SHALL retain a parallel record encoder, record-layout constant set, or domain-error outcome implementation.

#### Scenario: Produce the existing canonical identity record

- **WHEN** the shared record operation receives any audited motor, Express, stock-BMS, or VBMS32 firmware payload
- **THEN** it returns the exact existing bounded record bytes for every version, name, UUID, test marker, hardware type, configuration count, optional firmware name, and optional CRC field

#### Scenario: Preserve portable failure classification

- **WHEN** the operation receives a wrong command, malformed or truncated identity, or an impossible constructed identity representation
- **THEN** it returns the matching closed unsupported-command, structural, or internal outcome without a partial record or private payload diagnostic

#### Scenario: Select the same result through both bridges

- **WHEN** Apple and Android decode the same accepted or rejected identity through their compiled production bridge
- **THEN** each bridge maps the one shared outcome to its unchanged status and caller-owned result without executing bridge-local serialization or structural decoding

#### Scenario: Preserve safe component isolation

- **WHEN** repository verification inspects the safe component and both host bridges
- **THEN** the component remains dependency-free and unsafe-free, the bridges remain independent platform artifacts, and no generated binding, macro-generated export, runtime, callback, or host type crosses the dependency boundary

### Requirement: Apple Rust Refloat legacy telemetry cutover

The shared Apple backend SHALL use `refloat-legacy-telemetry` as its sole production implementation of fixed Refloat custom-app-data command-10 detail levels 1 through 4 and the active-firmware-fault form on iOS and macOS. Rust SHALL decide the exact envelope/form/length, every signed scale and unit conversion, finite distance, package state and mode, footpad, stop, setpoint, alert and firmware-fault classifications, boolean state flags, every optional detail tail, and field-weakening sentinel behavior. Swift MAY perform only strict versioned-record validation, representation conversion into the existing normalized telemetry model, established error mapping, and higher-level behavior.

The safe component SHALL own one dependency-free, unsafe-free, allocation-free canonical record and closed outcome for later platform reuse. The temporary Apple bridge SHALL publish that record through caller-owned memory with no Rust allocation, reference, callback, object, or lifetime crossing C. The cutover MUST delete superseded Swift command-10 wire parsing and MUST NOT add a runtime or source fallback. Android, dynamic telemetry, compatibility, routing, polling, cache merging, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Decode every Apple compact detail level

- **WHEN** Apple production receives a valid direct or CAN-routed command-10 detail-level 1 through 4 payload
- **THEN** Rust supplies every exact normalized field and optional absence while Swift executes no command-10 length, byte-reading, scaling, semantic-state, enum, or sentinel algorithm

#### Scenario: Decode an Apple active firmware fault

- **WHEN** Apple production receives the exact command-10 active-firmware-fault form with a nonzero known or unknown code
- **THEN** Rust supplies the raw and classified fault through the canonical record and Swift constructs the established unknown-ride-state firmware-fault telemetry without classifying payload bytes

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an unsupported envelope, truncated or invalid payload, invalid caller contract, insufficient output, ABI mismatch, malformed record, or contained internal failure
- **THEN** Swift returns the established bounded protocol error without partial telemetry, payload-bearing diagnostics, or a native fallback decoder

#### Scenario: Preserve higher-level Apple owners

- **WHEN** Rust returns a valid legacy telemetry record
- **THEN** existing compatibility, route attribution, framing, scheduling, polling, cache merging, state publication, session logging, and application behavior continue unchanged

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple legacy-telemetry cutover completes
- **THEN** Android continues through its Kotlin command-10 decoder without linking, packaging, loading, or calling the Apple bridge

#### Scenario: Retire temporary protocol bridges later

- **WHEN** a future unified Rust application runtime owns protocol and application behavior on every host
- **THEN** the per-protocol Apple and Android legacy-telemetry bridges are removed rather than retained as a permanent API layer

### Requirement: Android Rust Refloat legacy telemetry cutover

The Android backend SHALL use `refloat-legacy-telemetry` as its sole production implementation of fixed Refloat custom-app-data command-10 detail levels 1 through 4 and the active-firmware-fault form. Rust SHALL decide the exact envelope, form, and length; every signed scale and unit conversion; finite distance; package state and mode; footpad, stop, setpoint, alert, and firmware-fault classifications; boolean state flags; every optional detail tail; and field-weakening sentinel behavior. Kotlin SHALL perform only strict versioned-record validation, representation conversion into the existing normalized telemetry model, established error mapping, and higher-level behavior.

#### Scenario: Decode every Android legacy form through Rust

- **WHEN** Android production receives any valid command-10 detail-level or active-fault payload
- **THEN** its selected JNI path returns the canonical safe-component record and Kotlin publishes the exact established normalized telemetry result

#### Scenario: Preserve canonical unavailable values

- **WHEN** a payload omits a detail tail or carries the field-weakening sentinel
- **THEN** the canonical record and Android model retain absence rather than fabricating numeric zero or a sentinel measurement

#### Scenario: Fail closed without a Kotlin fallback

- **WHEN** the linked ABI, JNI call, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, or canonical absent values are invalid
- **THEN** Android returns its established bounded protocol failure without interpreting command-10 bytes in Kotlin or exposing payload data

#### Scenario: Preserve higher-level Android owners

- **WHEN** Rust returns a valid legacy telemetry record
- **THEN** existing Kotlin compatibility, routing, framing, scheduling, compact-cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their existing safe Rust decoder and Apple bridge without linking, loading, or calling the Android JNI artifact

#### Scenario: Retire the temporary bridge with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Android JNI and Apple FFI legacy-telemetry crates and host record adapters are deleted rather than retained as permanent APIs

### Requirement: Rust Refloat dynamic telemetry candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to decoding complete unframed Refloat custom-app-data command-32 layouts and command-31 dynamic telemetry. It SHALL own bounded negotiated layout representation, exact mask-dependent response structure, the source float16 format, semantic state and alert classifications, typed known fields, and bounded unknown numeric extensions without owning VESC framing, compatibility selection, direct/CAN routing, polling, layout lifetime across reconnect, compact-cache merging, application state, persistence, or transport.

The candidate SHALL accept at most 64 globally unique nonempty UTF-8 identifiers, at most 64 encoded bytes per identifier, at most 4,096 aggregate identifier bytes, and at most 32 unknown extensions. It SHALL decode the source format's complete finite `+-131008` float16 range, preserve negotiated order, source ticks, running-only values, charging data, the 1.2.x two-word active-alert tail, fatal latch, raw firmware-fault code, and stable classifications, return no partial layout or sample, expose only bounded payload-safe failures, forbid unsafe code, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Decode a complete bounded layout

- **WHEN** the candidate receives a source-derived or future-bounded command-32 layout
- **THEN** it returns the exact ordered always-present and running-only identifiers only after validating the complete envelope, UTF-8, extents, aggregate bound, count bound, and global uniqueness

#### Scenario: Decode every dynamic response form

- **WHEN** the candidate receives a valid 1.1.x or 1.2.x command-31 response against its accepted layout
- **THEN** it returns exact source ticks, semantic state, all negotiated known values, bounded extensions, running-only presence, charging values, active alerts, fatal latch, and firmware-fault information required by that form

#### Scenario: Preserve the complete source float16 range

- **WHEN** a negotiated value uses zero, a subnormal, a normal, exponent `0x1f`, or either finite saturation endpoint
- **THEN** the candidate returns the source-defined finite value with the correct sign and does not apply IEEE infinity or NaN semantics

#### Scenario: Reject malformed dynamic telemetry

- **WHEN** a layout or realtime payload has the wrong envelope, invalid UTF-8, empty or duplicate name, excessive bound, unsupported mask, inexact extent, impossible alert/fault relation, or excessive extension set
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial identifiers, values, or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for dynamic layout and telemetry decoding and no supported-backend or native-code deletion claim is made

### Requirement: Apple Rust Refloat dynamic telemetry cutover

The shared Apple backend SHALL use `refloat-dynamic-telemetry` as its sole production implementation of Refloat command-32 layout and command-31 realtime decoding on iOS and macOS. Rust SHALL decide exact envelopes, UTF-8 identifiers, negotiated ordering and bounds, mask-dependent extents, complete finite source float16 values, source ticks, semantic state, behavior flags, charging values, active-alert words, fatal latch, firmware-fault classification, known fields, and bounded ordered extensions before publishing canonical versioned records. Swift SHALL only validate and mechanically map those records into existing layout and telemetry models.

The selected boundary SHALL be stateless across calls: Apple SHALL retain only the canonical bounded layout record in its existing connection generation, clear it on every reset or replacement, and supply it to Rust for each realtime decode. No Swift command-31/32 wire parser, float16 decoder, semantic mapping, fallback, Rust opaque handle, allocation, reference, or callback SHALL remain across the boundary. Compatibility, request construction, framing, direct/CAN routing, polling, cache merging, observation time, application state, persistence, transport, and OS capabilities SHALL remain with their existing owners.

#### Scenario: Negotiate and retain one layout

- **WHEN** Apple receives a valid command-32 response for the current connection generation
- **THEN** Rust returns the exact canonical layout record, Swift publishes the existing ordered field lists, and the same bounded record is retained only until reset or replacement

#### Scenario: Decode realtime through the retained layout

- **WHEN** Apple receives a valid command-31 response after layout negotiation
- **THEN** Rust validates the canonical layout and payload and returns one complete canonical sample that Swift mechanically maps without interpreting raw command bytes

#### Scenario: Fail closed without a Swift fallback

- **WHEN** the ABI, caller regions, layout record, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, alert/fault relation, extension entries, or canonical absent values are invalid
- **THEN** Apple returns its established bounded protocol failure without partial state, payload diagnostics, stale-layout reuse, or native wire decoding

#### Scenario: Preserve the complete source float16 range

- **WHEN** production Apple telemetry contains zero, signed zero, a subnormal, a normal, exponent `0x1f`, or either finite source saturation endpoint
- **THEN** Rust publishes the exact finite value and Swift does not impose IEEE infinity or NaN semantics on the source format

#### Scenario: Preserve higher-level Apple owners

- **WHEN** Rust returns a valid layout or realtime record
- **THEN** existing Apple compatibility, routing, framing, scheduling, polling, connection lifetime, cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple cutover builds, packages, and executes
- **THEN** Android continues using its Kotlin dynamic decoder without linking, loading, packaging, or calling the Apple bridge

#### Scenario: Retire the temporary bridge with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Apple FFI and Android JNI dynamic-telemetry crates and host record adapters are deleted rather than retained as permanent APIs

### Requirement: Apple Rust Refloat compatibility cutover

The shared Apple backend SHALL use `refloat-compatibility` as its sole production implementation of the exact Refloat INFO request, bounded INFO identity decoding, supported-release/profile selection, and required dynamic-layout admission on iOS and macOS. Rust SHALL atomically decode INFO and select one closed compatibility result containing structural wire identity, telemetry profile/family, lighting profile, and fatal-clear availability. Swift SHALL perform only strict canonical-record validation, representation conversion into the existing models, established error mapping, and higher-level behavior.

The safe compatibility component SHALL own one fixed dependency-free-of-third-parties, unsafe-free canonical record and SHALL admit the existing safe dynamic-telemetry layout record without a second host serialization. The temporary Apple bridge SHALL publish caller-owned records with no Rust allocation, reference, callback, object, or lifetime crossing C. The cutover MUST delete superseded Swift INFO offsets, catalogue entries, profile derivation, required-name constants, set admission, and runtime fallback. Android, controller discovery effects, routing, scheduling, telemetry decoding, controls, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Construct and decode Apple INFO through Rust

- **WHEN** Apple production requests INFO or receives a valid direct or CAN-routed legacy or version-2 INFO response
- **THEN** Rust supplies the exact request and one complete accepted compatibility record while Swift executes no INFO byte-offset or catalogue/profile algorithm

#### Scenario: Admit Apple dynamic layout through Rust

- **WHEN** Apple production decodes a command-32 layout for the current connection generation
- **THEN** Rust compatibility validates the canonical Rust-owned layout record and decides required-field admission without Swift required-name constants, set logic, or a second layout serialization

#### Scenario: Fail closed without a Swift fallback

- **WHEN** the ABI, caller regions, payload, INFO protocol/release, compatibility record, profile codes, reserved bytes, canonical layout record, or required groups are invalid
- **THEN** Apple returns its established bounded failure or unsupported state without partial compatibility, payload diagnostics, stale admission, or native protocol-policy fallback

#### Scenario: Preserve higher-level Apple owners

- **WHEN** Rust returns a valid compatibility decision or layout admission
- **THEN** existing Apple discovery effects, route attribution, framing, scheduling, connection lifetime, polling, state publication, sessions, controls, persistence, and application behavior remain unchanged

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple cutover builds, packages, and executes
- **THEN** Android continues using its Kotlin INFO and compatibility implementation without linking, packaging, loading, or calling the Apple bridge

#### Scenario: Retire the temporary bridge with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the Apple compatibility FFI and corresponding later Android JNI crate and host record adapters are deleted rather than retained as permanent APIs

### Requirement: Rust Refloat compatibility candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to constructing the unframed Refloat INFO request, decoding bounded INFO response identity, selecting exact audited compatibility and control profiles, and admitting required dynamic telemetry identifiers. It SHALL own no VESC framing, direct/CAN routing, request scheduling, connection generation, discovery state, telemetry value decoding, application state, persistence, transport, or OS capability.

The candidate SHALL preserve command `36`, package ID `101`, INFO command `0`, requested protocol version `2`, zero request flags, legacy compact wire identity without an invented patch, version-2 major/minor/patch offsets, the exact source-release catalogue from 1.0.0 through 1.2.3, closed legacy/dynamic/dynamic-alert telemetry profiles, legacy/stable lighting profiles, fatal-clear availability, and the exact required always-present and running-only dynamic identifiers. It SHALL enforce the existing 4,096-byte payload bound and bounded layout names, return no partial result, expose only payload-safe failures, forbid unsafe code, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Construct and decode source INFO forms

- **WHEN** the candidate constructs an INFO request or decodes a valid minimal/source legacy compact or complete version-2 response
- **THEN** it returns the exact request bytes and structural wire identity, preserving legacy `1.0` without fabricating patch `.0`

#### Scenario: Select every audited profile exactly

- **WHEN** the candidate looks up any audited wire identity for source releases 1.0.0–1.0.1, 1.1.0–1.1.2, or 1.2.0–1.2.3
- **THEN** it returns the exact source entries, telemetry family, lighting profile, and fatal-clear availability without range inference

#### Scenario: Admit a complete bounded dynamic layout

- **WHEN** both identifier groups contain every exact required name plus any bounded unique future names in any order
- **THEN** the candidate accepts the layout without moving value decoding, layout lifetime, or readiness state into the component

#### Scenario: Reject unsupported or malformed compatibility input

- **WHEN** an INFO payload has a wrong envelope, unsupported protocol byte, truncated version-2 prefix, excessive extent, or a layout has invalid bounds, duplicates, a missing required identifier, or an identifier in the wrong group
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial identity, profile, layout decision, or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for INFO decoding and compatibility admission and no supported-backend or native-code deletion claim is made

### Requirement: Android Rust Refloat dynamic telemetry cutover

The Android backend SHALL use `refloat-dynamic-telemetry` as its sole production implementation of Refloat command-32 layout and command-31 realtime decoding. Rust SHALL decide exact envelopes, UTF-8 identifiers, negotiated ordering and bounds, mask-dependent extents, complete finite source float16 values, source ticks, semantic state, behavior flags, charging values, active-alert words, fatal latch, firmware-fault classification, known fields, and bounded ordered extensions before publishing canonical versioned records. Kotlin SHALL only validate and mechanically map those records into existing layout and telemetry models.

The selected boundary SHALL be stateless across calls: Android SHALL retain only the canonical bounded layout record in its existing connection generation, clear it on every reset, replace it only after a completely valid new layout, and supply it to Rust for each realtime decode. No Kotlin command-31/32 wire parser, float16 decoder, semantic mapping, fallback, Rust opaque handle, allocation, JVM reference, or callback SHALL remain across the boundary. Compatibility, request construction, framing, direct/CAN routing, polling, cache merging, observation time, application state, persistence, transport, and OS capabilities SHALL remain with their existing owners.

#### Scenario: Negotiate and retain one layout

- **WHEN** Android receives a valid command-32 response for the current connection generation
- **THEN** Rust returns the exact canonical layout record, Kotlin publishes the existing ordered field lists, and the same bounded record is retained only until reset or valid replacement

#### Scenario: Reject a layout replacement atomically

- **WHEN** Android receives a malformed command-32 response after accepting a layout
- **THEN** the replacement fails without partially changing the accepted field lists or canonical record

#### Scenario: Decode realtime through the retained layout

- **WHEN** Android receives a valid command-31 response after layout negotiation
- **THEN** Rust validates the canonical layout and payload and returns one complete canonical sample that Kotlin mechanically maps without interpreting raw command bytes

#### Scenario: Fail closed without a Kotlin fallback

- **WHEN** the ABI, JVM inputs, layout record, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, alert/fault relation, extension entries, or canonical absent values are invalid
- **THEN** Android returns its established bounded protocol failure without partial state, payload diagnostics, stale-layout reconstruction, or native wire decoding

#### Scenario: Preserve the complete source float16 range

- **WHEN** production Android telemetry contains zero, signed zero, a subnormal, a normal, exponent `0x1f`, or either finite source saturation endpoint
- **THEN** Rust publishes the exact finite value and Kotlin does not impose IEEE infinity or NaN semantics on the source format

#### Scenario: Preserve higher-level Android owners

- **WHEN** Rust returns a valid layout or realtime record
- **THEN** existing Android compatibility, routing, framing, scheduling, polling, connection lifetime, cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their selected Apple bridge without linking, loading, packaging, or calling the Android JNI bridge

#### Scenario: Retire the temporary bridges with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Apple FFI and Android JNI dynamic-telemetry crates and host record adapters are deleted rather than retained as permanent APIs

### Requirement: Android Rust Refloat compatibility cutover

The Android backend SHALL use `refloat-compatibility` as its sole production implementation of the exact Refloat INFO request, bounded INFO identity decoding, supported-release/profile selection, and required dynamic-layout admission. Rust SHALL atomically decode INFO and select one closed compatibility result containing structural wire identity, telemetry profile/family, lighting profile, and fatal-clear availability. Kotlin SHALL perform only strict canonical-record validation, representation conversion into the existing models, established error mapping, and higher-level behavior.

The safe compatibility component SHALL remain the sole owner of its fixed canonical record and SHALL admit the existing safe dynamic-telemetry layout record without a second host serialization. The temporary Android bridge SHALL use JVM-owned primitive arrays and retain no Rust allocation, Java reference, callback, object, or lifetime across calls. The cutover MUST delete superseded Kotlin INFO offsets, catalogue entries, profile derivation, required-name constants, set admission, and runtime fallback. Apple, controller discovery effects, routing, scheduling, telemetry decoding, controls, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Construct and decode Android INFO through Rust

- **WHEN** Android production requests INFO or receives a valid direct or CAN-routed legacy or version-2 INFO response
- **THEN** Rust supplies the exact request and one complete accepted compatibility record while Kotlin executes no INFO byte-offset or catalogue/profile algorithm

#### Scenario: Admit Android dynamic layout through Rust

- **WHEN** Android production decodes a command-32 layout for the current connection generation
- **THEN** Rust compatibility validates the canonical Rust-owned layout record and decides required-field admission without Kotlin required-name constants, set logic, or a second layout serialization

#### Scenario: Fail closed without a Kotlin fallback

- **WHEN** the ABI, JVM arrays, payload, INFO protocol/release, compatibility record, profile codes, reserved bytes, canonical layout record, or required groups are invalid
- **THEN** Android returns its established bounded failure or unsupported state without partial compatibility, payload diagnostics, stale admission, or native protocol-policy fallback

#### Scenario: Preserve higher-level Android owners

- **WHEN** Rust returns a valid compatibility decision or layout admission
- **THEN** existing Android discovery effects, route attribution, framing, scheduling, connection lifetime, polling, state publication, sessions, controls, persistence, and application behavior remain unchanged

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their selected Apple bridge without linking, packaging, loading, or calling the Android JNI bridge

#### Scenario: Retire the temporary bridges with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the Apple compatibility FFI and Android compatibility JNI crates and host record adapters are deleted rather than retained as permanent APIs

### Requirement: Rust Refloat configuration codec candidate parity

The bounded Rust candidate SHALL independently reproduce the selected native Refloat configuration schema and value semantics without becoming a supported backend or production runtime. It SHALL decode the exact Qt-compressed and expanded schema forms, validate and normalize the closed field/group model, calculate the canonical source fingerprint and wire signature, and decode or encode complete typed value sets with the same bounded deterministic results as the audited Swift and Kotlin owners. It MUST return closed payload-free failures, expose no partial result, forbid unsafe code in candidate-owned source, and retain no raw schema or configuration bytes after normalization.

#### Scenario: Decode every audited schema

- **WHEN** the candidate receives each immutable source-derived Refloat 1.0.0–1.2.3 Qt-compressed schema
- **THEN** it produces the exact fingerprint, signature, ordered fields, supported types and encodings, defaults, bounds, scale, choices, sanitized descriptions, groups, and fixture counts

#### Scenario: Round-trip every audited value family

- **WHEN** the candidate decodes each release's default, current, accepted-write, and expected-readback bytes against its exact schema
- **THEN** it validates the signature and complete serialized order and re-encodes the exact canonical bytes without a remainder or fabricated value

#### Scenario: Reject hostile schema input

- **WHEN** a compressed envelope, zlib stream, XML document, field definition, serialization order, grouping structure, metadata value, or resource extent violates a canonical bound or invariant
- **THEN** the candidate returns the corresponding closed failure without input-bearing diagnostics, partial schema publication, unbounded allocation, entity expansion, or host callback

#### Scenario: Reject hostile values

- **WHEN** values have a mismatched signature, truncation, remainder, invalid UTF-8 or terminator, non-finite or out-of-bounds number, unencodable fixed point, invalid enum/bitfield/boolean, wrong type, or missing or extra field
- **THEN** decode or encode fails before any partial value set or write payload is published

#### Scenario: Preserve production isolation

- **WHEN** the candidate is added to the locked workspace and complete software verification runs
- **THEN** Swift and Kotlin remain the only selected production codec owners and no Apple FFI, Android JNI, bridge catalogue entry, native package dependency, generated binding, exported symbol, or packaged candidate artifact exists

### Requirement: Bounded safe Refloat configuration dependencies

The candidate SHALL use only locked maintained safe Rust dependencies for zlib decompression, streaming XML processing, and SHA-256 hashing. The build SHALL use a pure-Rust decompression backend, SHALL work through the repository's offline locked verification path, and SHALL audit the exact direct dependency set and transitive graph. Candidate-owned source MUST implement no decompressor, XML tokenizer, cryptographic hash, host ABI, filesystem access, networking, scheduling, persistence, or OS integration.

#### Scenario: Verify locked dependency ownership

- **WHEN** workspace dependency and unsafe-code audits inspect the candidate and its locked graph
- **THEN** only the approved decompression, XML, and SHA-256 libraries and their audited transitive dependencies are present, candidate source forbids unsafe code, and no native zlib or platform framework is linked

#### Scenario: Build and test offline

- **WHEN** focused and complete repository gates run without dependency resolution or network access
- **THEN** the exact locked candidate builds, lints with warnings denied, and passes all source-derived and adversarial tests reproducibly

### Requirement: Apple Rust Refloat configuration codec cutover

The shared Apple backend SHALL select the bounded Rust Refloat configuration codec as its sole owner of compressed schema decoding, normalized schema fingerprint and wire-signature construction, typed current/default value decoding, and typed write-value encoding, while retaining stateful configuration and application behavior in Swift until their separate migrations.

#### Scenario: Decode and retain one canonical schema

- **WHEN** the Apple configuration service completes a bounded compatible Refloat schema transfer or selects its cached compressed schema
- **THEN** the production Rust codec SHALL validate and decode it into a canonical schema record
- **AND** Swift SHALL mechanically project that record into its existing normalized schema model
- **AND** the service SHALL retain the exact record with that schema for the active connection generation

#### Scenario: Decode current and default values with the selected schema

- **WHEN** Apple receives current or default configuration value bytes
- **THEN** the production Rust codec SHALL validate the retained schema record and decode every typed field into a canonical value record
- **AND** Swift SHALL mechanically project that record without owning field layout, scalar decoding, scale conversion, string framing, or CRC semantics

#### Scenario: Encode a configuration write with the selected schema

- **WHEN** Apple schedules a validated configuration write
- **THEN** Swift SHALL mechanically describe the complete typed value set in the canonical value record
- **AND** Rust SHALL reject any stale, duplicate, incomplete, mistyped, non-canonical, or out-of-bounds record
- **AND** Rust alone SHALL encode the accepted Refloat wire payload and checksum

#### Scenario: Fail closed without a Swift codec fallback

- **WHEN** ABI validation, bridge validation, record validation, schema decode, value decode, or value encode fails
- **THEN** Apple SHALL map the bounded failure to its existing closed configuration error behavior
- **AND** it SHALL NOT retry any superseded Swift XML, fingerprint, signature, scalar, framing, or checksum implementation

#### Scenario: Preserve stateful Apple ownership

- **WHEN** the stateless codec is selected
- **THEN** Swift SHALL retain ownership of compatibility catalogue selection, schema chunk assembly, cache policy and storage, connection generations, revisions, conflicts, flywheel rules, scheduler interaction, write/readback verification, Tune persistence, application publication, and frontend contract handling
- **AND** the Rust codec and bridge SHALL retain no host or connection state

#### Scenario: Preserve independent Android ownership

- **WHEN** this Apple cutover is built and verified
- **THEN** Android SHALL continue selecting its existing Kotlin configuration codec
- **AND** no Android JNI bridge, Gradle task, package, or artifact SHALL be added for this Apple-only change

#### Scenario: Retire the migration bridge later

- **WHEN** a future change moves the configuration and application service behind the unified Rust runtime
- **THEN** it SHALL delete this temporary Apple bridge and canonical-record adapter rather than preserve them as an internal permanent API

### Requirement: Android Rust Refloat configuration codec cutover

The Android backend SHALL select the bounded Rust Refloat configuration codec as its sole owner of compressed schema decoding, normalized schema fingerprint and wire-signature construction, typed current/default value decoding, and typed write-value encoding, while retaining stateful configuration and application behavior in Kotlin until their separate migrations.

#### Scenario: Decode and retain one canonical schema

- **WHEN** the Android configuration service completes a bounded compatible Refloat schema transfer or selects its cached compressed schema
- **THEN** the production Rust codec SHALL validate and decode it into the canonical schema record
- **AND** Kotlin SHALL mechanically project that record into its existing normalized schema model
- **AND** the service SHALL retain the exact record with that schema for the active connection generation

#### Scenario: Decode current and default values with the selected schema

- **WHEN** Android receives current or default configuration value bytes
- **THEN** the production Rust codec SHALL validate the retained schema record and decode every typed field into the canonical value record
- **AND** Kotlin SHALL mechanically project that record without owning field layout, scalar decoding, scale conversion, string framing, or CRC semantics

#### Scenario: Encode a configuration write with the selected schema

- **WHEN** Android schedules a validated configuration write
- **THEN** Kotlin SHALL mechanically describe the complete typed value set in the canonical value record
- **AND** Rust SHALL reject any stale, duplicate, incomplete, mistyped, non-canonical, or out-of-bounds record
- **AND** Rust alone SHALL encode the accepted Refloat wire payload and checksum

#### Scenario: Fail closed without a Kotlin codec fallback

- **WHEN** ABI validation, JNI validation, record validation, schema decode, value decode, or value encode fails
- **THEN** Android SHALL map the bounded failure to its existing closed configuration error behavior
- **AND** it SHALL NOT retry any superseded Kotlin XML, fingerprint, signature, scalar, framing, or checksum implementation

#### Scenario: Preserve stateful Android ownership

- **WHEN** the stateless codec is selected
- **THEN** Kotlin SHALL retain ownership of compatibility catalogue selection, schema chunk assembly, cache policy and storage, connection generations, revisions, conflicts, flywheel rules, scheduler interaction, write/readback verification, Tune persistence, application publication, and frontend contract handling
- **AND** the Rust codec and bridge SHALL retain no host or connection state

#### Scenario: Preserve selected Apple ownership

- **WHEN** this Android cutover is built and verified
- **THEN** iOS and macOS SHALL continue selecting their existing Rust codec and Apple bridge
- **AND** no Apple C ABI, adapter, package, archive, symbol, or behavior SHALL change

#### Scenario: Retire both migration bridges later

- **WHEN** a future change moves configuration and application behavior behind the unified Rust runtime
- **THEN** it SHALL delete the temporary Android and Apple codec bridges and both canonical-record adapters rather than preserve them as permanent internal APIs
