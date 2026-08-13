## Context

`refloat-dynamic-telemetry` is a dependency-free, unsafe-free Rust candidate that already decodes bounded Refloat command-32 layouts and every command-31 response form. Apple production still validates the same layouts, extents, source float16 values, semantic codes, alerts, faults, and extensions in `RefloatDynamic.swift`; iOS and macOS share that source. The service retains negotiated field names for the current connection and clears them on reset.

Existing Apple protocol components are selected through independent caller-owned-buffer C bridges built from locked source. Unlike fixed command-10 telemetry, dynamic command-31 decoding depends on a variable command-32 layout and returns bounded variable extensions. The cutover must therefore transfer both records without opaque Rust state, preserve connection-scoped layout lifetime, keep Android on Kotlin, and leave compatibility, routing, polling, cache merging, application behavior, persistence, transport, and OS capabilities native.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Apple production owner of command-31/32 wire structure and semantics.
- Put canonical bounded layout and sample records in the safe crate so later Android integration does not duplicate serialization.
- Add one temporary three-symbol Apple bridge with fixed statuses, caller-owned buffers, checked regions, and panic containment.
- Preserve normalized telemetry/layout models and established failures while deleting superseded Swift wire algorithms and `RefloatTelemetryMappings.swift`.
- Prove exact records, adversarial boundaries, Apple packaging/source selection, Android isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Selecting Rust dynamic telemetry on Android.
- Moving compatibility, request construction, direct/CAN routing, framing, scheduling, polling, connection generations, compact-cache merging, application behavior, SQLite, loopback transport, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract.
- Introducing the final Rust application runtime, callbacks, opaque handles, generated bindings, `cbindgen`, UniFFI, committed native binaries, or a new dependency.
- Generalizing or merging temporary per-protocol bridge crates.

## Decisions

### Put both canonical records in the safe decoder crate

Extend `refloat-dynamic-telemetry` with closed record outcomes and sole serializers/parsers for two version-1 byte records. The crate remains dependency-free and forbids unsafe code.

The layout record has an 8-byte header containing version, record kind, always-present count, running-only count, aggregate UTF-8 byte count, and reserved zero bytes. It appends each identifier as one nonzero byte length followed by its UTF-8 bytes, preserving the two lists and negotiated order. Its maximum is bounded by the existing 64-field, 64-byte-name, and 4,096-byte aggregate limits.

The sample record has a 32-byte header containing version, record kind, closed semantic codes, behavior/option flags, fault classification/raw code, extension count, reserved bytes, source ticks, a fixed-value presence mask, and canonical active-alert words. It appends 28 finite big-endian `f64` slots in stable order: the 26 known negotiated values followed by charging current and voltage. Absent slots contain positive-zero bits. Ordered extensions then use a nonzero one-byte name length, UTF-8 name, and finite big-endian value. At most 32 extensions and all existing name bounds apply.

The safe crate validates canonical layout records before using them for realtime decoding. This makes the bridge stateless and gives the later Android bridge the same record owner. Returning Rust structs/allocations, JSON, or encoding records independently in each bridge was rejected because those choices expose lifetimes, increase malformed surface, or duplicate maintenance.

### Use a temporary stateless three-symbol Apple bridge

Add `refloat-dynamic-telemetry-apple-ffi` with exactly an ABI-version query, layout decode, and realtime decode. Layout decode accepts one raw payload and publishes one canonical layout record. Realtime decode accepts one canonical layout record plus one raw payload and publishes one canonical sample record. Every region is bounded, valid, aligned where required, and pairwise disjoint; outputs are initialized deterministically; panics are contained; no pointer or allocation crosses the call.

Fixed statuses distinguish success, invalid arguments, unsupported envelope, truncated payload, invalid layout, invalid telemetry, insufficient output, and contained internal failure. `apple-ffi-support` owns only mechanical region handling. The safe telemetry crate owns protocol decisions, canonical records, and every serializer/parser.

