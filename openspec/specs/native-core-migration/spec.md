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

### Requirement: Rust Refloat configuration service candidate parity

The bounded Rust candidate SHALL independently reproduce the selected native Refloat configuration lifecycle and safety decisions above the existing Rust codec without becoming a supported backend or production runtime. It SHALL own the audited release catalogue, schema paging and assembly, cache admission decisions, generation and operation matching, flywheel restrictions, current/default revisions, write construction, acknowledgement-to-verification sequencing, normalized readback comparison, conflict classification, failure recovery, and reset behavior through deterministic platform-neutral facts, events, effects, and snapshots.

#### Scenario: Begin from cache miss or valid cache

- **WHEN** begin facts describe an eligible direct or CAN controller generation with no cache, a valid audited cached schema, an unaudited cached schema, or corrupt cached bytes
- **THEN** the candidate SHALL deterministically select bounded schema paging or the exact current-value request
- **AND** it SHALL emit the exact store/remove/no-cache decision without reading or writing host storage itself

#### Scenario: Assemble and admit an audited schema

- **WHEN** exact operation- and generation-matched schema chunks arrive in order within every declared extent
- **THEN** the candidate SHALL assemble no more than the codec bound, decode through the existing safe codec, resolve the exact reported release and full fingerprint/signature, emit a cache-store decision where eligible, and request current values unless flywheel mode forbids it

#### Scenario: Read current and default values

- **WHEN** matched current or default value bytes arrive against the selected schema
- **THEN** the candidate SHALL decode them through the safe codec, increment one monotonic revision, publish the exact source and complete typed values, and clear only the completed operation

#### Scenario: Verify one configuration write

- **WHEN** a writable normal-mode service receives an exact identity, base revision, and complete typed value set followed by a matching acknowledgement and authoritative readback
- **THEN** the candidate SHALL encode through the safe codec, sequence write then verification effects under one operation, normalize only the audited non-legacy metadata field, publish the verified current revision, and return success only for an exact normalized match

#### Scenario: Enforce state and flywheel safety

- **WHEN** callers request reads or writes while unavailable, loading, busy, stale, read-only, identity-mismatched, revision-mismatched, or in flywheel mode
- **THEN** the candidate SHALL return the exact closed native-equivalent failure without emitting host work or partial state
- **AND** entering flywheel SHALL invalidate current values and cancel only an automatic current read while leaving schema/default state bounded and intact
- **AND** exiting flywheel SHALL request one fresh current read only when the connection remains eligible and idle

#### Scenario: Reject stale, malformed, and adversarial sequences

- **WHEN** chunks, values, acknowledgements, verification responses, failures, aborts, generations, operations, routes, indexes, commands, lengths, offsets, or ordering violate the active transition
- **THEN** the candidate SHALL fail closed with a stable payload-free error, preserve bounded valid state where specified, and never publish partial schema, values, payload, cache bytes, or effects

#### Scenario: Reset and recover deterministically

- **WHEN** an active operation fails, aborts, reconnects, or resets across generation and operation-counter boundaries
- **THEN** the candidate SHALL match native availability/current/pending recovery semantics, reject all late work, clear connection-scoped data, and produce the same snapshot and effects on repeated equivalent sequences

#### Scenario: Preserve production isolation

- **WHEN** the candidate is added to the locked workspace and complete verification runs
- **THEN** Swift and Kotlin SHALL remain the selected production service owners and no Apple FFI, Android JNI, bridge catalogue entry, package dependency, generated binding, exported symbol, persistent format, or production call path SHALL change

#### Scenario: Retire both migration bridges later

- **WHEN** a future change moves configuration and application behavior behind the unified Rust runtime
- **THEN** it SHALL delete the temporary Android and Apple codec bridges and both canonical-record adapters rather than preserve them as permanent internal APIs
### Requirement: Rust VESC request scheduler candidate parity

The repository SHALL provide a safe, bounded Rust candidate for the complete connection-wide VESC request scheduling state machine while Apple and Android retain their selected native schedulers. The candidate SHALL preserve priority and FIFO arbitration, one response-bearing request in flight, one-way completion, typed response attribution, queue limits, generations, cancellation, explicit deadline expiry, deferred pumping, stable outcomes, and ordered host effects without depending on host execution.

#### Scenario: Arbitrate bounded work
- **WHEN** requests of every priority and equal-priority sequence are enqueued around one in-flight request
- **THEN** the candidate SHALL preserve the non-preemptive in-flight request, select queued work by highest priority then FIFO order, and reject only work beyond the exact bounded queue capacity

#### Scenario: Attribute a response exactly
- **WHEN** a response, route, Refloat command, or typed operation context does not match the current request and generation
- **THEN** the candidate SHALL leave that request in flight and SHALL NOT complete or advance unrelated work

#### Scenario: Replace a connection generation
- **WHEN** a new generation begins with queued or in-flight work and stale responses or deadlines arrive afterward
- **THEN** the candidate SHALL cancel the old work exactly once, reject old-generation enqueue attempts, and prevent stale events from affecting new-generation work

#### Scenario: Complete one-way and response work in order
- **WHEN** one-way, response-bearing, cancelled, timed-out, queue-full, and deferred requests transition
- **THEN** the candidate SHALL emit the same stable completion outcomes and ordered deadline, completion, diagnostic, and next-write effects as the source native behavior

#### Scenario: Remain total at adversarial bounds
- **WHEN** payload, queue, route, timing, sequence, generation, and operation values reach valid boundaries or malformed values exceed them
- **THEN** the candidate SHALL remain bounded and deterministic, use explicit wrapping only for source-equivalent counters, and reject invalid facts without panicking or partially mutating accepted state

### Requirement: Scheduler candidate remains below host execution

The Rust scheduler candidate SHALL expose only safe Rust facts, commands, snapshots, and ordered effects. It MUST NOT own or call a clock, timer, Bluetooth writer, callback, foreign handle, protocol codec, persistence port, transport, frontend, thread, filesystem, network socket, or OS framework.

#### Scenario: Emit host work without executing it
- **WHEN** a transition requires bytes to be written, a deadline to change, a diagnostic to be recorded, or a request to complete
- **THEN** the candidate SHALL return bounded typed effects in the required order and retain no host reference

#### Scenario: Add a later unified runtime
- **WHEN** later changes compose protocol and application services around the scheduler
- **THEN** they SHALL use the safe Rust state API directly and SHALL NOT require a protocol-specific Apple FFI or Android JNI scheduler bridge

