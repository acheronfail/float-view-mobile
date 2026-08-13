## Context

Both production hosts currently negotiate Refloat command 32 into two retained UTF-8 field lists and then use those lists to decode command 31. The Swift implementation is 156 lines and the Kotlin implementation is 178 lines; each repeats envelope and length handling, bounded field parsing, Refloat's non-IEEE-infinity float16 conversion, semantic state and alert mapping, charging and active-alert tails, known-field projection, and bounded extensions.

Pinned Refloat tags 1.1.0 through 1.2.3 define one 16-field always-present list and one 10-field running list. The committed source-provenance fixture contains the exact shared command-32 layout plus 1.1 and 1.2 running, charging, and active-firmware-fault command-31 responses. Canonical requirements additionally permit bounded future identifiers and require at most 64 total fields, 64 UTF-8 bytes per name, 4,096 aggregate name bytes, and 32 unknown numeric extensions.

Unlike legacy command 10, command 31 is not independently decodable: its extent and value order depend on the latest accepted command-32 layout. This change therefore extracts layout and realtime decoding together, but keeps the component unselected so its bounded state model can be proven before either host ABI changes.

## Goals / Non-Goals

**Goals:**

- Add one safe, dependency-free Rust component for complete command-32 layout and command-31 realtime decoding.
- Preserve full unframed envelope validation, negotiated order, exact mask-dependent extents, source ticks, state/mode/footpad/stop/adjustment/alert semantics, charging and 1.2 alert tails, fatal latch, firmware-fault classification, typed known fields, and bounded extensions.
- Decode the source float16 format across its complete finite `+-131008` range, including exponent `0x1f`, while rejecting no source-representable finite value.
- Reuse committed fixtures without a JSON dependency and cover all consumed byte, layout, identifier, mask, extent, enum, numeric, tail, extension, and privacy boundaries.
- Keep workspace shape, dependencies, bridge symbols, native packaging, and selected Swift/Kotlin production paths exact.
- Prove repository and real-BLE regression on Android, iOS, and macOS before archiving.

**Non-Goals:**

- Adding Apple FFI, Android JNI, generated bindings, host packaging, or production selection.
- Removing or changing either native decoder, compatibility catalogue, telemetry service, scheduler, compact cache, application model, or persistence flow.
- Decoding legacy command 1/10, command 33, history, data recording, configuration, or controls.
- Moving compatibility, direct/CAN routing, VESC framing, polling, cache merging, observation time, session behavior, SQLite, loopback transport, Bluetooth, background work, location, files, lifecycle, theme, permissions, or UI behavior.

## Decisions

### Extract command 32 and command 31 as one focused safe crate

Add workspace package `float-view-refloat-dynamic-telemetry` at `backends/core/refloat-dynamic-telemetry`. It will forbid unsafe code, use only the Rust standard library, and remain absent from the bridge build catalogue because no native artifact consumes it in this change.

Extracting command 32 alone was rejected because a layout without a value decoder cannot retire or validate the duplicated protocol behavior. Extracting command 31 alone was rejected because accepting arbitrary caller-provided names without owning their bounds and uniqueness would weaken the decoder boundary. Adding host bridges in the same change was rejected because variable layout state and variable-length extensions deserve isolated candidate evidence before ABI design.

### Represent negotiated layout as bounded owned state

The safe API will accept the complete unframed `[36, 101, 32, ...]` response and return a `DynamicLayout` owning ordered always-present and running-only UTF-8 identifiers. Construction will enforce exact envelope consumption, at most 64 combined fields, at most 64 encoded bytes per nonempty identifier, at most 4,096 aggregate bytes, and global uniqueness. No partially decoded layout is returned.

Owning strings was chosen because the negotiated layout survives beyond the response buffer and later command-31 calls must not borrow host memory. Fixed 64-by-64 byte matrices were rejected because they would make every normal layout unnecessarily large and complicate future bridge records. The bounded standard-library allocation is internal to the safe component and no allocator crosses a host boundary in this change.

### Decode one complete response against one immutable layout

The realtime API will accept `&DynamicLayout` plus the complete unframed `[36, 101, 31, ...]` payload. It will validate the closed mask, derive the exact extent from the always-present count, optional running list, charging tail, and 1.2 nine-byte alert tail, then decode values in negotiated order.

