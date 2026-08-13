## Context

`refloat-configuration-codec` is a safe, bounded Rust candidate that already reproduces the upstream Refloat schema and value wire semantics. The shared Apple backend still owns the same XML decompression/parsing, fingerprint and wire-signature construction, typed value decode, and typed value encode in Swift. Its stateful configuration service is deliberately separate: it owns VESC chunk scheduling, cache selection, catalogue compatibility, revisions, conflict detection, write/readback verification, Tune persistence, and application publication.

The next production boundary must therefore select Rust only for the stateless codec. It must work in the one Apple backend shared by sibling iOS and macOS, preserve Android's independently selected Kotlin implementation, and remain removable migration scaffolding rather than a permanent generated API layer.

## Goals / Non-Goals

**Goals:**

- Make the safe Rust crate Apple's sole owner of Refloat configuration schema and value wire semantics.
- Give Swift a bounded, versioned canonical representation rather than reproducing protocol algorithms.
- Keep the temporary Apple bridge narrow, stateless, deterministic, source-built, and mechanically auditable.
- Preserve every configuration-service, application, persistence, frontend, and OS capability behavior.
- Prove the cutover through focused software evidence and the complete Android/iOS/macOS ESP32 checkpoint.
- Leave an explicit deletion path for the bridge and record adapter when the unified Rust runtime owns the application.

**Non-Goals:**

- Cutting Android's configuration codec over to Rust.
- Moving configuration scheduling, cache/catalogue policy, revisions, conflicts, readback, Tune persistence, or application publication.
- Moving the loopback web server, application service, SQLite, BLE, location, background work, file presentation, theme, lifecycle, permissions, or WebViews.
- Changing the frontend contract, stored Tune format, configuration semantics, or user-visible behavior.
- Introducing generated language bindings or a general-purpose cross-platform bridge framework.

## Decisions

### 1. The safe crate owns two canonical binary records

The safe crate will define, serialize, parse, and test versioned schema and value records. Each record has a closed kind, explicit byte length and entry counts, big-endian scalar encoding, canonical UTF-8 strings, zero reserved bytes, finite numeric values, and crate-owned maximum sizes. Parsers reject trailing bytes, non-canonical flags, duplicate or incomplete values, unknown kinds, invalid schema relationships, and any mismatch between a value record and its schema.

The schema record contains all normalized field, default, numeric-option, choice, description, group, fingerprint, and wire-signature data needed by the existing Swift models. The value record contains one closed typed value for every schema field. Rust creates both decode outputs. Swift mechanically writes a value record when calling encode; Rust parses and validates that untrusted record before producing protocol bytes. This one small record writer is transport adaptation, not an alternative value codec.

The Apple configuration service retains the exact canonical schema-record bytes beside its normalized Swift schema for the active connection generation. Current/default decode and write encode must use that retained record, so stale or reconstructed schema state cannot select wire layout.

### 2. A temporary stateless Apple bridge has exactly four symbols

The new bridge exposes only:

1. ABI version,
2. compressed-schema decode to canonical schema record,
3. value-payload decode from schema record to canonical value record, and
4. value-payload encode from schema and value records.

Every operation uses caller-owned buffers and fixed status values. The bridge checks pointer/length pairs, maximum capacities, scalar outputs, pairwise-disjoint regions, and output sizes before use; catches panics; writes deterministic lengths; and retains no pointer, record, schema, value, or mutable global state. Protocol-specific parsing and serialization stay in the safe crate, while the bridge contains only ABI validation and outcome-to-status mapping.

The bridge is intentionally Apple-only. Adding it must not add an Android JNI artifact, Gradle task, or package.

### 3. Swift becomes a strict canonical-record adapter

The existing Apple Refloat configuration model remains the service-facing type to avoid broad application churn. Its implementation will import the hand-written C module, validate ABI and record headers, and mechanically project canonical fields, groups, and values into the existing models. It will also mechanically emit the canonical value record passed to Rust for writes.

