## Context

The selected Apple and Android backends each own near-identical Refloat configuration codecs: 559 Swift lines and 434 Kotlin lines. Both accept the same four-byte Qt `qCompress` length prefix, zlib stream, `ConfigParams` XML shape, schema limits, field normalization rules, raw-XML SHA-256 fingerprint, CRC32C serialization signature, and signed typed-value wire representation. The already-selected `vesc-custom-configuration` Rust component deliberately stops at commands 92–95 and treats compressed schemas and values as opaque, so this semantic codec is the next downward protocol layer.

The nine immutable Refloat 1.0.0–1.2.3 fixture families contain source-derived compressed schemas, normalized expectations, default/current/write/readback bytes, exact fingerprints, and signatures. Existing native and conformance tests also describe hostile envelopes, XML, metadata, and serialized-value cases. Configuration chunk assembly, source-version catalogue admission, connection generation, flywheel policy, schema cache, revisions, drafts, command scheduling, writes/readback, and durable Tunes live in stateful services above the codec and remain native.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust crate that exactly decodes a bounded Qt-compressed or expanded Refloat schema into an owned platform-neutral model.
- Preserve the complete normalized field/group model, raw validated XML fingerprint, CRC32C wire signature, and every supported field/wire type.
- Decode and encode complete signed value sets with deterministic ordering, exact numeric semantics, strict UTF-8 strings, and closed stable failures.
- Prove every audited release and adversarial boundary independently of either host, while keeping production call paths and artifacts unchanged.
- Use only small maintained safe dependencies whose locked transitive graph is audited and available to offline builds.

**Non-Goals:**

- Adding an Apple FFI or Android JNI crate, selecting Rust in production, or deleting Swift/Kotlin codec code.
- Resolving a schema fingerprint to an exact Refloat package, broadening the accepted release catalogue, or changing configuration behavior.
- Moving command-92 chunk assembly, commands 93–95 envelopes, caching, lifecycle, flywheel policy, drafts, revisions, Tune storage, write verification, scheduling, routing, application state, SQLite, loopback transport, BLE, or any OS-owned capability.
- Generating bindings, sharing host ABI records, or designing the final unified Rust application API in this extraction.

## Decisions

### Extract schema and value semantics together

Add workspace package `float-view-refloat-configuration-codec` at `backends/core/refloat-configuration-codec`. It will expose the constants, closed field/wire/value/error enums, owned field/group/schema records, `decode_qt_compressed`, `parse_schema`, `wire_signature`, `decode_values`, and `encode_values` operations.

Schema parsing and value serialization are one cohesive boundary because the parsed ordered fields, encodings, bounds, choices, scale, and computed signature are the authority for every value operation. Splitting them would duplicate or serialize that model between two Rust crates and create two future host bridges. Moving `RefloatConfigurationService` now was rejected because it combines protocol state with cache, scheduling, lifecycle, application publication, and persistence and therefore requires a separate stateful change.

### Use bounded streaming primitives and an owned ordinary Rust model

`decode_qt_compressed` will validate the four-byte big-endian expanded length, the 32 KiB compressed and 384 KiB expanded limits, then inflate into an exactly bounded caller-returned `Vec<u8>`. It will require zlib stream completion, exact declared output, and consumption of all compressed bytes. `parse_schema` will consume UTF-8 XML with a streaming event reader, reject DTD/entity constructs before model construction, and count depth, elements, fields, groups, subgroups, choices, and accumulated text before allocation growth crosses canonical limits.

The result owns bounded `String` and `Vec` values because later application ownership needs the schema beyond the input call and Rust-to-Rust orchestration should not inherit a host ABI shape. No raw XML or compressed bytes are retained. Errors are a closed enum with payload-free `Display`, no input-bearing `Debug` records, and no partial schema/value result.

Returning a JSON/interchange record was rejected because it would add serialization overhead and prematurely shape temporary bridges. Borrowing strings from expanded XML was rejected because entity decoding, sanitization, and long-lived service ownership require normalized owned values.

### Lock three safe dependencies and keep CRC32C local