### Requirement: Rust Refloat telemetry service candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated Refloat telemetry application service while Apple and Android retain their selected native services. The candidate SHALL compose the existing Rust legacy/dynamic decoded telemetry models and request-scheduler facts to own layout retry, route-correct request construction, legacy/realtime/compact selection, pending-response attribution, timeout recovery, adaptive cadence, compact-cache merging, normalized samples, session sequence and elapsed time, and ordered host effects.

#### Scenario: Negotiate a dynamic layout
- **WHEN** layout negotiation begins and receives timer ticks without establishment
- **THEN** the candidate SHALL emit one immediate and at most four later route-correct command-32 requests at the established priority/deadline, then cancel retry work and report the stable timeout exactly once

#### Scenario: Select serialized telemetry work
- **WHEN** legacy or dynamic polling starts, repeats, times out, or makes compact data due
- **THEN** the candidate SHALL retain at most one semantic telemetry request pending and emit exact direct/CAN request bytes and scheduler facts for the next legacy, realtime, or compact request without a parallel lane or immediate timeout retry

#### Scenario: Adapt ride cadence
- **WHEN** accepted samples transition among idle, riding, and the inclusive post-ride tail
- **THEN** the candidate SHALL select the established 100 ms riding cadence or configured idle cadence and emit timer replacement plus immediate next-request effects in source-equivalent order

#### Scenario: Merge normalized telemetry
- **WHEN** legacy samples, dynamic compact samples, dynamic realtime samples, decreasing compact totals, bounded extensions, or active firmware faults are accepted
- **THEN** legacy publishable data SHALL normalize directly, compact data SHALL replace the route-scoped cache without publication, realtime data SHALL merge only the latest slow fields and observation time, and published samples SHALL preserve every semantic field with monotonic source-equivalent sequence and nonnegative elapsed time

#### Scenario: Reject stale or mismatched input
- **WHEN** a response kind, scheduler completion, timer token, service generation, route, or telemetry family does not match current state
- **THEN** the candidate SHALL not clear, publish, retry, or advance unrelated work and SHALL return only the stable bounded diagnostic where native behavior reports one

#### Scenario: Remain total at public bounds
- **WHEN** intervals, tail durations, monotonic or observation times, sequences, tokens, generations, routes, extensions, and decoded optional values reach valid limits or invalid input exceeds them
- **THEN** the candidate SHALL remain bounded and deterministic, reject invalid input without partial mutation, and use explicit wrapping only for source-equivalent counters

### Requirement: Telemetry candidate remains below host execution

The Rust Refloat telemetry-service candidate SHALL expose only safe Rust state, commands, snapshots, decoded-model input, normalized samples, scheduler facts, and ordered effects. It MUST NOT own or call a clock, timer, BLE writer, callback, foreign handle, bridge record, persistence port, SQLite connection, transport, local server, frontend, thread, filesystem, network socket, async runtime, or OS framework.

#### Scenario: Return host work without executing it
- **WHEN** a transition requires a timer change, scheduler enqueue, sample publication, failure, or diagnostic
- **THEN** the candidate SHALL return the bounded typed effects in required order and retain no host reference

#### Scenario: Compose a later unified runtime
- **WHEN** a later Rust runtime owns scheduler execution, protocol decoding, application behavior, persistence, and loopback serving
- **THEN** it SHALL call this safe API directly and SHALL NOT require `refloat-telemetry-service-apple-ffi` or `refloat-telemetry-service-android-jni`

### Requirement: Rust VESC compatibility candidate parity

The repository SHALL provide one safe bounded Rust candidate that consumes the existing safe firmware-identity model and owns the exact VESC motor-controller compatibility catalogue, access-mode decision, operation authorization, and shutdown availability while Apple and Android retain their selected native policy.

#### Scenario: Admit only exact audited motor firmware
- **WHEN** a structurally decoded identity has any hardware type and any major/minor pair, including the complete byte-domain boundaries
- **THEN** the candidate SHALL admit only motor hardware type `0` at exactly 6.02, 6.05, 6.06, or 7.00 and SHALL reject every adjacent, unknown, gateway, BMS, or custom-module identity

#### Scenario: Classify stable and test access
- **WHEN** an admitted identity reports test marker zero or any nonzero byte
- **THEN** the candidate SHALL classify zero as read/write and every nonzero value as read-only without using names, UUID, CRC, custom-configuration count, or other identity fields

#### Scenario: Authorize operations
- **WHEN** read or write authorization is requested for supported stable firmware, supported test firmware, or unsupported firmware
- **THEN** reads SHALL be permitted for both supported access modes, writes SHALL be permitted only for read/write mode, and unsupported firmware SHALL permit neither

#### Scenario: Authorize exact shutdown releases
- **WHEN** shutdown availability is requested
- **THEN** it SHALL be true only for stable motor firmware 6.05, 6.06, or 7.00 and false for 6.02, every test build, and every unsupported identity

#### Scenario: Remain total over identity bounds
- **WHEN** every compatibility input and unrelated bounded identity field reaches valid limits
- **THEN** all public decisions SHALL remain deterministic and panic-free and private identity material SHALL remain absent from diagnostics

### Requirement: VESC compatibility candidate remains below application execution

The Rust VESC compatibility candidate SHALL expose only safe immutable catalogue and decision APIs over the safe firmware-identity model. It MUST NOT own framing, payload decoding, routing, discovery state, clocks, scheduling, BLE, mutation execution, callbacks, foreign records, bridges, persistence, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, or OS frameworks.

#### Scenario: Compose controller discovery directly
- **WHEN** the next Rust controller-discovery candidate evaluates direct or forwarded identities
- **THEN** it SHALL call this safe API directly without copying the catalogue or introducing a VESC-compatibility Apple FFI or Android JNI crate

#### Scenario: Compose a unified runtime later
- **WHEN** a later unified Rust runtime owns discovery and application mutation policy
- **THEN** it SHALL retain this direct safe dependency while the temporary firmware-identity and protocol bridge crates are deleted

### Requirement: Rust controller-discovery candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated direct/CAN controller-discovery decision state machine while Apple and Android retain their selected native owners. The candidate SHALL directly compose safe firmware identity, VESC compatibility, and Refloat compatibility models to own route admission, configured and learned recovery, enumeration, continuity, candidate selection, readiness, reset, and stable failure decisions without host execution.

#### Scenario: Select a direct controller
- **WHEN** the transport identity is a motor controller at an exactly supported stable or test firmware version
- **THEN** the candidate SHALL select the direct route, retain the identity and access decision, and request Refloat identification without CAN discovery