Combining this with an existing bridge was rejected because it couples unrelated release surfaces. An opaque layout handle was rejected because reconnect cleanup and eventual bridge deletion would gain a Rust lifetime/state protocol.

### Retain canonical layout bytes in the existing Apple connection generation

The Swift adapter returns the existing public field lists plus private canonical layout bytes. `VescProtocolService` retains that value only for the current connection, clears it wherever it currently clears negotiated field arrays, and passes the exact bytes to realtime decode. A realtime response without a current layout fails through the established protocol error. Rust reparses the bounded canonical record on each command-31 call; this small cost avoids cross-call Rust state.

Swift strictly validates bridge ABI, record version/kind/extent, counts, UTF-8, reserved bytes, code/flag masks, option consistency, finite numbers, canonical absent values, alert/fault relations, extension uniqueness/order, and complete cursor consumption before constructing the existing models. These are representation checks, not command-31/32 parsing. After selection, raw envelope/name/float16/extent/tail algorithms and the semantic mapping file are deleted with no fallback.

### Add one Apple-only catalog entry while preserving Android’s closed set

Add an Apple-capable `refloat-dynamic-telemetry` entry to the shared bridge catalog. The Android builder rejects the entry because it has no JNI metadata, and Android’s typed Gradle catalog, task graph, seven artifacts, packages, and sources remain unchanged. SwiftPM and Xcode add the hand-written C module, source inputs, archive search path, and linkage for macOS, iOS device, and iOS simulator.

The temporary catalog asymmetry is intentional until the separate Android cutover. Adding an unused JNI crate or Android task now would falsely imply production integration.

### Delete Swift wire ownership only after focused selection evidence

Focused safe-Rust tests cover exact layout/sample records, all semantic codes/flags/options, every float16 bit pattern, active alerts/faults, ordered extensions, record corruption, and redacted failures. Compiled C tests cover all three symbols, statuses, region/capacity behavior, and exact bytes. Swift tests cover mechanical mapping, connection reset/replacement, malformed records, and production call selection.

After clean packaging and `just verify`, run identical focused protocol/control/BMS/ride/disconnect scenarios on Android, iOS, and macOS, plus mobile background/session behavior. Apple runs exercise production Rust command-31/32 decoding; Android proves isolation. Only absence of a location sample within its bounded dwell is deferred.

## Risks / Trade-offs

- **[Risk] Variable records can drift from negotiated order or normalized models.** → One safe serializer/parser, exact constants, exhaustive corruption tests, strict Swift cursor validation, and source-ownership audits make drift fail closed.
- **[Risk] Retained layout bytes can survive a reconnect.** → Store them in the existing protocol-service connection generation and audit every current reset/replacement path.
- **[Risk] Reconstructing the layout per sample adds work.** → The input is bounded to 64 names/4,096 bytes and avoids opaque cross-call state; physical cadence tests detect regressions.
- **[Risk] Apple-only catalog metadata can leak into Android builds.** → Keep Android’s typed catalog at seven entries, reject missing JNI metadata, and audit artifacts/tasks/packages unchanged.
- **[Trade-off] The adapter still contains record-to-model mapping.** → It contains no wire protocol algorithm and is explicitly temporary; the final Rust application runtime removes it with the bridge.

## Migration Plan

1. Add canonical safe records, corruption tests, the Apple bridge/C harness, catalog metadata, and audits without selecting Swift.
2. Wire SwiftPM and Xcode, implement the strict record adapter and connection-scoped layout retention, and add focused tests.
3. Select Rust, delete Swift command-31/32 wire parsing and semantic mappings, then prove clean Apple packaging, Android isolation, strict OpenSpec validation, and `just verify`.
4. Run the focused ESP32 matrix on connected Android, iOS, and macOS, deferring only bounded location acquisition.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no data migration, persisted-format change, dual runtime path, or remote deployment.

## Open Questions

None. Android production selection and the unified Rust application/runtime boundary remain separate changes.