The result will expose source ticks, semantic header values, state flags, optional charging values, both active-alert words, optional fatal-latch state, raw and classified firmware fault, typed options for every known 1.1/1.2 field, and at most 32 ordered unknown finite extension values. Compatibility baseline checks, layout replacement timing, reconnect generation, polling cadence, compact-cache merging, observation time, and normalized application publication remain outside the candidate.

Returning only an untyped name/value map was rejected because it would leave the duplicated known-field projection in Swift and Kotlin. Returning a host application record was rejected because ride state, session sequence, timestamps, persistence, and frontend names remain application concerns. Reusing the legacy crate's narrower enums was rejected because dynamic releases add adjustment and alert cases and would create a misleading dependency from dynamic protocol behavior to a legacy-specific package.

### Implement Refloat's complete finite float16 format

The decoder will implement the pinned source `to_float16` inverse: sign, five-bit exponent, and ten-bit fraction are finite across every bit pattern, including exponent `31`. Positive and negative zero, subnormals, normals, saturation endpoints, and deterministic rounding fixtures will be covered. The candidate will not apply an IEEE-754 NaN/infinity rule because upstream explicitly repurposes that exponent for values through `+-131008`.

This differs from the currently selected native helper, which rejects exponent `31`; keeping the candidate unselected prevents an incidental production behavior change. Later host-cutover changes must explicitly prove the corrected boundary before deleting each native implementation.

### Use closed redacted failures and bounded outputs

The public error surface will distinguish truncated input, unsupported envelope, invalid layout, and invalid realtime content without payload bytes, field values, Bluetooth identity, or addresses. Layout and sample `Debug` output will expose only bounded structural counts. All collection growth will be checked before allocation or insertion, duplicate known or extension values will be impossible after layout validation, and errors will return no partial sample.

### Keep candidate and production evidence distinct

Focused Rust tests will read the committed fixture JSON through a tiny test-only extractor and derive adversarial cases from those bytes. Repository audits will prove there is no bridge symbol, native package dependency, generated artifact, bridge catalogue entry, or production call site.

`just verify` will build and test the candidate while existing native and canonical tests continue proving production behavior. Physical identity, direct/CAN control, dynamic telemetry, ride, unexpected-disconnect, and mobile background/session scenarios exercise only the selected native decoders; they prove regression freedom, not Rust candidate execution. Separate Apple and Android cutovers must later add and select focused bridges, prove live command-31/32 execution, and delete their native parser independently.

## Risks / Trade-offs

- **The candidate allocates bounded layout and extension strings.** → Keep exact field/name/aggregate/extension limits, use no third-party dependency, and transfer no allocation across a host boundary.
- **The candidate can drift while native code remains selected.** → Reuse immutable source-derived fixtures, audit production isolation, and follow with separate per-host cutovers before expanding scope.
- **Correct source float16 endpoints differ from selected native behavior.** → Keep the correction candidate-only and require explicit boundary evidence in each later host cutover.
- **Typed known fields repeat some semantic enums already present in the legacy crate.** → Keep the new component independent for now; consolidate protocol/application models only when a later unified Rust runtime owns both paths, rather than coupling a dynamic decoder to a legacy-named crate.
- **Physical runs cannot execute an unselected crate.** → Label them regression evidence only; later cutovers must provide selected-path Rust evidence before native deletion.

## Migration Plan

1. Add the isolated safe crate and exact workspace, ownership documentation, dependency, and production-isolation audits.
2. Implement bounded layout state, complete realtime decoding, source-derived tests, and adversarial tests.
3. Run focused Rust/native/conformance/audit checks and the complete `just verify` gate.
4. Run focused production-path physical regression on Android, iOS, and macOS with the ESP32 fixture.
5. Leave both native dynamic decoders selected and follow with one Apple and one Android cutover change.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no native package, production call path, persistent data, or user-visible behavior to migrate or reverse.

## Open Questions

None. Variable host record design, host cutover order, shared telemetry models, compatibility and polling ownership, and the unified application runtime remain deliberate later decisions.