#### Scenario: Reject unsupported direct firmware
- **WHEN** a direct motor identity has an unsupported version or a non-motor endpoint cannot establish a controller route
- **THEN** the candidate SHALL return the exact stable unsupported/no-controller failure without admitting a different identity or exposing private fields

#### Scenario: Recover configured and learned routes
- **WHEN** a gateway has an authoritative preferred CAN ID or a meaningful learned UUID hint
- **THEN** the candidate SHALL probe the exact route first, enforce configured UUID continuity, accept a matching learned hint, and clear then enumerate after a stale or timed-out learned hint without falling back from an authoritative preference

#### Scenario: Enumerate a bounded CAN network
- **WHEN** CAN discovery supplies duplicates, IDs outside `0...253`, mixed hardware, missing responses, and valid controller identities
- **THEN** the candidate SHALL normalize and sort at most 64 unique valid IDs, probe each sequentially at most once, retain a complete bounded classified inventory, and never select a non-motor node

#### Scenario: Enforce enumeration bounds
- **WHEN** normalized discovery exceeds 64 CAN IDs or probing finds a seventeenth motor controller
- **THEN** the candidate SHALL fail with `too_many_can_devices` or `too_many_controllers`, publish no truncated successful result, and retain no unbounded raw input

#### Scenario: Resolve continuity and ambiguity
- **WHEN** enumeration completes with zero, one, or multiple controllers during a fresh connection or active-session recovery
- **THEN** the candidate SHALL reproduce UUID-first continuity, supported prior-route fallback only when UUID is absent, controller-changed failures, sole-controller admission/rejection, and sorted display-safe candidate publication

#### Scenario: Select only a current supported candidate
- **WHEN** selection names a current supported candidate, an unsupported candidate, an unknown ID, or stale state
- **THEN** the candidate SHALL resume Refloat identification only for the valid candidate and return a stable invalid-state result without mutation for every invalid selection

#### Scenario: Complete Refloat readiness
- **WHEN** an admitted controller receives an audited legacy or dynamic Refloat profile and a dynamic-layout admission result
- **THEN** legacy SHALL become ready directly, dynamic SHALL request layout then become ready only after successful admission, and unsupported input SHALL fail closed

#### Scenario: Isolate generations and reset
- **WHEN** begins, responses, probe timeouts, identification timeouts, selections, failures, and resets are repeated, reordered, stale, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale facts SHALL NOT alter current state, and reset SHALL clear all connection state while invalidating old work

### Requirement: Controller discovery remains below runtime execution

The Rust controller-discovery candidate SHALL expose only safe bounded facts, state, snapshots, issues, and ordered effects. It MUST NOT own or call BLE, clocks, timers, schedulers, request-byte codecs, callbacks, foreign records, bridges, persistence, Board Profile JSON, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, or OS frameworks.

#### Scenario: Return host work without executing it
- **WHEN** discovery requires a probe, CAN ping, routing-hint clear/store, Refloat request, layout request, readiness publication, candidate selection, or failure
- **THEN** it SHALL return the corresponding bounded typed effect and retain no host reference

#### Scenario: Compose the unified runtime directly
- **WHEN** a later Rust runtime owns request scheduling, persistence, application publication, and protocol execution
- **THEN** it SHALL call this safe discovery API directly without adding controller-discovery Apple FFI or Android JNI crates

### Requirement: Rust auxiliary-node discovery candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated auxiliary VBMS discovery decision state machine while Apple and Android retain their selected native owners. The candidate SHALL directly compose the safe controller-discovery route and inventory model to own exact BMS admission, direct/preferred/automatic route selection, inventory reuse, bounded fallback enumeration, availability, observation retention, reset, and stable failure decisions without host execution.

#### Scenario: Select an exact direct BMS

- **WHEN** the local routed identity is exact stock VESC BMS 6.00 or VBMS32 custom-module 6.06
- **THEN** the candidate SHALL select the direct route and request values without CAN discovery

#### Scenario: Reject an unaudited identity

- **WHEN** a local or CAN identity has another hardware type, hardware name, version, or malformed route attribution
- **THEN** the candidate SHALL NOT retain or poll it as a BMS and SHALL expose no private identity field

#### Scenario: Reuse a complete controller inventory

- **WHEN** current controller discovery supplies a complete routed inventory containing zero, one, or multiple exact BMS identities
- **THEN** the candidate SHALL classify that inventory without another CAN ping or firmware probe and resolve not-found, unique selection, or ambiguity deterministically

#### Scenario: Enumerate an incomplete inventory

- **WHEN** no complete reusable inventory exists and no direct BMS is selected
- **THEN** the candidate SHALL request one CAN ping, normalize at most 64 unique IDs in `0...253`, and probe each route sequentially at most once

#### Scenario: Honor an exact preferred route

- **WHEN** a valid preferred BMS CAN ID is configured
- **THEN** the candidate SHALL use a matching complete-inventory identity or probe that exact route directly, select it only when compatible, and fail unavailable without falling back when it is missing, incompatible, or timed out

#### Scenario: Enforce discovery bounds

- **WHEN** normalized discovery exceeds 64 CAN IDs or classification finds a seventeenth exact BMS
- **THEN** the candidate SHALL become unavailable with a stable bounded issue, select no partial route, and retain no unbounded raw input

#### Scenario: Retain and recover observation state

- **WHEN** a selected route receives a valid current observation, becomes unavailable, and later receives another valid current observation
- **THEN** it SHALL become attached, retain the prior observation as stale while unavailable, and replace it while recovering without inspecting or logging the observation payload

#### Scenario: Isolate generations and reset

- **WHEN** begins, CAN results, firmware results, timeouts, observations, failures, and resets are repeated, reordered, stale, invalid, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale work SHALL NOT alter current state, and reset SHALL clear all auxiliary state while invalidating old work

### Requirement: Auxiliary discovery remains below runtime execution

The Rust auxiliary-node discovery candidate SHALL expose only safe bounded routed facts, state, generic owned observation retention, stable issues, and ordered effects. It MUST NOT own or call BLE, clocks, timers, schedulers, request bytes, protocol decoders, callbacks, bridges, persistence, Board Profile JSON, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, frontend serialization, or OS frameworks.

#### Scenario: Return host work without executing it

- **WHEN** discovery requires a CAN ping, firmware probe, values request, publication, or failure
- **THEN** it SHALL return or expose the corresponding bounded typed effect and state while retaining no host reference

#### Scenario: Compose the unified runtime directly

