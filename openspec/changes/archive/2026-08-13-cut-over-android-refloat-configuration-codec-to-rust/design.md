## Context

`refloat-configuration-codec` is now the production Apple owner of Refloat `ConfigParams` decompression/XML normalization, schema fingerprint and wire-signature construction, typed current/default value decoding, and typed write encoding. Its safe crate also owns bounded versioned canonical schema/value records. Android still duplicates those protocol algorithms in `RefloatConfigurationCodec.kt`; its stateful `RefloatConfigurationService` separately owns schema transfers and cache selection, connection generations, revisions, conflicts, scheduling, write/readback verification, Tune persistence, and application publication.

The next boundary should select the already-proven safe codec for Android without moving that stateful service. It must fit the existing typed Android bridge build system, preserve the selected Apple path byte-for-byte, and remain visibly temporary because all per-protocol JNI and Apple FFI crates are deleted after the unified Rust runtime owns application behavior.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of Refloat configuration schema and value wire semantics.
- Reuse the exact safe-crate canonical records already proven by Apple rather than create Android-specific serialization.
- Add one temporary stateless four-operation JNI bridge with fixed statuses, JVM-owned arrays, bounded call-local copies, panic containment, and no retained references.
- Keep Kotlin mapping strict and mechanical, retain the canonical schema record with its active service generation, and preserve all existing models, errors, and application behavior.
- Delete Kotlin's decompression, XML, fingerprint/signature, byte-reader/writer, scaling, string-framing, and CRC algorithms without fallback.
- Prove host and all-ABI packaging, source/symbol ownership, complete software behavior, and production Android/iOS/macOS ESP32 behavior.

**Non-Goals:**

- Changing the safe codec or canonical record format except where a verified platform-neutral safety defect requires a coordinated correction.
- Moving compatibility catalogue policy, chunk assembly, cache/storage policy, scheduling, revisions, conflicts, write/readback, Tune persistence, application publication, SQLite, loopback transport, BLE, location, background work, lifecycle, files, theme, permissions, WebViews, or frontend contracts.
- Changing Apple's selected bridge, C ABI, adapter, package graph, or runtime behavior.
- Generating bindings, combining temporary bridge crates, adding callbacks/handles/allocators, or committing native binaries.

## Decisions

### 1. Reuse safe-crate records as the only cross-boundary representation

The JNI bridge delegates schema decode, values decode, and values encode to the same `records` APIs selected by Apple. Kotlin receives schema/value record bytes and strictly projects them into the existing Android models; for writes it mechanically constructs one canonical value record that Rust fully parses and validates before emitting protocol bytes. No JNI crate or Kotlin source may independently interpret compressed XML, calculate a fingerprint/signature, interpret value payload fields, apply scales, frame strings, or calculate checksums.

The Android service stores the exact schema-record bytes alongside the projected schema for the active connection generation. Cached and freshly assembled schemas set both values atomically; reconnect, compatibility change, or schema failure clears both. Current/default decode and write encode require that retained record, preventing Kotlin reconstruction or stale layout selection.

Creating JVM model objects directly in Rust was rejected because it expands JNI surface and couples Rust to Kotlin constructors. JSON, direct buffers, generated bindings, and a second record format were rejected because the existing primitive-array records are bounded, portable, source-auditable, and intentionally disposable.

### 2. Add one temporary Android JNI crate with four operations

`refloat-configuration-codec-android-jni` exposes ABI version, schema decode, values decode, and values encode native methods. Operations accept only distinct JVM primitive arrays and an exact one-element result array for produced/required length. They validate references, pending exceptions, input and output maxima, array distinctness, and result initialization through `android-jni-support`; copy inputs only for the call; contain panics; map safe outcomes to fixed payload-free statuses; and retain no JVM reference, Rust pointer, schema, record, or mutable global state.

Output operations support a bounded two-pass call: an empty or insufficient output reports the exact required length without partial publication, then Kotlin allocates exactly that bounded size and repeats the stateless call. This avoids permanent multi-megabyte buffers while preserving caller ownership and deterministic behavior. Protocol work may repeat once, but configuration operations are infrequent and bounded.

Adding these methods to another JNI library was rejected because it couples independent protocol release/deletion surfaces. Returning Rust allocations or handles was rejected because it introduces ownership/lifecycle complexity that the eventual unified runtime, rather than temporary protocol bridges, should solve.

### 3. Kotlin becomes a strict record adapter and keeps the stable service API

