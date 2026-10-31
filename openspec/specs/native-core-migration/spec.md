# Shared Backend Component Guarantees

## Purpose

Define the retained protocol, coordination and session-storage guarantees of the production Rust components. The historical capability identifier remains stable for references.

Completed extraction, intermediate native ownership and per-protocol bridge
acceptance are historical. Original requirements and evidence remain in
[archived changes](../../changes/archive/); ongoing checks follow
[AGENTS.md](../../../AGENTS.md#verification).

Production composition and native boundaries are specified in
[shared-rust-backend-service](../shared-rust-backend-service/spec.md),
[unified-rust-runtime](../unified-rust-runtime/spec.md) and the coarse host-binding specs.

## Requirements

### Requirement: Safe components return execution work

Portable protocol/coordination components SHALL expose bounded safe values and deterministic ordered effects without executing host work, retaining caller/host references or introducing per-component foreign bridges. The shared runtime/service SHALL compose existing owners directly, preserving downward dependencies.

#### Scenario: Execute coordination effects

- **WHEN** scheduling, discovery, telemetry, mutation, configuration or current-limit coordination requires external work
- **THEN** its safe owner returns typed writes, timer/deadline changes, requests, cache/routing-hint work, publications, completions or diagnostics for execution by the existing runtime/service
- **AND** these coordination owners contain no BLE/frame-stream execution, physical clock, timer executor, callback, foreign handle/record, persistence/cache implementation, SQLite/session store, server, frontend, thread, filesystem, socket, async runtime or OS framework

#### Scenario: Preserve focused responsibilities

- **WHEN** dependencies or responsibilities change
- **THEN** the scheduler owns neither codecs nor a persistence port; discovery emits probe/request facts without owning schedulers or request-byte codecs; auxiliary discovery accepts decoded observations without owning protocol decoders; compatibility remains immutable policy without framing, routing, decoding, discovery state or mutation execution
- **AND** the mutation owner returns scheduler request facts without owning a scheduler instance or VESC framing; composed coordination uses safe children and does not execute framing or storage work

### Requirement: Rust VESC framing parity

The shared Rust framing component SHALL reproduce the shipped bounded VESC framing behavior: CRC-16 with polynomial `0x1021` and zero initial state; short frames for payloads through 255 bytes; long frames above 255 bytes; big-endian long length and CRC fields; footer byte `3`; a maximum payload of 4,096 bytes; and a maximum stream buffer of 8,192 bytes.

#### Scenario: Preserve bounded framing

- **WHEN** VESC byte streams are framed or decoded
- **THEN** exact decoding and stream reassembly SHALL fail closed on invalid CRC, footer, header, declared length, trailing bytes, and aggregate bounds while accepting valid fragmentation, multiple frames, and leading noise according to current product behavior.

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

### Requirement: Rust VESC firmware identity behavior

The `backends/core` workspace SHALL contain one safe Rust firmware identity component dedicated to structurally decoding bounded VESC `COMM_FW_VERSION` payloads.

#### Scenario: Decode complete identity

- **WHEN** a bounded firmware identity payload is decoded
- **THEN** it SHALL expose major and minor versions, a hardware name of at most 64 input bytes, a private 12-byte UUID, test-version marker, hardware type, custom-configuration count, an optional firmware name of at most 64 input bytes, and an optional big-endian four-byte hardware-configuration CRC. It SHALL consume but not expose the established pairing and capability bytes and SHALL log no payload, UUID, name, address, token, or full identity.
- **AND** the decoder SHALL accept the existing audited motor and custom-module tail, SHALL accept hardware type `1` only as exact stock VESC BMS 6.00 with no trailing bytes, and SHALL fail closed with typed unsupported-command or structural/truncation errors for every malformed prefix, missing or overlong terminator, incomplete capability tail, malformed BMS identity, invalid CRC-tail length, or unexpected trailing byte. Invalid UTF-8 in a bounded terminated name SHALL use replacement decoding.

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

### Requirement: Rust VESC custom-configuration behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to encoding and decoding bounded VESC custom-configuration command envelopes.

#### Scenario: Preserve configuration envelopes

- **WHEN** a configuration request or response crosses the codec
- **THEN** it SHALL encode command-92 schema requests, command-93 current requests, command-94 default requests, and command-95 writes; it SHALL decode bounded schema chunks, opaque current/default values, and exact write acknowledgements without interpreting compressed schema or typed Refloat values.
- **AND** the component SHALL preserve the selected production command bytes, index byte, signed big-endian length/offset rules, 400-byte schema chunk bound, minimum four-byte values signature extent, existing write bound, response variant, and structural versus recognized-unsupported failure distinction. It SHALL borrow or return only bounded caller-owned bytes, forbid unsafe code, and log no configuration payload.

#### Scenario: Encode exact configuration requests

- **WHEN** the component receives valid schema, current/default, or write inputs at representative and boundary extents
- **THEN** it emits the exact command 92–95 payload bytes produced by the selected host adapters

#### Scenario: Decode exact configuration responses

- **WHEN** the component receives a valid schema chunk, current/default values response, or write acknowledgement
- **THEN** it returns the matching typed envelope, exact index/extents/command, and unchanged opaque bytes without parsing Refloat semantics

#### Scenario: Reject malformed configuration envelopes

- **WHEN** input has invalid integer extent, truncation, excessive chunk data, inconsistent total/offset/chunk arithmetic, insufficient value bytes, an invalid acknowledgement tail, an unrecognized command, or a recognized non-configuration command
- **THEN** the component returns the matching bounded structural or unsupported-command failure and exposes no partial response or payload diagnostic

### Requirement: Rust VESC BMS values behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to structurally decoding bounded `COMM_BMS_GET_VALUES = 96` payloads.

#### Scenario: Preserve complete BMS values

- **WHEN** a bounded command-96 response is decoded
- **THEN** it SHALL decode all fixed electrical, environmental, state, lifetime, pressure, reported-CAN, bounded cell, balancing, temperature, and optional extended-tail fields without owning observation time, route scheduling, discovery, polling, availability, application state, or persistence.
- **AND** the component SHALL preserve signed big-endian fixed-point and VESC auto-float rules; zero-through-50 cell and temperature counts; exact boolean flags; reported CAN IDs through 253 and optional route matching; exact legacy termination; one-byte data version and valid UTF-8 status through 40 content bytes for the extended form; exact remainder consumption; finite numeric results; and unsupported-command versus structural failure behavior. It SHALL use statically bounded collection storage, forbid unsafe code, log no payload or decoded values.

#### Scenario: Decode exact source-derived BMS values

- **WHEN** the component receives either committed legacy or extended VBMS/VBMS32 command-96 payload
- **THEN** it returns every fixed value, array entry, flag, reported CAN ID, data version, and status with the same units and values as established production behavior

#### Scenario: Accept exact collection and tail bounds

- **WHEN** the component receives zero or 50 cells and temperatures, valid balancing flags, and absent or maximum 40-byte extended status
- **THEN** it returns complete bounded counts and values without allocation proportional to untrusted input

#### Scenario: Reject malformed BMS values

- **WHEN** input is empty, truncated, has another command, excessive counts, a non-boolean flag, a non-finite or excessive auto-float, invalid or mismatched reported CAN ID, malformed UTF-8, unterminated or oversized status, or unexpected remainder
- **THEN** the component returns the matching bounded structural or unsupported-command failure and exposes no partial values or payload diagnostic

### Requirement: Rust Refloat control codec behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to encoding bounded Refloat package, flywheel, legacy/stable lighting, and fatal-clear custom-app-data payloads and decoding bounded lighting responses.

#### Scenario: Preserve control wire semantics

- **WHEN** a supported control request or lighting response is encoded or decoded
- **THEN** it SHALL accept typed caller-provided control profile and lighting state values without owning compatibility lookup, VESC framing, CAN routing, mutation authorization, scheduling, confirmation, telemetry, application state, or persistence.
- **AND** the component SHALL preserve VESC command `36`, Refloat package ID `101`, exact Refloat command and parameter bytes, inverted package enable semantics, flywheel mode/default/footpad-abort bytes, lighting profile and mask/flag behavior, zero-mask query behavior, fatal-clear availability, exact four-byte response extent, defined lighting bits, and unsupported-command versus malformed failure behavior. It SHALL return statically bounded payloads, forbid unsafe code, log no payload or state.

#### Scenario: Encode exact source-derived controls

- **WHEN** the component receives each committed package, flywheel, legacy/stable lighting, or fatal-clear input
- **THEN** it returns the exact source-derived custom-app-data payload the immutable source-derived fixtures define

#### Scenario: Preserve lighting state and query semantics

- **WHEN** the component encodes either lighting profile for every current-state, target, and enabled combination or for no target
- **THEN** it changes only the selected bit, preserves the other bit, uses the exact mask, and emits a zero-mask query without changing the supplied flags

#### Scenario: Decode exact lighting responses

- **WHEN** the component receives an exact four-byte response for the supplied legacy or stable profile with any defined two-bit lighting state
- **THEN** it returns the matching LED and headlight booleans

#### Scenario: Reject unavailable or malformed controls

- **WHEN** fatal clear is unavailable or a lighting response has an empty, short, long, wrong-prefix, wrong-package, wrong-command, wrong-profile, or undefined-flag form
- **THEN** the component returns the matching bounded unsupported-command or malformed failure and exposes no partial state or payload diagnostic

### Requirement: Rust VESC controller-data behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to decoding the consumed standard VESC `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset.

#### Scenario: Preserve controller field types

- **WHEN** controller values are decoded
- **THEN** it SHALL accept unframed payloads without owning request construction, firmware compatibility, VESC framing, CAN routing, scheduling, telemetry polling, application state, or persistence.
- **AND** the component SHALL preserve command validation; big-endian signed fixed-point scales; the consumed 30-byte realtime prefix and permitted later fields; raw fault code and none, over-voltage, or unknown classification; VESC auto-float decoding; four finite nonzero current limits; the closed supported serialization-signature to battery-series-cell-offset table; and optional behavior for unknown signatures, absent offsets, and zero cell counts. It SHALL forbid unsafe code, expose only bounded payload-safe failures.

#### Scenario: Decode exact realtime values

- **WHEN** the component receives each committed normal or faulted command-4 payload
- **THEN** it returns the exact temperatures, currents, duty, ERPM, voltage, raw fault code, and stable fault classification the immutable source-derived fixtures define

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

### Requirement: Rust Refloat legacy telemetry behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to decoding complete unframed Refloat custom-app-data command-10 compact telemetry.

#### Scenario: Preserve legacy field semantics

- **WHEN** legacy Refloat telemetry is decoded
- **THEN** it SHALL accept the exact active-firmware-fault form and detail levels 1 through 4 without owning VESC framing, direct/CAN routing, compatibility, polling, dynamic field negotiation, application state, persistence, or transport.
- **AND** the component SHALL preserve the VESC custom-app-data command and Refloat package/command identifiers; exact form and detail-level lengths; big-endian signed scales; one metres/second-to-kilometres/hour conversion; finite VESC auto-float distance; legacy package state, mode, footpad, stop, adjustment, alert, and firmware-fault mappings; all fixed and optional detail fields; and raw unknown fault classification. It SHALL expose the source `222` field-weakening sentinel as no value, forbid unsafe code, return allocation-free fixed-size output, expose only bounded payload-safe failures.

#### Scenario: Decode every compact detail level

- **WHEN** the component receives each committed source-derived command-10 detail-level 1 through 4 payload
- **THEN** it returns every field present at that level with exact units and semantics while fields belonging only to later levels remain absent

#### Scenario: Decode an active firmware fault

- **WHEN** the component receives the exact five-byte command-10 fault form with a nonzero known or unknown fault code
- **THEN** it returns the raw code and stable firmware-fault classification without fabricating a normal telemetry sample

#### Scenario: Preserve unavailable field weakening

- **WHEN** a valid compact response carries upstream sentinel byte `222` in the field-weakening position
- **THEN** the component returns no field-weakening current rather than reporting the sentinel as a measured value

#### Scenario: Reject malformed legacy telemetry

- **WHEN** a payload has the wrong envelope, package, or command; an unsupported detail level; a zero fault code; an inexact form length; a truncated field; or a non-finite distance value
- **THEN** the component fails closed with a stable bounded error and exposes no partial values or payload diagnostic

### Requirement: Rust Refloat dynamic telemetry behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to decoding complete unframed Refloat custom-app-data command-32 layouts and command-31 dynamic telemetry.

#### Scenario: Preserve negotiated field semantics

- **WHEN** a negotiated telemetry response is decoded
- **THEN** it SHALL own bounded negotiated layout representation, exact mask-dependent response structure, the source float16 format, semantic state and alert classifications, typed known fields, and bounded unknown numeric extensions without owning VESC framing, compatibility selection, direct/CAN routing, polling, layout lifetime across reconnect, compact-cache merging, application state, persistence, or transport.
- **AND** the component SHALL accept at most 64 globally unique nonempty UTF-8 identifiers, at most 64 encoded bytes per identifier, at most 4,096 aggregate identifier bytes, and at most 32 unknown extensions. It SHALL decode the source format's complete finite `+-131008` float16 range, preserve negotiated order, source ticks, running-only values, charging data, the 1.2.x two-word active-alert tail, fatal latch, raw firmware-fault code, and stable classifications, return no partial layout or sample, expose only bounded payload-safe failures, forbid unsafe code.

#### Scenario: Decode a complete bounded layout

- **WHEN** the component receives a source-derived or future-bounded command-32 layout
- **THEN** it returns the exact ordered always-present and running-only identifiers only after validating the complete envelope, UTF-8, extents, aggregate bound, count bound, and global uniqueness

#### Scenario: Decode every dynamic response form

- **WHEN** the component receives a valid 1.1.x or 1.2.x command-31 response against its accepted layout
- **THEN** it returns exact source ticks, semantic state, all negotiated known values, bounded extensions, running-only presence, charging values, active alerts, fatal latch, and firmware-fault information required by that form

#### Scenario: Preserve the complete source float16 range

- **WHEN** a negotiated value uses zero, a subnormal, a normal, exponent `0x1f`, or either finite saturation endpoint
- **THEN** the component returns the source-defined finite value with the correct sign and does not apply IEEE infinity or NaN semantics

#### Scenario: Reject malformed dynamic telemetry

- **WHEN** a layout or realtime payload has the wrong envelope, invalid UTF-8, empty or duplicate name, excessive bound, unsupported mask, inexact extent, impossible alert/fault relation, or excessive extension set
- **THEN** the component fails closed with a stable bounded error and exposes no partial identifiers, values, or payload diagnostic

### Requirement: Rust Refloat compatibility behavior

The `backends/core` workspace SHALL contain one safe Rust component dedicated to constructing the unframed Refloat INFO request, decoding bounded INFO response identity, selecting exact audited compatibility and control profiles, and admitting required dynamic telemetry identifiers.

#### Scenario: Resolve audited compatibility

- **WHEN** a Refloat identity is resolved
- **THEN** it SHALL own no VESC framing, direct/CAN routing, request scheduling, connection generation, discovery state, telemetry value decoding, application state, persistence, transport, or OS capability.
- **AND** the component SHALL preserve command `36`, package ID `101`, INFO command `0`, requested protocol version `2`, zero request flags, legacy compact wire identity without an invented patch, version-2 major/minor/patch offsets, the exact audited source-release catalogue, closed legacy/dynamic/dynamic-alert/selectable telemetry profiles, legacy/stable lighting profiles, fatal-clear availability, and the exact required always-present and running-only dynamic identifiers. It SHALL enforce the existing 4,096-byte payload bound and bounded layout names, return no partial result, expose only payload-safe failures, forbid unsafe code.

#### Scenario: Construct and decode source INFO forms

- **WHEN** the component constructs an INFO request or decodes a valid minimal/source legacy compact or complete version-2 response
- **THEN** it returns the exact request bytes and structural wire identity, preserving legacy `1.0` without fabricating patch `.0`

#### Scenario: Select every audited profile exactly

- **WHEN** the component looks up any audited wire identity for every audited source release, including the retained older wire families
- **THEN** it returns the exact source entries, telemetry family, lighting profile, and fatal-clear availability without range inference

#### Scenario: Admit a complete bounded dynamic layout

- **WHEN** both identifier groups contain every exact required name plus any bounded unique future names in any order
- **THEN** the component accepts the layout without moving value decoding, layout lifetime, or readiness state into the component

#### Scenario: Reject unsupported or malformed compatibility input

- **WHEN** an INFO payload has a wrong envelope, unsupported protocol byte, truncated version-2 prefix, excessive extent, or a layout has invalid bounds, duplicates, a missing required identifier, or an identifier in the wrong group
- **THEN** the component fails closed with a stable bounded error and exposes no partial identity, profile, layout decision, or payload diagnostic

### Requirement: Rust Refloat configuration codec behavior

The bounded Rust component SHALL own the established Refloat configuration schema and value semantics.

#### Scenario: Preserve complete schema semantics

- **WHEN** a schema or value set is normalized
- **THEN** it SHALL decode the exact Qt-compressed and expanded schema forms, validate and normalize the closed field/group model, calculate the canonical source fingerprint and wire signature, and decode or encode complete typed value sets with the same bounded deterministic results as the immutable source-derived vectors. It MUST return closed payload-free failures, expose no partial result, forbid unsafe code in component-owned source, and retain no raw schema or configuration bytes after normalization.

#### Scenario: Decode every audited schema

- **WHEN** the component receives each immutable source-derived audited Refloat Qt-compressed schema
- **THEN** it produces the exact fingerprint, signature, ordered fields, supported types and encodings, defaults, bounds, scale, choices, sanitized descriptions, groups, and fixture counts

#### Scenario: Round-trip every audited value family

- **WHEN** the component decodes each release's default, current, accepted-write, and expected-readback bytes against its exact schema
- **THEN** it validates the signature and complete serialized order and re-encodes the exact canonical bytes without a remainder or fabricated value

#### Scenario: Reject hostile schema input

- **WHEN** a compressed envelope, zlib stream, XML document, field definition, serialization order, grouping structure, metadata value, or resource extent violates a canonical bound or invariant
- **THEN** the component returns the corresponding closed failure without input-bearing diagnostics, partial schema publication, unbounded allocation, entity expansion, or host callback

#### Scenario: Reject hostile values

- **WHEN** values have a mismatched signature, truncation, remainder, invalid UTF-8 or terminator, non-finite or out-of-bounds number, unencodable fixed point, invalid enum/bitfield/boolean, wrong type, or missing or extra field
- **THEN** decode or encode fails before any partial value set or write payload is published

### Requirement: Bounded safe Refloat configuration dependencies

The component SHALL use only locked maintained safe Rust dependencies for zlib decompression, streaming XML processing, and SHA-256 hashing.

#### Scenario: Keep schema dependencies focused

- **WHEN** the schema codec is built or audited
- **THEN** the build SHALL use a pure-Rust decompression backend, SHALL work through the repository's offline locked verification path, and SHALL audit the exact direct dependency set and transitive graph. Component-owned source MUST implement no decompressor, XML tokenizer, cryptographic hash, host ABI, filesystem access, networking, scheduling, persistence, or OS integration.

#### Scenario: Verify locked dependency ownership

- **WHEN** workspace dependency and unsafe-code audits inspect the component and its locked graph
- **THEN** only the approved decompression, XML, and SHA-256 libraries and their audited transitive dependencies are present, component source forbids unsafe code, and no native zlib or platform framework is linked

#### Scenario: Build and test offline

- **WHEN** focused and complete repository gates run without dependency resolution or network access
- **THEN** the exact locked component builds, lints with warnings denied, and passes all source-derived and adversarial tests reproducibly

### Requirement: Rust Refloat configuration service behavior

The bounded Rust component SHALL own the established Refloat configuration lifecycle and safety decisions above the existing Rust codec.

#### Scenario: Preserve configuration lifecycle

- **WHEN** the configuration owner advances an operation
- **THEN** it SHALL own the audited release catalogue, schema paging and assembly, cache admission decisions, generation and operation matching, flywheel restrictions, current/default revisions, write construction, acknowledgement-to-verification sequencing, normalized readback comparison, conflict classification, failure recovery, and reset behavior through deterministic platform-neutral facts, events, effects, and snapshots.

#### Scenario: Begin from cache miss or valid cache

- **WHEN** begin facts describe an eligible direct or CAN controller generation with no cache, a valid audited cached schema, an unaudited cached schema, or corrupt cached bytes
- **THEN** the component SHALL deterministically select bounded schema paging or the exact current-value request
- **AND** it SHALL emit the exact store/remove/no-cache decision without reading or writing host storage itself

#### Scenario: Assemble and admit an audited schema

- **WHEN** exact operation- and generation-matched schema chunks arrive in order within every declared extent
- **THEN** the component SHALL assemble no more than the codec bound, decode through the existing safe codec, resolve the exact reported release and full fingerprint/signature, emit a cache-store decision where eligible, and request current values unless flywheel mode forbids it

#### Scenario: Read current and default values

- **WHEN** matched current or default value bytes arrive against the selected schema
- **THEN** the component SHALL decode them through the safe codec, increment one monotonic revision, publish the exact source and complete typed values, and clear only the completed operation

#### Scenario: Verify one configuration write

- **WHEN** a writable normal-mode service receives an exact identity, base revision, and complete typed value set followed by a matching acknowledgement and authoritative readback
- **THEN** the component SHALL encode through the safe codec, sequence write then verification effects under one operation, normalize only the audited non-legacy metadata field, publish the verified current revision, and return success only for an exact normalized match

#### Scenario: Enforce state and flywheel safety

- **WHEN** callers request reads or writes while unavailable, loading, busy, stale, read-only, identity-mismatched, revision-mismatched, or in flywheel mode
- **THEN** the component SHALL return the exact closed native-equivalent failure without emitting host work or partial state
- **AND** entering flywheel SHALL invalidate current values and cancel only an automatic current read while leaving schema/default state bounded and intact
- **AND** exiting flywheel SHALL request one fresh current read only when the connection remains eligible and idle

#### Scenario: Reject stale, malformed, and adversarial sequences

- **WHEN** chunks, values, acknowledgements, verification responses, failures, aborts, generations, operations, routes, indexes, commands, lengths, offsets, or ordering violate the active transition
- **THEN** the component SHALL fail closed with a stable payload-free error, preserve bounded valid state where specified, and never publish partial schema, values, payload, cache bytes, or effects

#### Scenario: Reset and recover deterministically

- **WHEN** an active operation fails, aborts, reconnects, or resets across generation and operation-counter boundaries
- **THEN** the component SHALL match native availability/current/pending recovery semantics, reject all late work, clear connection-scoped data, and produce the same snapshot and effects on repeated equivalent sequences

### Requirement: Rust VESC request scheduler behavior

The repository SHALL provide a safe, bounded Rust component for the complete connection-wide VESC request scheduling state machine. The component SHALL preserve priority and FIFO arbitration, one response-bearing request in flight, one-way completion, typed response attribution, queue limits, generations, cancellation, explicit deadline expiry, deferred pumping, stable outcomes, and ordered host effects without depending on host execution.

#### Scenario: Arbitrate bounded work

- **WHEN** requests of every priority and equal-priority sequence are enqueued around one in-flight request
- **THEN** the component SHALL preserve the non-preemptive in-flight request, select queued work by highest priority then FIFO order, and reject only work beyond the exact bounded queue capacity

#### Scenario: Attribute a response exactly

- **WHEN** a response, route, Refloat command, or typed operation context does not match the current request and generation
- **THEN** the component SHALL leave that request in flight and SHALL NOT complete or advance unrelated work

#### Scenario: Replace a connection generation

- **WHEN** a new generation begins with queued or in-flight work and stale responses or deadlines arrive afterward
- **THEN** the component SHALL cancel the old work exactly once, reject old-generation enqueue attempts, and prevent stale events from affecting new-generation work

#### Scenario: Complete one-way and response work in order

- **WHEN** one-way, response-bearing, cancelled, timed-out, queue-full, and deferred requests transition
- **THEN** the component SHALL emit the same stable completion outcomes and ordered deadline, completion, diagnostic, and next-write effects as the source native behavior

#### Scenario: Remain total at adversarial bounds

- **WHEN** payload, queue, route, timing, sequence, generation, and operation values reach valid boundaries or malformed values exceed them
- **THEN** the component SHALL remain bounded and deterministic, use explicit wrapping only for source-equivalent counters, and reject invalid facts without panicking or partially mutating accepted state

### Requirement: Rust Refloat telemetry service behavior

The repository SHALL provide one safe bounded Rust component for the Refloat telemetry application service.

#### Scenario: Preserve telemetry service boundaries

- **WHEN** the telemetry owner advances polling or publication
- **THEN** the component SHALL compose the existing Rust legacy/dynamic/selectable decoded telemetry models and request-scheduler facts to own layout retry, route-correct request construction, release-selected legacy/realtime/compact work, pending-response attribution, timeout recovery, adaptive cadence, compact-cache merging, normalized samples, session sequence and elapsed time, and ordered host effects.

#### Scenario: Negotiate a dynamic layout

- **WHEN** layout negotiation begins and receives timer ticks without establishment
- **THEN** the component SHALL emit one immediate and at most four later route-correct command-32 requests at the established priority/deadline, then cancel retry work and report the stable timeout exactly once

#### Scenario: Select serialized telemetry work

- **WHEN** legacy or dynamic polling starts, repeats, times out, or makes compact data due
- **THEN** the component SHALL retain at most one semantic telemetry request pending and emit exact direct/CAN request bytes and scheduler facts for the next legacy, realtime, or compact request without a parallel lane or immediate timeout retry

#### Scenario: Adapt ride cadence

- **WHEN** accepted samples transition among idle, riding, and the inclusive post-ride tail
- **THEN** the component SHALL select the established 100 ms riding cadence or configured idle cadence and emit timer replacement plus immediate next-request effects in source-equivalent order

#### Scenario: Merge normalized telemetry

- **WHEN** legacy samples, dynamic compact samples, dynamic realtime samples, decreasing compact totals, bounded extensions, or active firmware faults are accepted
- **THEN** legacy publishable data SHALL normalize directly, compact data SHALL replace the route-scoped cache without publication, realtime data SHALL merge only the latest slow fields and observation time, and published samples SHALL preserve every semantic field with monotonic source-equivalent sequence and nonnegative elapsed time

#### Scenario: Reject stale or mismatched input

- **WHEN** a response kind, scheduler completion, timer token, service generation, route, or telemetry family does not match current state
- **THEN** the component SHALL not clear, publish, retry, or advance unrelated work and SHALL return only the stable bounded diagnostic where native behavior reports one

#### Scenario: Remain total at public bounds

- **WHEN** intervals, tail durations, monotonic or observation times, sequences, tokens, generations, routes, extensions, and decoded optional values reach valid limits or invalid input exceeds them
- **THEN** the component SHALL remain bounded and deterministic, reject invalid input without partial mutation, and use explicit wrapping only for source-equivalent counters

### Requirement: Rust VESC compatibility behavior

The repository SHALL provide one safe bounded Rust component that consumes the existing safe firmware-identity model and owns the exact VESC motor-controller compatibility catalogue, access-mode decision, operation authorization, and shutdown availability.

#### Scenario: Admit only exact audited motor firmware

- **WHEN** a structurally decoded identity has any hardware type and any major/minor pair, including the complete byte-domain boundaries
- **THEN** the component SHALL admit only motor hardware type `0` at exactly 6.02, 6.05, 6.06, or 7.00 and SHALL reject every adjacent, unknown, gateway, BMS, or custom-module identity

#### Scenario: Classify audited stable and test access

- **WHEN** an admitted identity reports a zero or nonzero test marker
- **THEN** both select read/write compatibility, without bypassing per-operation safety gates or using unrelated identity fields to broaden admission

#### Scenario: Authorize operations

- **WHEN** a read or guarded mutation is requested on audited stable/test firmware or unsupported firmware
- **THEN** compatible reads and guarded writes remain eligible on admitted identities while unsupported identities permit neither; mutation safety is still evaluated by its shared owner

#### Scenario: Authorize exact shutdown releases

- **WHEN** shutdown compatibility is requested
- **THEN** it is available only for audited motor firmware 6.05, 6.06 and 7.00, including test builds, and unavailable for 6.02 or unsupported identities; readiness, idle, generation and mutation-exclusion gates still apply

#### Scenario: Remain total over identity bounds

- **WHEN** every compatibility input and unrelated bounded identity field reaches valid limits
- **THEN** all public decisions SHALL remain deterministic and panic-free and private identity material SHALL remain absent from diagnostics

### Requirement: Rust controller-discovery behavior

The repository SHALL provide one safe bounded Rust component for the direct/CAN controller-discovery decision state machine. The component SHALL directly compose safe firmware identity, VESC compatibility, and Refloat compatibility models to own route admission, configured and learned recovery, enumeration, continuity, candidate selection, readiness, reset, and stable failure decisions without host execution.

#### Scenario: Select a direct controller

- **WHEN** the transport identity is a motor controller at an exactly supported stable or test firmware version
- **THEN** the component SHALL select the direct route, retain the identity and access decision, and request Refloat identification without CAN discovery

#### Scenario: Reject unsupported direct firmware

- **WHEN** a direct motor identity has an unsupported version or a non-motor endpoint cannot establish a controller route
- **THEN** the component SHALL return the exact stable unsupported/no-controller failure without admitting a different identity or exposing private fields

#### Scenario: Recover configured and learned routes

- **WHEN** a gateway has an authoritative preferred CAN ID or a meaningful learned UUID hint
- **THEN** the component SHALL probe the exact route first, enforce configured UUID continuity, accept a matching learned hint, and clear then enumerate after a stale or timed-out learned hint without falling back from an authoritative preference

#### Scenario: Enumerate a bounded CAN network

- **WHEN** CAN discovery supplies duplicates, IDs outside `0...253`, mixed hardware, missing responses, and valid controller identities
- **THEN** the component SHALL normalize and sort at most 64 unique valid IDs, probe each sequentially at most once, retain a complete bounded classified inventory, and never select a non-motor node

#### Scenario: Enforce enumeration bounds

- **WHEN** normalized discovery exceeds 64 CAN IDs or probing finds a seventeenth motor controller
- **THEN** the component SHALL fail with `too_many_can_devices` or `too_many_controllers`, publish no truncated successful result, and retain no unbounded raw input

#### Scenario: Resolve continuity and ambiguity

- **WHEN** enumeration completes with zero, one, or multiple controllers during a fresh connection or active-session recovery
- **THEN** the component SHALL reproduce UUID-first continuity, supported prior-route fallback only when UUID is absent, controller-changed failures, sole-controller admission/rejection, and sorted display-safe candidate publication

#### Scenario: Select only a current supported candidate

- **WHEN** selection names a current supported candidate, an unsupported candidate, an unknown ID, or stale state
- **THEN** the component SHALL resume Refloat identification only for the valid candidate and return a stable invalid-state result without mutation for every invalid selection

#### Scenario: Complete Refloat readiness

- **WHEN** an admitted controller receives an audited legacy or dynamic Refloat profile and a dynamic-layout admission result
- **THEN** legacy SHALL become ready directly, dynamic SHALL request layout then become ready only after successful admission, and unsupported input SHALL fail closed

#### Scenario: Isolate generations and reset

- **WHEN** begins, responses, probe timeouts, identification timeouts, selections, failures, and resets are repeated, reordered, stale, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale facts SHALL NOT alter current state, and reset SHALL clear all connection state while invalidating old work

### Requirement: Rust auxiliary-node discovery behavior

The repository SHALL provide one safe bounded Rust component for the auxiliary VBMS discovery decision state machine. The component SHALL directly compose the safe controller-discovery route and inventory model to own exact BMS admission, direct/preferred/automatic route selection, inventory reuse, bounded fallback enumeration, availability, observation retention, reset, and stable failure decisions without host execution.

#### Scenario: Select an exact direct BMS

- **WHEN** the local routed identity is exact stock VESC BMS 6.00 or VBMS32 custom-module 6.06
- **THEN** the component SHALL select the direct route and request values without CAN discovery

#### Scenario: Reject an unaudited identity

- **WHEN** a local or CAN identity has another hardware type, hardware name, version, or malformed route attribution
- **THEN** the component SHALL NOT retain or poll it as a BMS and SHALL expose no private identity field

#### Scenario: Reuse a complete controller inventory

- **WHEN** current controller discovery supplies a complete routed inventory containing zero, one, or multiple exact BMS identities
- **THEN** the component SHALL classify that inventory without another CAN ping or firmware probe and resolve not-found, unique selection, or ambiguity deterministically

#### Scenario: Enumerate an incomplete inventory

- **WHEN** no complete reusable inventory exists and no direct BMS is selected
- **THEN** the component SHALL request one CAN ping, normalize at most 64 unique IDs in `0...253`, and probe each route sequentially at most once

#### Scenario: Honor an exact preferred route

- **WHEN** a valid preferred BMS CAN ID is configured
- **THEN** the component SHALL use a matching complete-inventory identity or probe that exact route directly, select it only when compatible, and fail unavailable without falling back when it is missing, incompatible, or timed out

#### Scenario: Enforce discovery bounds

- **WHEN** normalized discovery exceeds 64 CAN IDs or classification finds a seventeenth exact BMS
- **THEN** the component SHALL become unavailable with a stable bounded issue, select no partial route, and retain no unbounded raw input

#### Scenario: Retain and recover observation state

- **WHEN** a selected route receives a valid current observation, becomes unavailable, and later receives another valid current observation
- **THEN** it SHALL become attached, retain the prior observation as stale while unavailable, and replace it while recovering without inspecting or logging the observation payload

#### Scenario: Isolate generations and reset

- **WHEN** begins, CAN results, firmware results, timeouts, observations, failures, and resets are repeated, reordered, stale, invalid, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale work SHALL NOT alter current state, and reset SHALL clear all auxiliary state while invalidating old work

### Requirement: Rust VESC BMS telemetry service behavior

The repository SHALL provide one safe bounded Rust component for the VESC BMS polling/application lifecycle. The component SHALL directly compose Rust auxiliary discovery, command-96 decoding, and request-scheduler facts to own discovery execution facts, selected-route polling, concrete observations, completion-driven cadence, failure accounting, unavailability, recovery, generation isolation, reset, and ordered host effects.

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

### Requirement: Rust connection application coordinator behavior

The repository SHALL provide one safe bounded Rust component for the connection-generation application spine. The component SHALL directly compose Rust controller discovery, request scheduling, Refloat telemetry, and VESC BMS telemetry to own start/reset, request attribution, discovery sequencing, readiness activation, child lifecycle, and ordered runtime effects.

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

### Requirement: Rust device mutation coordinator behavior

The repository SHALL provide one safe bounded Rust component for the connection-scoped device mutation authorization and serialization owner. The component SHALL directly use safe compatibility, control-codec, normalized-telemetry, firmware-identity, and scheduler-fact models to own current authoritative mutation state without a foreign bridge.

#### Scenario: Configure one exact generation

- **WHEN** a current checked controller route, structurally decoded firmware identity, admitted Refloat compatibility profile, and readiness facts configure the component
- **THEN** it SHALL derive writable and shutdown capability through safe Rust policy, replace all prior-generation state atomically, and publish one bounded loading control snapshot with no request

#### Scenario: Establish authoritative control state

- **WHEN** current monotonic normalized telemetry and exact profile-matched lighting observations arrive
- **THEN** package, flywheel, fatal-latch, active-fault, clearability, LED, and headlight state SHALL converge to the same ready/loading snapshot and confirmation order as the shared application contract

#### Scenario: Authorize every semantic control

- **WHEN** package, flywheel, lighting, or fatal-clear is requested against current readiness, compatibility, access, ride, package, footpad, fault, and pending state
- **THEN** the component SHALL either return an exact idempotent snapshot with no request or one generation-qualified mutation token and exact route-correct scheduler request facts matching the shared control contract

#### Scenario: Preserve the flywheel safety exception

- **WHEN** authoritative package mode is flywheel
- **THEN** every mutation SHALL be rejected except exact eligible flywheel disable, which SHALL remain available while normalized ride state is riding and SHALL confirm only after later normal-mode telemetry

#### Scenario: Serialize controller shutdown

- **WHEN** shutdown is requested on a current ready idle exact supported writable controller with no competing mutation
- **THEN** the component SHALL reserve the sole pending slot and return one exact direct or once-forwarded accepted-only shutdown request while unsupported, moving, flywheel, stale, read-only, or competing states emit no request

#### Scenario: Serialize configuration and Tune work

- **WHEN** an edited configuration write or Tune application meets its release-specific readiness and state gates with no competing mutation
- **THEN** the component SHALL return one exact generation/sequence-qualified lease excluding controls and shutdown until verified finish, failure, cancellation or reset
- **AND** edited writes retain the idle/ready restriction, while eligible Tune application follows the [normal-mode riding policy](../refloat-configuration/spec.md#requirement-exact-version-saved-configurations); pressed, unknown or missing footpad readings restrict neither operation

#### Scenario: Confirm, fail, and reset exactly

- **WHEN** authoritative observations, request failure, lease completion, shutdown clearing, cancellation, or connection reset occur
- **THEN** only the exact current token and generation SHALL alter pending state, confirmations SHALL follow source order, and stale, repeated, mismatched, or prior-generation events SHALL remain inert

#### Scenario: Remain total at public bounds

- **WHEN** routes, identities, generations, sequences, sample sequences, payloads, profiles, observations, and transition order reach valid limits or invalid input exceeds them
- **THEN** the component SHALL remain bounded and deterministic, reject invalid input without partial mutation, wrap only the explicit token counter, and expose no private firmware or telemetry payload through diagnostics

### Requirement: Composed Rust device configuration application lifecycle

The safe Rust connection application coordinator SHALL directly own and compose the Rust device mutation coordinator and Refloat configuration service with its existing generation, scheduler, discovery, telemetry, and BMS owners. The composition SHALL preserve the selected native authorization, serialization, request attribution, response handling, cache intent, confirmation, failure, and reset behavior without a foreign bridge.

#### Scenario: Activate one selected controller generation

- **WHEN** current discovery reaches a supported controller and the shared product store activates the exact awaiting generation with an optional exact cache value
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

### Requirement: Safe Rust session-history reduction kernel

The repository SHALL provide one safe bounded Rust component for historical series reduction. The component SHALL consume ordered platform-neutral rows incrementally and own exact M4 role selection, null/gap continuity generations, and deterministic bounded column publication.

#### Scenario: Configure bounded reduction

- **WHEN** a caller supplies 1–16 unique bounded field names, an inclusive nonnegative elapsed range, 1–1,024 buckets, a valid explicit or automatic gap policy, positive persistence cadence, and bounded retained resolution
- **THEN** the component SHALL allocate only field/bucket-bounded state and preserve the supplied field order

#### Scenario: Select M4 roles

- **WHEN** ordered finite observations populate one or more buckets
- **THEN** each field/bucket SHALL retain the chronologically first and last rows plus the minimum and maximum finite values, resolve equal roles by earliest `(elapsedMs, sequence)`, deduplicate multi-role source rows, and publish candidates in chronological order

#### Scenario: Preserve discontinuities

- **WHEN** a value is absent, an elapsed delta exceeds the explicit threshold, or an automatic phase/cadence/resolution threshold is crossed
- **THEN** the affected continuity generation SHALL advance exactly and later values SHALL NOT be connected across that boundary

#### Scenario: Reject invalid ordered input

- **WHEN** configuration or input has duplicate/oversized fields, invalid ranges, invalid counts, non-finite present values, mismatched value arity, out-of-range rows, descending or duplicate keys, invalid cadence/thresholds, arithmetic overflow, generation exhaustion, or more than 65,536 selected observations
- **THEN** the component SHALL return one stable payload-free error without panicking, partially publishing a result, or retaining unbounded input

### Requirement: Safe Rust session prune-selection kernel

The Rust session-history component SHALL select retained real source rows from an ordered stream using elapsed-zero-aligned buckets, protected semantic evidence, deterministic extrema coverage, and an explicit cap of eight ordinary rows per bucket. Its live memory SHALL remain bounded by field count and the current bucket rather than ride length.

#### Scenario: Retain protected evidence

- **WHEN** a session stream contains endpoints, first non-null field sources, both sides of riding-phase transitions, caller-marked event sources, caller-marked summary sources, or other protected rows
- **THEN** every such source row SHALL be retained independently of the ordinary eight-row bucket cap

#### Scenario: Cover ordinary bucket roles

- **WHEN** an elapsed bucket contains unprotected first/last and ordered per-field minimum/maximum candidates
- **THEN** the component SHALL select deterministic maximum-coverage rows with chronological tie-breaking, retain no more than eight ordinary rows, and report any roles left uncovered by that cap

#### Scenario: Finalize incrementally

- **WHEN** ordered input advances into another elapsed-zero-aligned bucket or the caller finishes the stream
- **THEN** the component SHALL return the finalized prior selection in chronological order and release its raw bucket rows before accepting unbounded later history

#### Scenario: Preserve aligned coarser behavior

- **WHEN** previously retained source rows are selected again at a strictly coarser aligned integer-multiple resolution
- **THEN** protected evidence SHALL remain present and the deterministic result SHALL introduce no row absent from the supplied retained source

#### Scenario: Reject invalid prune input

- **WHEN** field, resolution, ordering, sequence, elapsed, value, bucket-width, counter, or role bounds are invalid or exhausted
- **THEN** the component SHALL fail closed with a stable payload-free error and SHALL publish no partial current bucket

### Requirement: Session-history kernel remains below persistence and runtime execution

The Rust session-history component SHALL expose only safe Rust configuration, row, column, selection, issue, and ordered-finalization values. It MUST NOT own or call SQLite, SQL, a persistence port, cursor JSON/base64, a local server, frontend code, BLE, callbacks, foreign handles, bridge records, threads, filesystems, sockets, async runtimes, or OS frameworks.

#### Scenario: Compose SQLite directly

- **WHEN** the Rust session store reads ordered SQLite rows or performs prune selection
- **THEN** it SHALL call this safe kernel directly and persist returned facts without a session-history Apple FFI or Android JNI boundary

### Requirement: Rust canonical session SQLite writer

The repository SHALL provide one bounded Rust component that creates and reopens a canonical per-session SQLite file and atomically persists normalized telemetry, derived field/event evidence, commit summaries, and location observations.

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

The Rust session SQLite package SHALL own independent writer, reader, and pruner connections and expose only safe owned creation, open, append, marker, checkpoint, close, metadata, field, bounded query, reduction, page-key, prune plan/result, and interruption values.

#### Scenario: Keep session persistence focused

- **WHEN** session storage is composed by the shared service
- **THEN** it MUST NOT own catalog SQLite, settings, profiles, opaque cursors, prune scheduling or eligibility policy, imports/exports, deletion/retention, recovery policy, local serving, gzip/JSON envelopes, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose persistence directly

- **WHEN** the shared catalog/application runtime creates, records, queries, or prunes a session
- **THEN** it SHALL call the safe writer, reader, pruner, and history-kernel APIs directly without session SQLite Apple FFI or Android JNI boundaries

#### Scenario: Bound dependencies and diagnostics

- **WHEN** maintainers inspect the locked graph or any valid/adversarial value reaches errors and debug formatting
- **THEN** dependencies SHALL remain limited to the named safe Rust telemetry/history owners, maintained bundled SQLite/JSON support and their build graph, and output SHALL reveal no path, SQL, telemetry, coordinate, extension, event, JSON, or SQLite-message payload

### Requirement: Safe Rust session SQLite pruning

The Rust session SQLite component SHALL preview and atomically apply deterministic pruning to one validated canonical session database by streaming durable source rows directly through the session-history prune kernel. It SHALL remain bounded, race-safe, interruptible, and independent of catalog or host-runtime policy.

#### Scenario: Preview exact retained evidence

- **WHEN** a valid inactive canonical session is previewed at an initial or strictly coarser aligned resolution
- **THEN** the component SHALL retain exact kernel-selected endpoints, field/event/summary sources, phase boundaries, and deterministic extrema rows while returning recorded, stored, retained, reclaimable, target-resolution, skipped, and immutable-boundary facts without mutating the database

#### Scenario: Apply one current plan atomically

- **WHEN** a non-skipped preview still matches its database marker and retained-resolution facts at commit time
- **THEN** one immediate transaction SHALL delete only unretained samples and locations outside the retained-resolution neighborhood, preserve recorded counts and published sequence maxima, update stored counts and `m4-source-v1` metadata, increment history revision exactly once, and expose no partial state

#### Scenario: Preserve a no-op database

- **WHEN** every stored sample is selected by a still-current preview
- **THEN** commit SHALL report `already_at_or_below_target` without changing rows, prune metadata, counts, commit time, or history revision

#### Scenario: Reject stale or invalid pruning

- **WHEN** target bounds/alignment, schema/data/order/count bounds, database identity, marker facts, or retained resolution are invalid or advance between preview and commit
- **THEN** the component SHALL fail closed with a stable payload-free error before durable mutation and SHALL never silently recompute a different plan

#### Scenario: Roll back failure and interruption

- **WHEN** cancellation, lock contention, SQLite failure, or an injected fault occurs during preview or mutation
- **THEN** the component SHALL promptly release every statement and transaction, preserve the prior durable database exactly, remain reusable where compatible, and reveal no path, SQL, telemetry, coordinate, event, JSON, or SQLite-message payload

### Requirement: Session SQLite pruning remains below runtime policy

The Rust session SQLite pruning API SHALL expose only safe owned plan, result, timestamp, boundary, error, and interruption values. It MUST NOT own catalog SQLite, active-session or read-lease exclusion, maintenance jobs, checkpoint/vacuum scheduling, imports/exports, deletion/retention, recovery, local serving, cursors/envelopes, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose persistence directly

- **WHEN** the shared catalog/application runtime previews or applies session pruning
- **THEN** it SHALL call the session SQLite and history-kernel APIs directly without a pruning Apple FFI or Android JNI boundary