- **WHEN** a later Rust runtime owns request scheduling, BMS decoding, application publication, and persistence
- **THEN** it SHALL call this safe auxiliary-discovery API directly without adding auxiliary-discovery Apple FFI or Android JNI crates
### Requirement: Rust VESC BMS telemetry service candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated VESC BMS polling/application lifecycle while Apple and Android retain their selected native owners. The candidate SHALL directly compose Rust auxiliary discovery, command-96 decoding, and request-scheduler facts to own discovery execution facts, selected-route polling, concrete observations, completion-driven cadence, failure accounting, unavailability, recovery, generation isolation, reset, and ordered host effects.

#### Scenario: Execute discovery work as scheduler facts

- **WHEN** current auxiliary discovery requests CAN enumeration, a firmware probe, or initial values
- **THEN** the service SHALL emit exact read-only direct/CAN request bytes, expected response, auxiliary priority, context, timeout, and scheduler generation without executing host work

#### Scenario: Poll one selected route

- **WHEN** a direct or CAN BMS is selected and a values request completes as response or sent
- **THEN** the service SHALL retain at most one current values poll, replace one generation-qualified timer for 1,000 ms, and request the same still-selected route when that exact timer fires

#### Scenario: Account for one failed poll

- **WHEN** a values request times out, is rejected because the bounded queue is full, cannot be enqueued, or returns one malformed command-96 payload
- **THEN** the service SHALL count exactly one consecutive failure for that poll and continue bounded recovery polling without delaying controller telemetry

#### Scenario: Become unavailable and recover

- **WHEN** a selected BMS reaches three consecutive failures and later returns a valid current complete response
- **THEN** status SHALL become unavailable while retaining the prior observation, then reset failures and recover to attached with the newer observation

#### Scenario: Own the complete bounded observation

- **WHEN** valid direct or CAN command-96 bytes arrive with a current expected route and observation time
- **THEN** the service SHALL own every bounded decoded field, cell, balancing flag, temperature, optional version/status, route, and time without publishing private values through diagnostics

#### Scenario: Preserve standalone BMS isolation

- **WHEN** an exact direct BMS reaches ready and is polled through the service
- **THEN** it SHALL use the same direct read-only lifecycle while emitting no session, location, persistence, controller-control, configuration, bootloader, or firmware-write effect

#### Scenario: Reject stale or mismatched work

- **WHEN** timer, completion, CAN result, identity, payload, route, request kind, auxiliary generation, scheduler generation, or service generation is stale, repeated, invalid, or mismatched
- **THEN** it SHALL not alter current discovery, failure, observation, cadence, or publication state

#### Scenario: Remain total at public bounds

- **WHEN** routes, inventories, payload counts, observation collections, timestamps, timers, counters, and generations reach valid limits or invalid input exceeds them
- **THEN** the service SHALL remain bounded and deterministic, reject invalid input without partial mutation, and wrap only explicit source-equivalent identity counters

### Requirement: BMS telemetry service remains below runtime execution

The Rust VESC BMS telemetry-service candidate SHALL expose only safe Rust commands, state, owned observations, scheduler facts, and ordered effects. It MUST NOT own or call BLE, a clock, timer executor, callback, foreign handle, bridge record, persistence port, SQLite connection, session store, transport, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return runtime work without executing it

- **WHEN** a transition requires scheduler enqueue, timer replacement/cancellation, state or observation publication, failure, or diagnostic work
- **THEN** the candidate SHALL return bounded typed effects in source-equivalent order and retain no host reference

#### Scenario: Compose the unified runtime directly

- **WHEN** a later Rust runtime owns scheduler execution, protocol decoding, application publication, persistence, and loopback serving
- **THEN** it SHALL call this safe service directly without `vesc-bms-telemetry-service-apple-ffi` or `vesc-bms-telemetry-service-android-jni`

### Requirement: Rust connection application coordinator candidate parity

The repository SHALL provide one safe bounded Rust candidate for the duplicated connection-generation application spine while Apple and Android retain their selected native owners. The candidate SHALL directly compose Rust controller discovery, request scheduling, Refloat telemetry, and VESC BMS telemetry to own start/reset, request attribution, discovery sequencing, readiness activation, child lifecycle, and ordered runtime effects.

#### Scenario: Begin one atomic connection generation

- **WHEN** valid connection settings, route preferences, continuity facts, and current time begin a new generation
- **THEN** the coordinator SHALL cancel and reset prior work, advance coordinator and scheduler generations, start exact direct firmware identification, and publish ordered deadline/write work without retaining a host reference

#### Scenario: Discover and prepare one controller

- **WHEN** current scheduler-attributed direct, preferred, hinted, enumerated, Refloat INFO, and dynamic-layout responses establish one supported controller
- **THEN** the coordinator SHALL preserve UUID-first continuity and explicit-selection policy, remember or clear routing hints in source-equivalent order, and enter awaiting activation with the exact selected route, identity, compatibility profile, and classified inventory

#### Scenario: Activate application services only after confirmation

- **WHEN** the current awaiting-activation generation receives host confirmation with valid monotonic/session facts
- **THEN** the coordinator SHALL start route-correct Refloat telemetry and attached BMS discovery, publish ready after their initial ordered effects, and reject stale or repeated activation without starting duplicate work

#### Scenario: Preserve standalone BMS isolation

- **WHEN** the direct transport identity is an exact supported standalone BMS
- **THEN** the coordinator SHALL enter ready and start only the direct read-only BMS lifecycle without controller selection, Refloat telemetry, session, location, persistence, configuration, mutation, or control effects

#### Scenario: Attribute every scheduler result once

- **WHEN** writes receive responses, are sent, time out, encounter queue capacity, are cancelled, or belong to an old generation
- **THEN** the coordinator SHALL route each exact completion to only its owning discovery or child-service transition before pumping later work and SHALL not double-count failure or publish stale data

#### Scenario: Route every child timer exactly

- **WHEN** a current Refloat layout/poll or BMS poll timer fires
- **THEN** the coordinator SHALL deliver it only to the exact current child generation and route, enqueue resulting facts through the owned scheduler, and ignore stale, repeated, mismatched, or cancelled timers

#### Scenario: Reset all connection-scoped state

- **WHEN** the connection resets before, during, or after discovery/readiness/telemetry
- **THEN** scheduler requests, deadlines, child timers, layouts, observations, failures, selections, candidates, and public state SHALL clear in deterministic order and all prior-generation events SHALL become inert

#### Scenario: Remain total at public bounds

