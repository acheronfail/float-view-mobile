## Context

Both hosts decode the same Refloat custom-app-data command-10 response into the same normalized compact telemetry record. The wire family is a particularly small extraction boundary: one exact five-byte firmware-fault form or one of four exact fixed layouts (35, 42, 55, or 59 bytes), with no negotiated schema, heap-backed extension map, retained decoder state, routing decision, or application lifetime.

Pinned `vendor/refloat/src/main.c` defines the field order, scales, state bits, detail-level tails, and `222` field-weakening sentinel. `backends/fixtures/refloat/detailed-source-provenance.json` contains immutable source-derived vectors for every detail level and an active fault. Existing native and conformance tests already establish normalized field expectations. The extraction must remain unselected so its correctness and isolation can be reviewed before either host ABI changes.

## Goals / Non-Goals

**Goals:**

- Add one safe, dependency-free Rust component for complete command-10 compact telemetry decoding.
- Preserve the audited envelope, exact lengths, numeric scales, state and flag mappings, nullable detail tails, speed conversion, active-fault classification, and deterministic closed failures.
- Model `222` as absent field-weakening current and reject non-finite auto-float distance values, matching pinned source intent and existing platform-neutral telemetry requirements.
- Reuse committed fixtures without adding a JSON dependency and cover all consumed byte, enum, numeric, and privacy boundaries.
- Keep workspace shape, dependencies, bridge symbols, native packaging, and selected Swift/Kotlin production paths exact.
- Prove repository and real-BLE regression on Android, iOS, and macOS before archiving.

**Non-Goals:**

- Adding Apple FFI, Android JNI, generated bindings, host packaging, or production selection.
- Removing or changing either native decoder, telemetry record, service router, scheduler, compact cache, or persistence flow.
- Decoding command 1, commands 31/32, negotiated fields, dynamic alert tails, configuration, or history.
- Moving compatibility, direct/CAN routing, VESC framing, polling, normalization merging, session behavior, SQLite, loopback transport, Bluetooth, background work, location, files, lifecycle, theme, permissions, or UI behavior.

## Decisions

### Add one focused safe crate

Add workspace package `float-view-refloat-legacy-telemetry` at `backends/core/refloat-legacy-telemetry`. It will forbid unsafe code and have no normal or third-party development dependencies. It will not be added to the bridge build catalog because no native artifact consumes it in this change.

Bundling legacy and dynamic telemetry was rejected: dynamic commands 31/32 introduce retained negotiated layout state, UTF-8 identifiers, bounded collections, extension values, and a materially wider future bridge. Extracting configuration or application behavior first was rejected because both have more dependencies and mutation risk. Extracting INFO alone was rejected as too small to retire meaningful duplication.

### Decode a complete unframed Refloat envelope into fixed Rust values

The safe API will accept the complete unframed `[36, 101, 10, ...]` payload and return a closed `LegacyTelemetry` enum: either one redacted fixed-size sample or an active firmware-fault record containing the raw code and stable classification. The sample will expose protocol-semantic enums and fixed `f64`/`Option<f64>` values for every field carried through detail level 4. It will not own the application telemetry kind, ride-session sequence, timestamps, compact-cache merge, JSON names, or persistence.

Accepting only bytes after the Refloat command was rejected because validating the VESC custom-app-data command and Refloat package identifier makes the future bridge boundary self-contained. Returning a host-specific record was rejected because native application models and error types remain outside the safe protocol component.

### Preserve exact layouts and source numeric semantics

Normal responses require detail level 1 through 4 and exact total lengths 35, 42, 55, or 59. Fault responses require discriminator `69`, exact length 5, and a nonzero raw fault code. Signed fields use big-endian two's-complement decoding and their source divisors; the compact speed in metres/second is converted exactly once to kilometres/hour; the distance word uses the VESC/IEEE normalized float representation and must be finite.

The field-weakening byte is `None` only for source sentinel `222`; every other byte retains the established magnitude conversion. Optional values are absent below their detail level. Unknown bounded alert and firmware-fault codes map to stable `Unknown` variants. Existing legacy state, package-mode, footpad-side, stop-condition, and setpoint-adjustment mappings remain exact.

### Use closed, redacted errors and allocation-free output

The public error surface will distinguish truncated input, unsupported envelope/command, and structurally invalid content without including payload bytes or decoded telemetry. Output `Debug` will be redacted. The result has no borrowed strings, collections, callbacks, allocator transfer, or host lifetime and will have an explicit size audit.

One generic panic-based parser was rejected because later host bridges require deterministic failure mapping. Preserving non-finite distance or treating sentinel `222` as 74 A was rejected because pinned upstream source cannot emit non-finite values and explicitly reserves the sentinel for unavailable field weakening.

### Keep candidate and production evidence distinct

Focused Rust tests will read the committed fixture JSON through a tiny test-only string extractor. They will cover all four layouts, the active-fault form, exact lengths, all state/mode/footpad/adjustment/alert/fault mappings, signed and scaled boundaries, auto-float finite failures, sentinel handling, deterministic output, and redacted diagnostics. Repository audits will prove there is no bridge symbol, package dependency, generated artifact, or production call site.

`just verify` builds and tests the candidate while existing native and canonical tests continue to prove production behavior. Physical identity, direct/CAN control, and mobile background/session scenarios exercise only the selected native decoders; they prove regression freedom, not Rust candidate execution. Separate Apple and Android cutovers must later add and select focused bridges, prove live command-10 decoding, and delete their superseded native source independently.

## Risks / Trade-offs

- **The candidate can drift while native code remains selected.** → Reuse immutable source-derived fixtures, audit production isolation, and follow with separate per-host cutovers before expanding scope.
- **The candidate intentionally fixes sentinel and non-finite behavior before production selection.** → Keep the correction candidate-only, test it against pinned source intent, and make each later host cutover explicitly verify normalized nullable behavior.
- **The result contains many optional scalar fields.** → Keep it fixed-size and allocation-free, audit its size, and avoid prematurely introducing a general telemetry/runtime model.
- **Physical runs cannot execute an unselected crate.** → Label them regression evidence only; later cutovers must provide selected-path Rust evidence before native deletion.

## Migration Plan

1. Add the isolated safe crate and exact workspace, ownership documentation, dependency, and production-isolation audits.
2. Implement complete fixed-layout decoding and source-derived/adversarial component tests.
3. Run focused Rust/native/conformance/audit checks and the complete `just verify` gate.
4. Run focused production-path physical regression on Android, iOS, and macOS with the ESP32 fixture.
5. Leave both native decoders selected and follow with one Apple and one Android cutover change.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no native package, production call path, persistent data, or user-visible behavior to migrate or reverse.

## Open Questions

None. Dynamic telemetry ownership, shared telemetry models, bridge record layout, host cutover order, and application normalization remain deliberate later decisions.
