## Context

The safe `vesc-custom-configuration` crate already owns a fixture- and adversarially-tested representation of VESC commands 92–95, but it remains only a candidate. The sibling iOS and macOS hosts share one Swift `VescCustomConfigurationCodec` that repeats the same request framing and response-envelope validation. Apple already has focused, caller-owned-buffer C bridges for Rust VESC framing and firmware identity, so the next bounded migration can reuse that integration pattern without moving higher-level Refloat configuration semantics or any OS service.

The bridge must remain buildable from locked source for macOS, iOS device, and iOS simulator; expose no Rust allocation or lifetime; preserve the existing Swift codec surface and errors; avoid payload-bearing diagnostics; and leave Android's Kotlin implementation and packaging graph untouched. The canonical physical control checkpoint currently proves safe Refloat controls but does not explicitly prove custom-configuration traffic, so it must be strengthened before Swift ownership is deleted.

## Goals / Non-Goals

**Goals:**

- Make the safe Rust crate the sole Apple production owner of command-92 schema requests, command-93/94 value requests, command-95 writes, and their response-envelope validation.
- Add one narrow, versioned Apple C ABI with fixed operations, statuses, output records, caller-owned buffers, panic containment, bounded regions, and an exact symbol allowlist.
- Preserve the shared Swift API, response enum, protocol error mapping, routing, scheduling, schema/value semantics, and all frontend-visible behavior.
- Build and link the bridge reproducibly for macOS, iOS device, and iOS simulator, with no generated bindings or committed archives.
- Prove the selected production path with focused Rust/C/Swift checks, clean packaging, `just verify`, and direct/CAN physical configuration operations on iOS and macOS while retaining Android regression coverage.
- Delete the superseded Swift command-envelope implementation and reject any fallback.

**Non-Goals:**

- Cutting Android over from its Kotlin custom-configuration codec.
- Moving schema assembly, decompression, XML, signatures, typed Refloat values, compatibility or safety policy, mutation coordination, routing, scheduling, or application behavior.
- Moving SQLite, the loopback server, BLE, background work, location, files, theme, lifecycle, permissions, persistence, or the frontend contract.
- Combining the bridge with the existing framing or firmware-identity ABI, transferring Rust allocation ownership, or introducing callbacks or opaque handles.

## Decisions

### Use a separate three-symbol Apple bridge

The change will add a dedicated `vesc-custom-configuration-apple-ffi` crate and hand-written `CVescCustomConfiguration` module. Its exact exported surface will be an ABI-version query, one encode operation, and one decode operation. Encode uses a fixed operation selector for schema, current, default, or write requests and copies the resulting payload into a bounded caller-owned output region. Decode writes one fixed-version metadata record describing the response kind, command, index, extents, and the validated opaque-byte range within the caller's original payload.

This keeps custom-configuration evolution independent of framing and identity, and keeps allocations and lifetimes on their originating side. Extending an existing bridge would reduce one archive name but would couple unrelated symbol allowlists and release surfaces. Returning a Rust allocation or object handle would create lifetime and cleanup obligations that are unnecessary for a stateless codec.

### Keep all VESC envelope decisions in safe Rust

The bridge will delegate every command byte, signed big-endian extent, chunk arithmetic, values minimum, acknowledgement shape, and structural-versus-unsupported decision to `vesc-custom-configuration`. The unsafe layer will be limited to validated pointer-to-slice conversion, disjoint-region checks, fixed-record writing, caller-buffer copying, status mapping, and panic containment. The safe codec remains `unsafe`-free and dependency-free.

Swift will validate the bridge ABI and fixed private record shape, convert representable primitives, copy the validated opaque subrange from its own input, and map fixed statuses to established `VescProtocolError` values. It will not retain command construction, signed-integer encoding, chunk/value/ack parsing, or an alternate recovery path. A richer generated binding was rejected because the repository intentionally uses small hand-written contracts, and copying decoded opaque bytes through a second bridge buffer was rejected because they already remain valid in caller-owned Swift storage for the duration of the call.

### Preserve the existing Swift service boundary

`VescCustomConfigurationCodec` will keep its public requests, `maximumChunkBytes`, response variants, and errors so `VescProtocolService` and `RefloatConfigurationService` do not change ownership. Only its implementation becomes a thin adapter over `CVescCustomConfiguration`. This confines the cutover below routing, request scheduling, schema assembly, typed values, and configuration mutation policy.

