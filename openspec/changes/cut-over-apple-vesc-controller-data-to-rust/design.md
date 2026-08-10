## Context

The dependency-free `vesc-controller-data` crate already strictly decodes the consumed command-4 realtime prefix and command-14 current-limit subset and passes committed fixtures, all supported motor-configuration signatures, numeric boundaries, and adversarial failures. Apple production still duplicates those decisions in `VescProtocolService.swift`; iOS and macOS share that implementation. Apple already source-builds independent audited C bridges through one allowlisted builder, so this change can reuse that integration pattern without moving request, routing, polling, state, or lifecycle concerns.

The bridge must build offline for macOS, iOS device, and iOS simulator; expose no Rust layout, allocation, reference, or lifetime; preserve existing Swift APIs and errors; avoid payload-bearing diagnostics; delete the Swift fallback; and leave Android's Kotlin decoder and packaging graph unchanged.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Apple production owner of consumed command-4 realtime and command-14 current-limit structural decoding on both sibling Apple hosts.
- Add one narrow, versioned two-symbol Apple C ABI using a tagged caller-owned record, panic containment, fixed statuses, and an exact symbol allowlist.
- Preserve `VescPayloadDecoder.realtime`, `VescPayloadDecoder.currentLimits`, `RealtimeValues`, `VescFault`, `ControllerCurrentLimits`, errors, routing, framing, scheduling, telemetry, and application behavior.
- Build and link the bridge reproducibly for macOS, iOS device, and iOS simulator without generated bindings or committed native archives.
- Prove production selection with focused Rust/C/Swift checks, clean packaging, `just verify`, and focused physical controller scenarios on iOS and macOS plus Android regression.
- Delete the superseded Swift byte reader, scaling, auto-float, signature/offset, and fault-classification algorithms with no fallback.

**Non-Goals:**

- Cutting Android over from its Kotlin controller-data decoder.
- Moving request construction, firmware compatibility, direct/CAN routing, VESC framing, scheduling, telemetry polling, fault response policy, state publication, or persistence.
- Moving Refloat layouts, application behavior, SQLite, loopback transport, BLE, background work, location, files, theme, lifecycle, permissions, or the frontend contract.
- Combining this ABI with an existing protocol bridge or introducing generated bindings, callbacks, opaque handles, Rust-owned runtime state, or common FFI plumbing shared by existing bridges.

## Decisions

### Use a separate two-symbol Apple bridge

Add `vesc-controller-data-apple-ffi` with exactly an ABI-version query and one decode operation, plus a hand-written `CVescControllerData` module. The decode call accepts a fixed operation value for realtime or current limits, one borrowed payload, one caller-owned record region, and caller-owned record-length and unsupported-command outputs. It maps only success, invalid-argument, unsupported-command, truncated-payload, insufficient-output, and contained-internal-failure statuses.

One generic decode symbol avoids duplicating the identical region and status contract for two stateless results. Adding these operations to another protocol bridge was rejected because it would couple unrelated release surfaces; returning a Rust allocation, C struct, callback, or object handle was rejected because it would introduce layout or lifetime obligations.

### Cross one explicit tagged version-1 record

Every successful record starts with eight bytes: record version, result kind, option flags, Rust fault kind, raw fault code, optional series-cell value, and two reserved zero bytes. A realtime record then carries six big-endian IEEE-754 f64 values—controller temperature, motor temperature, motor current, battery current, duty cycle, and pack voltage—followed by signed big-endian ERPM, for an exact 60-byte result. A current-limit record carries four big-endian f64 limits, for an exact 40-byte result. One flag distinguishes an absent series-cell value from a present value; irrelevant header fields must be zero.

Swift validates ABI, version, result kind, flags, reserved and irrelevant bytes, exact extent, finite doubles, ERPM representation, fault-kind range, and positive optional cell count before mechanically constructing the existing models. Rust remains responsible for scales, signs, fault classification, auto-float, serialization signatures, cell offsets, and numeric validity. Separate output arrays and textual serialization were rejected because they multiply unsafe regions or parsing complexity.