- **WHEN** payloads, CAN inventories, scheduler queues, timestamps, timers, counters, generations, nested effects, and decoded collections reach valid limits or invalid input exceeds them
- **THEN** the coordinator SHALL remain bounded and deterministic, reject invalid input without partial mutation, and wrap only explicitly source-equivalent counters

### Requirement: Connection coordinator remains below host runtime execution

The Rust connection application coordinator candidate SHALL expose only safe Rust commands, snapshots, owned protocol/application values, and ordered effects. It MUST NOT own or call BLE, frame streams, a physical clock, timer executor, callback, foreign handle, bridge record, persistence port, SQLite connection, session store, transport, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return all host work without executing it

- **WHEN** a transition requires BLE payload writing, deadline/timer work, routing-hint storage, application publication, persistence confirmation, failure, or diagnostic work
- **THEN** the candidate SHALL return bounded typed effects in source-equivalent order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later Rust runtime owns effect execution, persistence, sessions, application publication, and loopback serving
- **THEN** it SHALL call this coordinator and its safe dependencies directly without a connection-coordinator Apple FFI or Android JNI crate

### Requirement: Rust device mutation coordinator candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated connection-scoped device mutation authorization and serialization owner while Apple and Android retain their selected native owners. The candidate SHALL directly use safe compatibility, control-codec, normalized-telemetry, firmware-identity, and scheduler-fact models to own current authoritative mutation state without a foreign bridge.

#### Scenario: Configure one exact generation

- **WHEN** a current checked controller route, structurally decoded firmware identity, admitted Refloat compatibility profile, and readiness facts configure the candidate
- **THEN** it SHALL derive writable and shutdown capability through safe Rust policy, replace all prior-generation state atomically, and publish one bounded loading control snapshot with no request

#### Scenario: Establish authoritative control state

- **WHEN** current monotonic normalized telemetry and exact profile-matched lighting observations arrive
- **THEN** package, flywheel, fatal-latch, active-fault, clearability, LED, and headlight state SHALL converge to the same ready/loading snapshot and confirmation order as both native owners

#### Scenario: Authorize every semantic control

- **WHEN** package, flywheel, lighting, or fatal-clear is requested against current readiness, compatibility, access, ride, package, footpad, fault, and pending state
- **THEN** the candidate SHALL either return an exact idempotent snapshot with no request or one generation-qualified mutation token and exact route-correct scheduler request facts matching the selected native behavior

#### Scenario: Preserve the flywheel safety exception

- **WHEN** authoritative package mode is flywheel
- **THEN** every mutation SHALL be rejected except exact eligible flywheel disable, which SHALL remain available while normalized ride state is riding and SHALL confirm only after later normal-mode telemetry

#### Scenario: Serialize controller shutdown

- **WHEN** shutdown is requested on a current ready idle exact supported writable controller with no competing mutation
- **THEN** the candidate SHALL reserve the sole pending slot and return one exact direct or once-forwarded accepted-only shutdown request while unsupported, moving, flywheel, stale, read-only, or competing states emit no request

#### Scenario: Serialize configuration and Tune work

- **WHEN** a configuration write or Tune apply is requested with current ready writable idle state, no footpad engagement, ready normal package state, and no competing mutation
- **THEN** the candidate SHALL return one exact generation/sequence-qualified configuration lease that excludes controls and shutdown until matching verified finish, failure, cancellation, or reset

#### Scenario: Confirm, fail, and reset exactly

- **WHEN** authoritative observations, request failure, lease completion, shutdown clearing, cancellation, or connection reset occur
- **THEN** only the exact current token and generation SHALL alter pending state, confirmations SHALL follow source order, and stale, repeated, mismatched, or prior-generation events SHALL remain inert

#### Scenario: Remain total at public bounds

- **WHEN** routes, identities, generations, sequences, sample sequences, payloads, profiles, observations, and transition order reach valid limits or invalid input exceeds them
- **THEN** the candidate SHALL remain bounded and deterministic, reject invalid input without partial mutation, wrap only the explicit token counter, and expose no private firmware or telemetry payload through diagnostics

### Requirement: Device mutation coordinator remains below runtime execution

The Rust device mutation coordinator candidate SHALL expose only safe Rust commands, bounded snapshots, mutation tokens, scheduler request facts, stable errors/diagnostics, and ordered effects. It MUST NOT own or call BLE, VESC framing, a scheduler instance, physical clock, timer executor, callback, foreign handle, bridge record, persistence, SQLite, sessions, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return mutation work without executing it

- **WHEN** an authorized operation requires scheduler submission, application publication, pending completion, or diagnostic work
- **THEN** the candidate SHALL return the exact bounded owned values in deterministic order and retain no host or caller reference

#### Scenario: Compose the future coordinator directly

- **WHEN** a later Rust connection application coordinator owns configuration, request scheduling, response attribution, and effect publication
- **THEN** it SHALL call this safe candidate directly without a device-mutation Apple FFI or Android JNI crate

### Requirement: Composed Rust device configuration application lifecycle

The safe Rust connection application coordinator SHALL directly own and compose the Rust device mutation coordinator and Refloat configuration service with its existing generation, scheduler, discovery, telemetry, and BMS owners. The composition SHALL preserve the selected native authorization, serialization, request attribution, response handling, cache intent, confirmation, failure, and reset behavior without a foreign bridge.

#### Scenario: Activate one selected controller generation

- **WHEN** current discovery reaches a supported controller and the future persistence owner activates the exact awaiting generation with an optional exact cache value
- **THEN** the coordinator SHALL configure mutation and configuration children from the selected checked route, firmware identity, Refloat profile, writability, package version, flywheel state, and cache facts before starting ready application behavior

#### Scenario: Exclude standalone BMS application behavior

- **WHEN** the direct transport is a standalone supported BMS
- **THEN** the coordinator SHALL start only BMS behavior and SHALL expose no controller mutation, Refloat configuration, telemetry-session, cache, or controller-ready work

#### Scenario: Submit semantic mutations through one scheduler

- **WHEN** a current control, fatal-clear, lighting, or shutdown command is authorized
- **THEN** the mutation owner SHALL produce the sole exact token/request facts and the connection coordinator SHALL submit them through its existing scheduler without copying authorization or wire-codec behavior

#### Scenario: Observe authoritative confirmation in source order

- **WHEN** current normalized telemetry or an attributed lighting response arrives
- **THEN** the coordinator SHALL deliver the observation to the mutation owner before publishing resulting application state, preserving exact confirmation and flywheel-prohibition order

#### Scenario: Read configuration without a mutation lease

- **WHEN** current/default values or schema loading is requested without a device write
- **THEN** the configuration service SHALL produce commands 92–94 work through the existing scheduler while controls and shutdown remain governed only by their current pending state