After focused selection evidence passes, Apple deletes the Swift XML parser, Qt decompressor, SHA/fingerprint construction, wire-signature reconstruction, CRC/value byte reader and writer, and all fallback branches. Bridge or record failure maps to the existing closed configuration errors and cannot silently retry Swift logic.

### 4. The stateful Apple configuration service remains the owner around the codec

The configuration service continues to own chunk assembly, maximum transfer limits, cache storage and selection, catalogue compatibility, connection generations, revisions, conflicts, flywheel rules, scheduler interactions, write/readback verification, Tune import/export and persistence, and publication to application state. It stores the canonical record with the schema only so later codec calls use the selected Rust schema. No Rust callback, scheduler, database, host singleton, or long-lived handle is introduced.

### 5. Apple packaging extends the existing locked source-build catalogue

The shared bridge catalogue gains one Apple-only configuration-codec entry. SwiftPM and Xcode build the locked crate for macOS, iOS device, and iOS simulator, link the correct archive and C module, and track source and lockfile changes. Clean builds must work without a prebuilt or committed native archive. The typed Android bridge set remains unchanged and audits reject any accidental Android artifact for this Apple-only bridge.

### 6. Selection requires source, software, packaging, and physical evidence

The cutover is selected only after:

- safe-crate record round trips and adversarial record tests,
- bridge Rust and compiled-C harness tests,
- Swift adapter parity, malformed-record, ABI, status, schema-generation, and no-fallback tests,
- clean SwiftPM/Xcode packaging and source-ownership audits,
- the complete repository software gate, and
- Android, iOS, and macOS production apps exercise the ESP32 fixture, including direct and CAN configuration schema/current/default/write/readback behavior and mobile background behavior.

Android must continue selecting Kotlin; both Apple hosts must show the production Rust path. A bounded missing location sample is recorded and deferred exactly as allowed, but no BLE, configuration, background, packaging, or codec failure is deferrable.

## Risks / Trade-offs

- **Canonical-record drift between Rust and Swift.** Version/kind/length/count fields, zero reserved bytes, fixed golden records, a compiled-C harness, and malformed-record tests make drift fail closed.
- **The schema record adds temporary memory and adapter code.** It is bounded, retained once per connection generation, and replaces substantially larger protocol implementations. It can be deleted with the bridge at unified-runtime cutover.
- **Large string fields require larger buffers than telemetry bridges.** Crate-owned hard maxima and caller-owned capacity checks preserve bounded behavior; configuration operations are infrequent and never run per telemetry sample.
- **Retaining the wrong schema record could corrupt writes.** The service stores schema and record as one generation-scoped selection and clears them together on reconnect or compatibility change.
- **An Apple-only catalogue entry could leak into Android tooling.** Typed catalogue tests and Android package/task audits require the Android bridge set to remain exactly unchanged.
- **Two production hosts change together.** They share one Apple backend and one codec implementation, but retain separate packaging and physical checkpoints so host-specific regressions remain visible.

## Migration Plan

1. Add and test canonical record types and conversion entry points in the safe crate without selecting them in a host.
2. Add the four-symbol Apple bridge, C header/module, build catalogue entry, and ABI/C-harness evidence.
3. Link the bridge into SwiftPM and Xcode for macOS, iOS device, and iOS simulator; prove clean locked source builds.
4. Add the strict Swift record adapter and focused parity/failure tests while the old implementation is still available only for test comparison.
5. Select Rust in production, retain the schema record with the service generation, delete the Swift protocol algorithms and comparison-only path, and pass ownership audits.
6. Run complete software and all-host ESP32 checkpoints, then sync and archive the change.

Rollback before archive is a normal revert of this isolated selection change. No frontend or persistence migration occurs, so rollback does not require data conversion. After archive, any rollback must be a new OpenSpec change that restores one explicit owner and repeats complete gates; a runtime fallback is forbidden.

## Open Questions

None. Android's later configuration-codec cutover and the eventual removal of every host bridge remain separate changes.