### Keep unsafe work mechanical and bounded

The bridge accepts at most the established 4,096-byte VESC payload bound, limits output capacity to 60 bytes, validates operation, null/length pairs, scalar alignment, arithmetic overflow, and pairwise region disjointness before creating slices, and contains panics before C. It delegates all command, truncation, scaling, fault, auto-float, signature, offset, optional-cell, and numeric decisions to `vesc-controller-data`. Diagnostics contain only fixed reasons or status codes, never payloads, decoded values, addresses, or tokens.

This preserves `#![forbid(unsafe_code)]` and the empty dependency graph in the safe crate. A shared unsafe support crate or macros across existing bridges was rejected for this step because it would broaden regression scope across already-selected protocol paths.

### Preserve the shared Swift service boundary

Add a thin `VescControllerDataDecoder` under Apple `protocol/vesc`; `VescPayloadDecoder.realtime` and `currentLimits` delegate to it. The adapter obtains both raw fault code and Rust classification, so `VescFault` no longer classifies codes in Swift. Once selected, delete the generic Swift `ByteReader`, current-limit signature table, fixed-point/auto-float logic, and direct parsing from `VescProtocolService.swift`.

This isolates the ownership transfer below application coordination. Moving models, telemetry cadence, route decisions, current-limit publication, or fault handling into Rust now would combine structural decoding with host behavior and make regression attribution harder.

### Reuse shared packaging and focused physical evidence

Add one catalog entry and retain the shared Apple builder, SwiftPM system-library/link settings, and Xcode source-build/search/link conventions for all Apple target families. Root checks add compiled C coverage, exact two-symbol and ABI-version audits, dependency/layout constraints, clean Apple packaging, selected Swift call sites, parser-deletion checks, and Android isolation.

After focused software checks and `just verify`, run `identity`, `control-direct`, and `control-can` through iOS and macOS, where controller data now executes through Rust, and through Android, where Kotlin remains selected. Run `background` on both mobile hosts to preserve BLE/session continuity. Only a missing location observation within its bounded dwell is deferred; permission, capability, monotonic-boundary, BLE, controller-data, routing, state, session, control, and cleanup failures remain blocking.

## Risks / Trade-offs

- **[Risk] Another bridge increases Rust/C packaging surface.** → Reuse the shared allowlisted builder, keep exactly two symbols and one safe dependency, and audit all three Apple target families.
- **[Risk] A tagged-record offset or kind mismatch could silently mis-map values.** → Use exact fixed extents, big-endian fields, strict irrelevant/reserved-byte validation, Rust and compiled-C record tests, and Swift record-rejection tests.
- **[Risk] A single decode symbol adds an operation discriminator.** → Keep the operation closed to two constants and require the safe decoder to validate the payload command independently.
- **[Trade-off] Apple and Android temporarily select different implementations.** → Preserve the same host API and fixtures, run identical physical scenarios, and make Android its own subsequent reversible OpenSpec change.

## Migration Plan

1. Add and test the bridge crate, tagged record, C header/module/harness, catalog entry, and dependency/bounds audits without selecting it.
2. Wire SwiftPM and Xcode builds for macOS, iOS simulator, and iOS device, then add the thin shared Swift adapter and focused mapping/error tests.
3. Select the adapter from both `VescPayloadDecoder` entry points, delete the superseded Swift algorithms, and prove source selection, Android isolation, clean packaging, and `just verify`.
4. Run the focused controller and mobile-background physical scenarios on connected Android, iOS, and macOS against the ESP32 fixture, deferring only bounded location-sample absence.
5. Commit the verified implementation, sync delta specs, and archive the change.

Rollback is a source revert of the coherent implementation commit; there is no runtime dual path, data migration, or persisted format to reverse.

## Open Questions

None. Android production selection and higher-level controller/application ownership remain deliberately separate later changes.