#### Scenario: Hold one lease through verified configuration write

- **WHEN** a configuration write or Tune apply is authorized
- **THEN** the coordinator SHALL acquire the sole generation/sequence-qualified mutation lease before command-95 submission and retain it through acknowledgement and command-93 verified readback until exact success or failure

#### Scenario: Attribute every configuration completion

- **WHEN** a commands 92–95 response or scheduler completion arrives
- **THEN** exact scheduler context, route, index, operation, generation, command, and response bounds SHALL select one configuration transition, while stale, mismatched, malformed, repeated, or unrelated input SHALL alter no current operation

#### Scenario: Return cache and publication work

- **WHEN** configuration state admits, rejects, stores, removes, loads, writes, verifies, conflicts, or fails
- **THEN** the coordinator SHALL return deterministic bounded cache/publication/failure effects without executing persistence or exposing schema/value bytes through diagnostics

#### Scenario: Fail and reset all composed work exactly

- **WHEN** encoding, enqueue, timeout, queue-full, cancellation, stale generation, decode, service, flywheel, disconnect, reconnect, or reset terminates work
- **THEN** scheduler, configuration operation, mutation token, in-memory schema/value state, and published snapshots SHALL converge atomically for the exact current generation with no stuck lease or stale completion

#### Scenario: Remain total at composition bounds

- **WHEN** generations, counters, routes, payloads, schema pages, values, cache bytes, nested effects, transition ordering, and public commands reach valid limits or exceed invalid bounds
- **THEN** the coordinator SHALL remain deterministic and panic-free for public input, reject invalid work before partial publication, enforce the finite effect bound, and expose no private firmware, schema, configuration, or telemetry payload in diagnostics

### Requirement: Composed application coordinator remains below runtime execution

The composed Rust coordinator SHALL expose only safe Rust commands, snapshots, owned work values, and ordered effects. It MUST NOT own or invoke BLE, VESC framing, a physical clock, timer executor, callback, foreign handle, bridge record, cache implementation, persistence port, SQLite, session storage, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return execution work without performing it

- **WHEN** a transition requires a BLE write, deadline/timer action, cache mutation, persistence/publication action, or diagnostic
- **THEN** the coordinator SHALL return the exact bounded owned effect in deterministic order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later platform-neutral Rust runtime owns effect execution, SQLite/sessions, application publication, and loopback serving
- **THEN** it SHALL call this safe coordinator and its safe children directly without connection, mutation, configuration, or protocol Apple FFI/Android JNI boundaries

### Requirement: Composed Rust controller current-limit application lifecycle

The safe Rust connection application coordinator SHALL directly own the complete one-shot controller current-limit application lifecycle by composing the safe VESC controller-data decoder with its current activation generation and request scheduler. The composition SHALL preserve native request order, direct/CAN routing, response attribution, optional live-state publication, soft failure, and reset behavior without a foreign bridge.

#### Scenario: Request current limits after controller activation

- **WHEN** an exact supported controller generation is activated
- **THEN** the coordinator SHALL enqueue one route-correct command-14 request with controller-work priority, the native-equivalent timeout, exact current scheduler generation, and current-limit response context after initial configuration work and before telemetry/BMS startup completes

#### Scenario: Exclude standalone BMS

- **WHEN** the direct transport is an exact supported standalone BMS
- **THEN** the coordinator SHALL publish no current-limit value and SHALL emit no command-14 request

#### Scenario: Decode one attributed direct or CAN response

- **WHEN** the exact current scheduled direct/CAN current-limit request receives a valid command-14 response
- **THEN** the safe controller-data owner SHALL decode all four finite nonzero limits and optional positive battery-series-cell count, and the coordinator SHALL retain and publish exactly that current ready value

#### Scenario: Keep optional read failures non-fatal

- **WHEN** current-limit work is malformed, timed out, queue-full, cancelled, stale, repeated, mismatched, or reset
- **THEN** the coordinator SHALL expose no stale or partial current-limit value, SHALL leave connection readiness unchanged, and SHALL emit at most stable payload-free diagnostic work

#### Scenario: Clear limits with the connection generation

- **WHEN** begin, reconnect, disconnect, or reset replaces the current connection generation
- **THEN** retained current limits and every in-flight completion from the prior generation SHALL become absent and inert before new application state is published

#### Scenario: Preserve bounded private state

- **WHEN** valid or adversarial current-limit values, payload extents, routes, generations, completions, and transition orders reach their bounds
- **THEN** the coordinator SHALL remain deterministic and panic-free for public input, retain at most one bounded value, and expose no numeric limits or payload bytes through diagnostics or debug formatting

### Requirement: Current-limit composition remains below runtime execution

The composed current-limit lifecycle SHALL expose only safe Rust snapshot values and ordered effects. It MUST NOT own or invoke BLE, VESC framing, a physical clock, timer executor, callback, foreign handle, bridge record, persistence, SQLite, sessions, local serving, frontend code, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return request and publication work without executing it

- **WHEN** activation or an attributed completion requires BLE request execution or application publication
- **THEN** the coordinator SHALL return exact bounded owned effects in deterministic order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later platform-neutral Rust runtime executes coordinator effects and serves application state
- **THEN** it SHALL consume the safe current-limit value directly without a controller-data or coordinator Apple FFI/Android JNI boundary

### Requirement: Safe Rust session-history reduction kernel

The repository SHALL provide one safe bounded Rust candidate for historical series reduction while Apple and Android retain their selected native reducers. The candidate SHALL consume ordered platform-neutral rows incrementally and own exact M4 role selection, null/gap continuity generations, and deterministic bounded column publication.

#### Scenario: Configure bounded reduction

- **WHEN** a caller supplies 1–16 unique bounded field names, an inclusive nonnegative elapsed range, 1–1,024 buckets, a valid explicit or automatic gap policy, positive persistence cadence, and bounded retained resolution
- **THEN** the candidate SHALL allocate only field/bucket-bounded state and preserve the supplied field order

#### Scenario: Select M4 roles

- **WHEN** ordered finite observations populate one or more buckets
- **THEN** each field/bucket SHALL retain the chronologically first and last rows plus the minimum and maximum finite values, resolve equal roles by earliest `(elapsedMs, sequence)`, deduplicate multi-role source rows, and publish candidates in chronological order

#### Scenario: Preserve discontinuities

- **WHEN** a value is absent, an elapsed delta exceeds the explicit threshold, or an automatic phase/cadence/resolution threshold is crossed
- **THEN** the affected continuity generation SHALL advance exactly and later values SHALL NOT be connected across that boundary

