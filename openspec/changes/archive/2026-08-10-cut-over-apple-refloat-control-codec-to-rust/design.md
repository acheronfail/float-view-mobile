## Context

`float-view-refloat-control` is a safe, dependency-free, stateless codec for the complete bounded Refloat live-control wire slice. It already exhausts every source-derived package, flywheel, lighting, query, and fatal-clear request; both lighting profiles and all two-bit state combinations; exact response states; malformed inputs; unavailable commands; fixed capacities; and redacted failures. Apple production still implements those bytes and lighting-response checks in `RefloatControlCodec.swift`, shared by iOS and macOS.

Apple already source-builds independent audited C bridges for VESC framing, firmware identity, custom configuration, and BMS values. The recent bridge-tooling change centralizes their toolchain, target, package, artifact, and symbol metadata, so a new focused bridge needs one catalogue entry rather than another build script. The cutover must remain offline and reproducible for macOS, iOS device, and iOS simulator; cross no Rust layout or allocation; preserve compatibility/routing/application owners; leave Android unchanged; and delete the Swift wire algorithm after verification.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Apple production owner of Refloat package, flywheel, legacy/stable lighting, zero-mask query, fatal-clear request encoding, and lighting-response structural decoding.
- Add one narrow versioned three-symbol Apple C ABI using fixed primitive inputs, caller-owned bounded output, panic containment, exact statuses, pairwise region validation, and payload-safe diagnostics.
- Reuse the shared bridge catalogue and generic Apple build helper for every Apple target family without generated bindings or committed archives.
- Preserve Swift control/profile/state models, `VescProtocolError`, compatibility policy, direct/CAN routing, VESC framing, authorization, scheduling, confirmation, telemetry, application state, and frontend behavior.
- Delete the superseded Swift wire algorithm and prove production selection with focused Rust/C/Swift checks, clean packaging, `just verify`, and all-host ESP32 control regression.

**Non-Goals:**

- Cutting Android over from its Kotlin Refloat control codec.
- Moving Refloat compatibility lookup, routing, framing, mutation coordination, scheduling, confirmation, telemetry, configuration, application behavior, persistence, SQLite, the loopback server, or any OS capability.
- Combining this ABI with an existing VESC bridge, adding a general generated binding layer, or crossing callbacks, Rust allocations, references, layouts, or opaque lifetimes.
- Changing the frontend contract, persisted data, exports, or user-visible control semantics.

## Decisions

### Use a dedicated three-symbol Apple bridge

Add `refloat-control-apple-ffi` with exactly an ABI-version query, one encode operation, and one lighting-decode operation. Add the hand-written `CVescRefloatControl` module beneath the shared Apple package. The bridge depends only on `float-view-refloat-control` and the Rust standard library; the safe crate remains independent of hosts and unsafe code.

The encode call receives fixed operation, lighting-profile, fatal-availability, target, enabled, and current-flags bytes and writes at most nine bytes plus one output length and unsupported-command byte into caller-owned regions. It admits only canonical inputs for the selected operation. The decode call receives a bounded borrowed response and selected lighting profile and writes one validated two-bit lighting-flags result. Fixed statuses distinguish success, invalid caller contract, malformed payload, unsupported command, insufficient output, and contained internal failure.

Adding these operations to the custom-configuration bridge was rejected because it would couple independent protocol release and symbol surfaces. One C function per control was rejected because it expands the exported surface without improving ownership. A serialized record, Rust allocation, callback, or opaque handle was rejected because this state is at most nine request bytes or one result byte.

### Keep unsafe work mechanical and bounded

The bridge validates raw enum and boolean values, canonical unused parameters, output capacity, scalar alignment, arithmetic, and pairwise disjoint regions before creating slices or writing results. Decode rejects a response extent above the exact four-byte codec input before borrowing it. Every exported operation contains panics and initializes scalar outputs only after validating them. Diagnostics expose only fixed reasons or numeric statuses, never payloads, state, addresses, tokens, or full telemetry.

