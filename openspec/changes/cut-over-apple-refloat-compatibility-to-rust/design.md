## Context

`refloat-compatibility` is a dependency-free, unsafe-free Rust candidate that already constructs the exact INFO request, decodes bounded legacy/version-2 identities, selects the closed nine-release telemetry/control catalogue, and admits dynamic layouts against the exact required identifiers. Apple production still performs those decisions in `RefloatInfo.swift` and `RefloatCompatibility.swift`; iOS and macOS share that source. Android independently retains the equivalent Kotlin implementation.

Apple already selects Rust for dynamic command-31/32 decoding and retains a canonical bounded layout record for the current connection generation. Existing Apple protocol components use independent caller-owned-buffer C bridges built from locked source. This cutover can therefore pass the Rust-owned layout record directly into compatibility admission without serializing field arrays again, while keeping discovery effects and application coordination native.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Apple production owner of INFO request/response identity, exact release/profile selection, and required dynamic-layout admission.
- Add one canonical bounded compatibility record to the safe crate for later Android reuse.
- Add one temporary stateless Apple bridge with fixed statuses, caller-owned buffers, checked regions, and panic containment.
- Reuse the existing canonical dynamic-layout record instead of defining a second name-list transport.
- Preserve existing wire-version strings, normalized profile models, errors, controller-discovery effects, state/session behavior, and frontend output while deleting superseded Swift algorithms.
- Prove exact records, adversarial boundaries, Apple packaging/source selection, Android isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Selecting Rust compatibility on Android.
- Moving controller discovery, route selection, request scheduling, connection generations, telemetry decoding, controls, application behavior, SQLite, loopback transport, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract.
- Introducing the unified Rust application runtime, opaque handles, callbacks, generated bindings, `cbindgen`, UniFFI, committed native binaries, or a new third-party dependency.
- Consolidating temporary per-protocol bridge crates.

## Decisions

### Publish one canonical fixed compatibility record

Extend `refloat-compatibility` with a version-1 fixed record and closed record outcome. The record contains structural wire major/minor/optional patch, the closed telemetry profile/family, lighting profile, fatal-clear availability, and reserved zero bytes. The safe crate is the only serializer/parser; malformed version, kind, codes, flags, reserved bytes, and impossible profile combinations fail closed.

The INFO record operation performs envelope decoding and exact catalogue selection atomically. Unknown wire identities therefore never cross the host boundary as partially accepted versions. Keeping decoding and lookup as separate host calls was rejected because it permits drift and retains policy sequencing in Swift. JSON and C structs were rejected because they add allocation/lifetime or layout concerns for a small fixed result.

### Reuse the Rust dynamic-layout record for admission

Add a normal dependency from `refloat-compatibility` to `refloat-dynamic-telemetry` and expose admission over that crate’s canonical layout record. The compatibility crate validates/parses the record through the existing safe owner, then checks required names in their correct groups. This replaces the candidate’s empty dependency graph with one downward safe-Rust protocol dependency while preserving dependency-free third-party production code and unsafe-free behavior.

Passing arrays of C strings, pointer tables, NUL-delimited names, or a second compatibility-specific layout record was rejected because each creates another bridge format and repeats bounds/UTF-8/group serialization already owned by Rust dynamic telemetry. Moving discovery state into Rust was rejected because layout admission is pure policy and discovery effects remain a later application-behavior slice.

### Use a temporary stateless four-symbol Apple bridge

Add `refloat-compatibility-apple-ffi` with exactly an ABI-version query, INFO-request publication, INFO-decode/compatibility-record publication, and canonical-layout admission. Every input/output region is bounded and pairwise disjoint where applicable; scalar outputs are initialized deterministically; panics are contained; no Rust allocation, pointer, object, callback, or lifetime crosses C.

Fixed statuses distinguish success, invalid arguments, unsupported envelope/protocol/release, truncated or excessive input, invalid layout, insufficient output, and contained internal failure. `apple-ffi-support` owns only mechanical region handling. The safe compatibility/dynamic crates own protocol decisions and record formats.

### Carry the accepted decision through existing Apple coordination

The Swift adapter mechanically validates the canonical compatibility record and maps it into the existing wire-version, telemetry-family, and control-profile models. `VescProtocolService` obtains the INFO request and accepted decision from the bridge. The existing protocol message and controller-discovery path carry that already-accepted decision; discovery still decides readiness/layout request effects and application code still owns state, errors, sessions, and controls.

The existing canonical dynamic layout record is retained only for its connection generation. After command-32 decode, Apple asks Rust compatibility to admit that record and carries the Boolean decision with the existing field lists. Swift no longer owns required identifier constants, set construction, catalogue lookup, or INFO byte offsets. Application control setup and loopback support reporting consume the accepted decision already stored by discovery rather than repeating string lookup.

### Add one Apple-only bridge catalogue entry and delete Swift ownership after selection

Add an Apple-capable compatibility row to the shared source-build catalogue. Android’s typed catalogue must reject the missing JNI metadata and preserve its exact eight current bridge artifacts/tasks/packages. SwiftPM and Xcode add the hand-written C module, archive search path, source inputs, and linkage for macOS, iOS device, and iOS simulator.

Focused safe-Rust tests cover exact requests/records/catalogue/layout admission and record corruption. Compiled C tests cover all four symbols, statuses, region/capacity behavior, deterministic outputs, and exact bytes. Swift tests cover mechanical mapping, production request/decode selection, discovery propagation, layout admission, malformed records/statuses, and absence of fallback algorithms. Once those pass, delete the superseded Swift INFO decoder and catalogue/required-layout implementations.

## Risks / Trade-offs

- **[Risk] The accepted compatibility decision could separate from its reported version.** → Carry one immutable mapped decision from the atomic Rust INFO operation through discovery and use that stored decision for control/state publication.
- **[Risk] Compatibility could admit a stale dynamic layout record.** → Reuse only the layout record retained by the existing protocol-service connection generation and preserve every reset/replacement boundary.
- **[Risk] Adding a safe-crate dependency could create a cycle.** → Dependency remains one-way from compatibility policy to the lower dynamic-layout record owner; dynamic telemetry does not depend on compatibility.
- **[Risk] Apple-only catalogue metadata could leak into Android packaging.** → Preserve Android’s closed typed catalogue and audit its exact task/artifact/source set.
- **[Trade-off] The Swift record adapter may not reduce total bridge-era lines immediately.** → It contains only temporary representation validation; all source protocol/catalogue policy moves to Rust and the adapter/bridge disappear with unified Rust application behavior.

## Migration Plan

1. Add the canonical compatibility record, dynamic-layout-record admission, exhaustive safe tests, Apple bridge/C harness, catalogue metadata, and audits without selecting Swift.
2. Wire SwiftPM and Xcode, implement strict record mapping and accepted-decision propagation, and add focused Apple tests.
3. Select Rust request/decode/catalogue/layout admission, delete superseded Swift algorithms, then prove clean Apple packaging, Android isolation, strict OpenSpec validation, and `just verify`.
4. Run focused ESP32 regression on connected Android, iOS, and macOS, deferring only bounded location acquisition.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no persistence migration, frontend contract change, dual runtime path, or external deployment.

## Open Questions

None. Android compatibility selection and the unified Rust application/runtime boundary remain separate changes.