#### Scenario: Reject invalid ordered input

- **WHEN** configuration or input has duplicate/oversized fields, invalid ranges, invalid counts, non-finite present values, mismatched value arity, out-of-range rows, descending or duplicate keys, invalid cadence/thresholds, arithmetic overflow, generation exhaustion, or more than 65,536 selected observations
- **THEN** the candidate SHALL return one stable payload-free error without panicking, partially publishing a result, or retaining unbounded input

### Requirement: Safe Rust session prune-selection kernel

The Rust session-history candidate SHALL select retained real source rows from an ordered stream using elapsed-zero-aligned buckets, protected semantic evidence, deterministic extrema coverage, and an explicit cap of eight ordinary rows per bucket. Its live memory SHALL remain bounded by field count and the current bucket rather than ride length.

#### Scenario: Retain protected evidence

- **WHEN** a session stream contains endpoints, first non-null field sources, both sides of riding-phase transitions, caller-marked event sources, caller-marked summary sources, or other protected rows
- **THEN** every such source row SHALL be retained independently of the ordinary eight-row bucket cap

#### Scenario: Cover ordinary bucket roles

- **WHEN** an elapsed bucket contains unprotected first/last and ordered per-field minimum/maximum candidates
- **THEN** the candidate SHALL select deterministic maximum-coverage rows with chronological tie-breaking, retain no more than eight ordinary rows, and report any roles left uncovered by that cap

#### Scenario: Finalize incrementally

- **WHEN** ordered input advances into another elapsed-zero-aligned bucket or the caller finishes the stream
- **THEN** the candidate SHALL return the finalized prior selection in chronological order and release its raw bucket rows before accepting unbounded later history

#### Scenario: Preserve aligned coarser behavior

- **WHEN** previously retained source rows are selected again at a strictly coarser aligned integer-multiple resolution
- **THEN** protected evidence SHALL remain present and the deterministic result SHALL introduce no row absent from the supplied retained source

#### Scenario: Reject invalid prune input

- **WHEN** field, resolution, ordering, sequence, elapsed, value, bucket-width, counter, or role bounds are invalid or exhausted
- **THEN** the candidate SHALL fail closed with a stable payload-free error and SHALL publish no partial current bucket

### Requirement: Session-history kernel remains below persistence and runtime execution

The Rust session-history candidate SHALL expose only safe Rust configuration, row, column, selection, issue, and ordered-finalization values. It MUST NOT own or call SQLite, SQL, a persistence port, cursor JSON/base64, a local server, frontend code, BLE, callbacks, foreign handles, bridge records, threads, filesystems, sockets, async runtimes, or OS frameworks.

#### Scenario: Compose future SQLite directly

- **WHEN** a later platform-neutral Rust session store reads ordered SQLite rows or performs prune selection
- **THEN** it SHALL call this safe kernel directly and persist returned facts without a session-history Apple FFI or Android JNI boundary

#### Scenario: Preserve current production ownership

- **WHEN** the candidate is built and tested before runtime cutover
- **THEN** selected native reducers, stores, cursor encoders, loopback handlers, durable schemas, and application publication SHALL remain unchanged

### Requirement: Rust canonical session SQLite writer

The repository SHALL provide one bounded Rust candidate that creates and reopens a canonical per-session SQLite file and atomically persists normalized telemetry, derived field/event evidence, commit summaries, and location observations while selected Apple and Android stores remain unchanged.

#### Scenario: Create an exact canonical session file

- **WHEN** a caller supplies a new path, positive bounded idle/riding persistence cadences, and a commit timestamp
- **THEN** the writer SHALL initialize the byte-exact `backends/sql/session.sql` schema, one metadata row, and one zero-count commit marker in a private temporary file and atomically publish only a complete database without overwriting an existing target

#### Scenario: Reopen a compatible database

- **WHEN** a completed canonical session file passes integrity and required schema/singleton/cadence checks
- **THEN** the writer SHALL reopen it and continue durable sample, event, field, location, and marker sequences from stored state

#### Scenario: Reject an incompatible database

- **WHEN** a target is missing required schema objects or singleton rows, is corrupt, has invalid metadata, or fails canonical compatibility checks
- **THEN** the writer SHALL return a stable payload-free incompatibility or corruption error without repairing, migrating, truncating, or partially mutating the file

### Requirement: Atomic normalized telemetry persistence

The Rust writer SHALL accept bounded ordered `refloat-telemetry-service::NormalizedSample` batches directly and persist the canonical sample columns, bounded deterministic extension JSON, first-source numeric fields, transition events, and exact commit-marker aggregates in one immediate transaction.

#### Scenario: Persist a new ordered batch

- **WHEN** 1–1,024 valid strictly ordered samples contain bounded finite stable and extension values
- **THEN** the writer SHALL insert every new sequence, preserve all canonical nullable values, discover each stable/extension field once, emit only changed transition values, and atomically update recorded/stored/event counts, committed sequences/time, riding elapsed, distance, extrema values and earliest source sequences, plus last speed/state

#### Scenario: Replay already stored sequences

- **WHEN** an otherwise valid ordered batch includes sample sequences already present in the database
- **THEN** existing sample bytes, first-field sources, and transition events SHALL remain unchanged, no duplicate stored row SHALL appear, recorded count SHALL include every submitted sample, and the marker SHALL remain a deterministic projection of durable rows plus submitted-count semantics

#### Scenario: Continue across batches and reopen

- **WHEN** ordered samples are appended in multiple calls separated by checkpoint, close, or reopen
- **THEN** transition comparison, event sequence, first-field discovery, summaries, extrema ties, counts, and committed source sequences SHALL match one continuous durable session

#### Scenario: Roll back a failed sample transaction

- **WHEN** any input bound, conversion, JSON bound, SQLite statement, constraint, busy/full/I/O condition, aggregate computation, or marker update fails
- **THEN** the call SHALL expose one stable payload-free error and SHALL commit none of that batch's samples, fields, events, counts, or marker changes

### Requirement: Atomic canonical location persistence

The Rust writer SHALL accept bounded already-observed location facts independently of location acquisition and persist canonical validated/deduplicated rows plus marker state in one immediate transaction.

#### Scenario: Persist ordered locations

- **WHEN** 1–1,024 observations contain durable nonnegative time/elapsed values, finite in-range coordinates, and absent or finite nonnegative accuracy
- **THEN** the writer SHALL order them deterministically, ignore existing elapsed/latitude/longitude duplicates, assign consecutive location sequences, and atomically update committed location sequence/elapsed, recorded/stored counts, and available status