`RefloatConfigurationCodec.kt` retains its service-facing model types and stable codec entry points but replaces protocol algorithms with a JNI loader and record reader/writer. The adapter validates linked ABI, exact record version/kind/extent/counts, reserved bytes, closed field/wire/value codes, canonical booleans/options, UTF-8, finite scalars, field uniqueness/completeness, schema/value identity, and complete cursor consumption. Fixed bridge statuses map to existing `RefloatConfigurationCodecError` values without exposing payload data.

Focused JVM tests cover every audited release fixture, every value type, exact schema/value projections, write/readback bytes, two-pass capacity behavior, ABI/status failures, malformed records, stale-generation handling, and absence of fallback. Once these pass, Kotlin's Inflater/SAX/MessageDigest/XML handler, signature/CRC, byte reader/writer, scale conversion, and protocol encode/decode branches are deleted.

### 4. Extend the typed Android build catalogue by one independent entry

The shared bridge catalogue gains Android package, library stem, JNI prefix, and exact four-symbol metadata on the existing Refloat configuration entry while retaining every Apple field. Gradle gains one typed bridge specification, raising the exact set from nine to ten, with host-JVM and four-ABI Debug/Release source builds, generated JNI merge inputs, test-library property, and dependency invalidation for the safe crate, JNI crate, shared support, lockfile/toolchain, and build scripts.

Audits require the exact symbols, inputs, outputs, package/task set, canonical record owner, Kotlin source deletion, Apple isolation, and absence of generated/committed artifacts. No new dependency is required: the JNI crate reuses `android-jni-support` and the existing safe codec.

### 5. Select only after software, packaging, and physical evidence

Selection requires safe-record regression, focused JNI Rust tests, host-JVM adapter/service tests, warnings-denied lint, exact source/symbol audits, clean host and four-ABI Android packaging, unchanged Apple packaging, strict OpenSpec validation, and `just verify`.

The production Android, iOS, and macOS apps then exercise the ESP32 fixture. Android must prove the new Rust configuration path across direct/CAN schema/current/default/write/readback plus BLE, routing, reconnect, files, theme, persistence, and mobile background behavior. Apple must prove isolation through its unchanged selected Rust path. Only bounded absence of a location sample may be recorded for later manual verification; no other failure is deferrable.

## Risks / Trade-offs

- **[Risk] Kotlin record parsing accepts a non-canonical or inconsistent record.** → Validate every header, extent, count, closed code, option flag, scalar, string, relationship, reserved byte, and trailing byte; exhaust malformed records in JVM tests.
- **[Risk] Two-pass calls perform expensive decompression twice.** → Configuration operations are bounded and infrequent; exact-size allocation avoids long-lived maximum buffers and keeps the bridge stateless/removable.
- **[Risk] A tenth Android bridge drifts across shell, Gradle, tests, and packaging.** → Extend the typed catalogue once and audit the exact task, package, property, symbol, input, output, and merge-consumer sets.
- **[Risk] Service state retains a schema and record from different generations.** → Store and clear them atomically and require the retained record for every decode/encode operation.
- **[Risk] Deleting Kotlin algorithms changes error classification.** → Preserve established public error categories, verify all fixture and adversarial mappings, and fail closed rather than add a fallback.
- **[Trade-off] Another JNI crate temporarily adds source.** → Keep it mechanical and four-operation only, delete substantially larger duplicate protocol code now, and explicitly remove both platform codec bridges with the unified runtime.

## Migration Plan

1. Add the Android JNI crate, safe-delegation tests, catalogue/Gradle metadata, packaging, and exact audits without selecting it from Kotlin.
2. Add the strict Kotlin record adapter and focused host-JVM parity/failure tests while retaining the old implementation only as test comparison evidence.
3. Retain the canonical schema record in the service, select the bridge in production, delete all superseded Kotlin protocol algorithms and comparison paths, and update ownership documentation.
4. Run focused tests, clean host/all-ABI Android and unchanged Apple packaging, strict validation, and the complete `just verify` gate.
5. Run the complete Android/iOS/macOS ESP32 checkpoint, rerun final software validation, commit implementation, then sync and archive.

Rollback before archive is a coherent source revert. No data or frontend contract migration occurs. After archive, rollback is a new OpenSpec ownership change with complete gates; a runtime Kotlin fallback is forbidden.

## Open Questions

None. Moving the stateful configuration service behind the unified Rust application runtime remains a later change and is the point at which both codec bridge crates and canonical-record adapters are deleted.