Use `flate2` with its pure-Rust backend for bounded zlib processing, `quick-xml` for streaming XML events and unescaping, and `sha2` for SHA-256. These replace platform zlib/XML/crypto APIs and avoid writing security-sensitive decompression, XML tokenization, or cryptography. Pin their resolved versions in `backends/core/Cargo.lock`, build offline in repository gates, deny warnings, and audit the exact direct dependency set plus safe source ownership.

Retain the small table-free CRC32C calculation in the crate because it is approximately ten straightforward lines already shared semantically by both hosts and adding another dependency would not remove comparable complexity. General XML data binding, DOM construction, C zlib bindings, OpenSSL, and generated models were rejected because they expand unsafe/native build surface or allocate the full document tree without improving the narrow codec.

### Preserve native normalization and wire semantics exactly

The schema fingerprint remains lowercase SHA-256 of the complete expanded XML bytes after successful validation. Only fields referenced by `SerOrder` become serialized fields; duplicate definitions/order entries, missing references, unsupported field types or numeric encodings, invalid finite metadata, unbounded names/descriptions/choices, or malformed grouping fail closed. Description markup is stripped and whitespace collapsed as today. Group references remain presentation metadata and do not broaden serialized ownership.

The wire signature remains CRC32C over each ordered field name, decimal field-type identifier, decimal wire-type identifier or zero, and concatenated choice names. Values begin with the exact four-byte signature and then use current big-endian signed/unsigned integer, fixed-point double16/double32, signed-byte enum, byte bitfield/boolean, and NUL-terminated UTF-8 string forms. Decode requires complete consumption; encode requires exactly one correctly typed bounded value per field.

Changing fingerprints to a normalized model digest was rejected because committed release identity and legacy 1.0.x disambiguation use the exact source XML digest. Tightening metadata beyond selected native behavior belongs in a separate behavior change with new fixtures, not a migration parity extraction.

### Keep candidate and selected-path evidence distinct

Focused Rust tests will load every committed schema and value family, compare exact manifest fingerprints/signatures/counts and expected bytes, and exercise synthetic field/wire types and adversarial limits. Repository audits will prove the crate is absent from the Apple package, Android Gradle bridge catalogue, native bridge build catalogue, exported symbols, and production Swift/Kotlin sources.

`just verify` will execute the candidate and unchanged host tests. ESP32 runs on Android, iOS, and macOS will exercise the selected native codec and configuration operations only, proving regression freedom rather than Rust selection. Separate host-cutover changes must later prove production Rust execution and delete their native algorithms.

## Risks / Trade-offs

- **Streaming XML behavior may differ subtly from Foundation or SAX.** → Compare all nine source documents and targeted empty-element, text-splitting, entity, depth, duplicate, grouping, and malformed cases; construct the model only after full validation.
- **Floating-point parsing or rounding may drift across languages.** → Exercise manifest defaults, accepted writes, signed extrema, scale integrality, finite/bounds checks, and byte-exact encode round trips; preserve IEEE-754 `f64` semantics.
- **New dependencies enlarge the locked graph.** → Use only three safe direct dependencies, disable native zlib, audit their exact graph and unsafe boundaries, and keep the crate unselected until host-specific packaging changes.
- **A large owned schema allocates many strings.** → Enforce compressed, expanded, field, group, choice, name, description, text, and value limits before growth and retain only normalized metadata, not raw XML.
- **Native copies can drift while the candidate is unselected.** → Keep all three implementations on the same immutable fixtures and follow with independent Apple then Android cutovers.

## Migration Plan

1. Add the isolated safe crate, locked dependencies, workspace/documentation entry, and production-isolation audits.
2. Implement bounded decompression, XML/schema normalization, fingerprint/signature calculation, and complete typed value decode/encode.
3. Add source-derived and adversarial fixture tests, focused gates, and `just verify` coverage.
4. Run unchanged production-path physical regression on Android, iOS, and macOS with the ESP32 fixture.
5. Leave both native codecs selected; follow with independent Apple and Android cutovers, then migrate `RefloatConfigurationService` as a later stateful Rust slice.

Rollback removes the unselected crate, dependency lock entries, and its audit/documentation targets. No host artifact, persistent state, contract, or user-visible behavior requires migration or reversal.

## Open Questions

None. Host ABI record shapes, cutover order, service ownership, cache representation, and final unified runtime API remain deliberate later changes.