#### Scenario: Preserve an all-duplicate marker

- **WHEN** every valid submitted observation already exists
- **THEN** no new row SHALL be stored and every location marker field SHALL remain unchanged

#### Scenario: Roll back a failed location transaction

- **WHEN** any observation bound, conversion, SQLite statement, constraint, busy/full/I/O condition, or marker update fails
- **THEN** the call SHALL expose one stable payload-free error and SHALL commit none of that batch's location rows or marker changes

### Requirement: Read canonical session SQLite snapshots

The Rust session SQLite package SHALL provide an independent read-only `SessionReader` that validates one caller-selected canonical session database and returns only safe owned metadata, commit boundary, and numeric-field facts without mutating or discovering files.

#### Scenario: Open a compatible canonical database

- **WHEN** a session database passes integrity, required schema/index/singleton, positive cadence, marker, and field-evidence checks
- **THEN** the reader SHALL open it read-only and expose exact metadata, retained-resolution, commit-marker, and ordered stable/extension field facts

#### Scenario: Reject incompatible or malformed durable state

- **WHEN** a file is absent, corrupt, incompatible, missing required objects or singletons, contains invalid cadence/marker/field data, or cannot be opened read-only
- **THEN** the reader SHALL return a stable payload-free error without repairing, migrating, checkpointing, truncating, or otherwise mutating the file

#### Scenario: Validate a published boundary inside the read snapshot

- **WHEN** a query supplies a published history revision and sample/event/location sequences
- **THEN** the reader SHALL validate them against the commit marker inside the same SQLite read transaction, require an equal history revision and nonfuture sequences, and cap every scan at those sequences

### Requirement: Bounded canonical sample and series reads

The Rust session reader SHALL resolve 1–16 unique bounded requested numeric fields through one closed stable-field catalogue or validated extension-field evidence and SHALL return deterministic ordered sample pages or `session-history-kernel` series reductions from the canonical sample table.

#### Scenario: Reduce a published series window

- **WHEN** a valid nonnegative elapsed range, 1–1,024 buckets, canonical cadence/gap policy, retained resolution, published boundary, and at most 200,000 ordered source rows are supplied
- **THEN** the reader SHALL stream finite stable/extension values and exact ride phases into `SeriesAccumulator` and return the kernel's bounded columnar first/minimum/maximum/last observations, sequences, and gap generations without retaining the source window

#### Scenario: Page lossless sample rows

- **WHEN** a valid range, 1–2,000 row limit, requested fields, published boundary, and optional prior `(elapsed, sequence)` key are supplied
- **THEN** the reader SHALL return the next rows ordered by `(elapsed, sequence)`, exact nullable finite values, and an owned continuation key only when another row exists

#### Scenario: Reject an unsafe field or query bound

- **WHEN** fields are empty, duplicated, too many, oversized, unsupported, missing extension evidence, or resolve malformed/nonfinite stored values, or a range/bucket/source/page/integer bound is invalid
- **THEN** the reader SHALL fail closed without returning a partial page or reduction and SHALL interpolate no caller-controlled SQL identifier or JSON path

### Requirement: Bounded canonical event and location reads

The Rust session reader SHALL expose deterministic published event and location pages from canonical durable rows while leaving opaque cursor and wire-format ownership above the persistence package.

#### Scenario: Page supported transition events

- **WHEN** a valid range, 1–500 row limit, published event boundary, and optional prior `(elapsed, sequence)` key are supplied
- **THEN** the reader SHALL return only the closed supported transition kinds in `(elapsed, sequence)` order with exact details JSON and nullable source-sample sequence plus an owned continuation key when more rows exist

#### Scenario: Page recorded locations

- **WHEN** a valid range, 1–2,000 row limit, published location boundary, and optional prior location sequence are supplied
- **THEN** the reader SHALL return consecutive ordered finite coordinates, nullable nonnegative accuracy, observation time, deterministic greater-than-ten-second `noFix` gaps, the published boundary, and a continuation sequence only when more rows exist

#### Scenario: Reject malformed event or location state

- **WHEN** durable event JSON/kind/source facts or location sequence/time/coordinate/accuracy facts violate canonical bounds or ordering
- **THEN** the query SHALL return a stable payload-free failure and no partial page

### Requirement: Session SQLite reads remain interruptible and writer-compatible

The Rust reader SHALL hold no thread or async runtime but SHALL expose a safe cloneable interrupter capable of cancelling an executing SQLite scan from another runtime-owned thread. Read transactions SHALL use WAL-compatible snapshot semantics and release every statement and transaction after success, failure, or cancellation.

#### Scenario: Interrupt a long historical scan

- **WHEN** another thread interrupts an active bounded scan
- **THEN** the scan SHALL return the stable cancelled error, release its transaction and statements, reveal no query or data payload, and permit a later query on the reader

#### Scenario: Read while the writer advances

- **WHEN** `SessionWriter` commits a newer batch while a reader holds an older valid published snapshot
- **THEN** the writer SHALL complete without waiting for reduction, the reader SHALL observe only its transaction and published sequence boundaries, and a later read SHALL observe the newer marker

### Requirement: Session SQLite remains below catalog and runtime execution

The Rust session SQLite package SHALL own independent writer and read-only connections and expose only safe owned creation, open, append, marker, checkpoint, close, metadata, field, bounded query, reduction, page-key, and interruption values. It MUST NOT own catalog SQLite, settings, profiles, opaque cursors, history prune mutation, imports/exports, deletion/retention, recovery policy, local serving, gzip/JSON envelopes, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose future persistence directly

- **WHEN** a later Rust catalog/application runtime creates, records, or queries a session
- **THEN** it SHALL call the safe writer, reader, and history-kernel APIs directly without session SQLite Apple FFI or Android JNI boundaries

#### Scenario: Preserve current production ownership

- **WHEN** the consolidated candidate is built and tested before runtime cutover
- **THEN** selected native stores, catalog reconciliation, session readers/reducers, cursor encoders, recovery, files, loopback handlers, and application publication SHALL remain unchanged

#### Scenario: Bound dependencies and diagnostics

- **WHEN** maintainers inspect the locked graph or any valid/adversarial value reaches errors and debug formatting
- **THEN** dependencies SHALL remain limited to the named safe Rust telemetry/history owners, maintained bundled SQLite/JSON support and their build graph, and output SHALL reveal no path, SQL, telemetry, coordinate, extension, event, JSON, or SQLite-message payload