The safe crate remains the only owner of Refloat command bytes, package prefix, profile commands, masks, flags, query semantics, fatal-clear availability, exact response shape, and semantic errors. Duplicating those decisions in the bridge was rejected because it would create a second Rust protocol implementation.

### Preserve the shared Swift service and application boundaries

Keep `RefloatBooleanControl`, `RefloatLightingState`, `RefloatControlProfile`, and the `RefloatControlCodec` call location beneath `protocol/refloat`. Replace its wire construction and response checks with primitive conversion, ABI/status validation, caller-owned buffer management, and established error mapping. Package and flywheel helpers become throwing so ABI or contained bridge failures fail closed; their production callers already operate inside throwing service/coordinator boundaries. Lighting and fatal-clear retain their existing failure classes.

`VescProtocolService`, `RefloatMutationCoordinator`, compatibility lookup, framing, route wrapping, pending-work exclusion, confirmation, and state publication remain unchanged apart from propagating the codec failure. The Swift `RefloatWire` helper remains because telemetry/info code still uses its package and command helpers; only the control-specific algorithm is deleted.

Moving profile lookup or mutation state into Rust was rejected because those are policy/application concerns and would combine independently testable layers. Keeping a Swift fallback was rejected because it would leave ambiguous production ownership.

### Reuse locked packaging and existing direct/CAN physical evidence

Add one `refloat-control` entry to the shared bridge catalogue and wire the generic Apple helper into SwiftPM and the iOS Xcode project for macOS, iOS device, and simulator. Root verification builds the bridge once per required target family, compiles a C harness, audits the exact three symbols and ABI version, checks the two-node dependency graph, rejects generated or committed artifacts, proves the sole Swift production call path, and proves Android isolation.

After focused checks and `just verify`, run `control-direct` and `control-can` on the connected iOS device and macOS host so both routing forms execute through Rust. Run the same scenarios on connected Android through unchanged Kotlin. Include the mobile background scenario where supported to preserve the existing background/session gate. Only a missing timed location observation is deferred; BLE, routing, control, confirmation, state, background/session, and cleanup failures remain blocking.

## Risks / Trade-offs

- **The bridge may be larger than the deleted Swift algorithm.** → Keep one consolidated three-operation ABI, reuse the shared build tooling, depend only on the safe crate, and treat the bridge as mechanical memory/status conversion rather than protocol code.
- **Primitive operation inputs could admit ambiguous combinations.** → Require exact canonical values for unused fields and exhaust invalid operation/profile/target/boolean/current-state combinations in Rust and compiled-C tests.
- **A bad caller region could cause memory unsafety.** → Bound every region, validate alignment and pairwise disjointness, initialize outputs deterministically, contain panics, and exercise null/capacity/overlap cases through the compiled ABI.
- **New throws from package/flywheel encoding could surface an internal bridge failure.** → Propagate the established bounded protocol error through already-throwing service/application boundaries instead of crashing or silently constructing a fallback payload.
- **Apple and Android temporarily select different implementations.** → Preserve identical models/fixtures and run the same direct/CAN physical scenarios; make Android selection the next independent reversible OpenSpec change.

## Migration Plan

1. Add the bridge crate, fixed ABI, C header/module/harness, shared-catalogue entry, dependency/symbol audits, and target builds without selecting it.
2. Wire SwiftPM and Xcode packaging, replace the shared Swift wire algorithm with the thin adapter, update callers for closed error propagation, and add exhaustive mapping/failure tests.
3. Delete the superseded Swift algorithm and prove exact production selection, Android isolation, clean Apple packaging, canonical behavior, and `just verify`.
4. Run focused direct/CAN Refloat control regression on Android, iOS, and macOS with the ESP32 fixture, plus mobile background continuity where applicable.
5. Commit the verified implementation, sync the delta specs, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no runtime dual path, persisted-data migration, or frontend contract change to reverse.

## Open Questions

None. Android production selection and broader Refloat/application ownership remain deliberately separate later changes.