Replacing the codec with a Rust-owned semantic configuration service was rejected because it would mix the independently testable envelope cutover with XML, Refloat-version, safety, and application concerns.

### Reuse locked Apple source-build and packaging conventions

The bridge will have a checked build helper for the pinned Rust toolchain and the three Apple target families. SwiftPM will link the local bridge for host tests; the Xcode project will build and link the same source for simulator and device builds. Root recipes will audit the exact three-symbol allowlist, empty unexpected dependency surface, target archive architecture, source selection, clean packaging, and absence of generated or committed native binaries.

This follows the already-proven framing and identity integration rather than introducing a new build system. The bridge stays Apple-only; Android source sets, Gradle tasks, JNI symbols, and packaged libraries must not reference it.

### Add an explicit physical configuration checkpoint

The existing direct/CAN physical control scenario will first open the Refloat configuration, read current and default values, apply the fixture's audited disposable `kp` mutation, and verify readback before performing the established no-motor-movement controls. The same scenario will run on iOS and macOS through the new Rust bridge and on Android through the unchanged Kotlin codec.

Reusing the existing focused scenario keeps fixture setup and route coverage bounded while proving commands 92–95 across production BLE. A separate broad ride scenario would be slower and still would not guarantee configuration writes. A missing location sample during a bounded dwell remains deferred manual coverage, but no BLE, configuration, route, control, session, capability, monotonic-boundary, or cleanup failure is waived.

### Treat deletion as ownership evidence

Once focused, clean-packaging, canonical, and physical gates pass, the Swift request/response envelope algorithms will be deleted in the same change. Repository audits will fail if their characteristic command construction or chunk/value/ack parsing returns. There will be no runtime feature flag or Swift fallback; rollback is a source revert of the coherent implementation commit.

## Risks / Trade-offs

- **A third Apple Rust bridge adds packaging surface** → Reuse the existing locked helpers and Xcode/SwiftPM conventions, audit exactly three symbols and three target families, and keep the bridge dependency graph limited to the safe codec.
- **FFI pointer, overlap, or capacity mistakes could corrupt memory** → Validate null/length pairs, maxima, pairwise region disjointness, record capacity, and representability before constructing slices; contain panics; exercise failures through Rust and compiled C tests.
- **A malicious or inconsistent result record could cause an invalid Swift slice** → Version the record, validate its exact size/kind and checked offset-plus-length against the original payload, and map every inconsistency to the established structural error without parsing a fallback envelope.
- **Caller-owned output copying adds bounded temporary allocations** → Size request buffers from the known fixed request extent or bounded input length and retain the existing 400-byte schema chunk/product bounds; no buffer persists across calls.
- **Writing configuration during physical tests changes fixture state** → Write the just-read current opaque values and immediately verify readback on the disposable ESP32 fixture before safe controls; do not target an uncontrolled board.
- **Apple and Android temporarily use different implementations** → Preserve identical codec surfaces and canonical results, run the same physical scenario on all three hosts, and leave Android's later cutover as its own reversible OpenSpec change.

## Migration Plan

1. Add and test the versioned Apple bridge crate, C header/module, build helper, fixed record, statuses, symbol allowlist, bounds, and dependency audits without selecting it in Swift.
2. Wire locked SwiftPM and Xcode macOS/iOS device/simulator source builds and prove clean packaging.
3. Replace the shared Swift codec internals with the thin adapter while preserving its public surface and focused behavior tests.
4. Extend canonical physical control orchestration with explicit schema/current/default/write/readback operations and run the complete software gate.
5. Run direct and CAN physical checkpoints on the connected Android device, connected iOS device, and macOS host against the ESP32 fixture; defer only bounded location sample absence.
6. Delete the superseded Swift envelope implementation, keep no fallback, rerun audits and `just verify`, then commit the verified implementation.
7. Sync the delta specifications and archive the completed change together. Rollback before archive is a revert of the implementation commit; after archive it remains a normal source/spec revert rather than a runtime dual path.

## Open Questions

None. Android production selection and higher-level configuration ownership are intentionally deferred to later changes.
