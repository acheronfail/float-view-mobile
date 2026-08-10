## Context

The dependency-free `vesc-bms-values` crate already strictly decodes bounded VESC command-96 payloads and passes the committed legacy/extended fixtures, zero/maximum collection bounds, and adversarial failures. Apple production still performs that same field, collection, auto-float, reported-CAN, and tail parsing in `VescProtocolService.swift`; iOS and macOS share this implementation. Apple already source-builds separate audited C bridges for framing, firmware identity, and custom-configuration, so this change can reuse that integration pattern without moving BMS lifecycle or application concerns.

The bridge must build offline for macOS, iOS device, and iOS simulator; expose no Rust layout, allocation, reference, or lifetime; preserve the existing Swift API and errors; avoid payload-bearing diagnostics; and leave Android's Kotlin decoder and packaging graph unchanged. The existing physical BMS scenarios already exercise standalone direct BMS, controller/CAN BMS, Express/CAN BMS, polling, authoritative state, and cleanup.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Apple production owner of command-96 structural decoding on both sibling Apple hosts.
- Add one narrow, versioned two-symbol Apple C ABI using caller-owned bounded regions, panic containment, fixed statuses, and an exact symbol allowlist.
- Preserve `VescPayloadDecoder.bmsValues`, `BmsSnapshot`, observation time, errors, direct/CAN routing, discovery, scheduling, polling, availability, and application behavior.
- Build and link the bridge reproducibly for macOS, iOS device, and iOS simulator without generated bindings or committed native archives.
- Prove production selection with focused Rust/C/Swift checks, clean packaging, `just verify`, and existing BMS physical scenarios on iOS and macOS, plus Android regression.
- Delete the superseded Swift BMS structural parser and retain no fallback.

**Non-Goals:**

- Cutting Android over from its Kotlin BMS values decoder.
- Moving BMS compatibility, firmware identity, discovery, request construction, routing, scheduling, polling, observation time, availability, state publication, or live-only policy.
- Moving other VESC or Refloat protocols, application behavior, SQLite, the loopback server, BLE, background work, location, files, theme, lifecycle, permissions, persistence, or the frontend contract.
- Combining this ABI with an existing Apple bridge or introducing generated bindings, callbacks, opaque handles, or a Rust-owned runtime.

## Decisions

### Use a separate two-symbol Apple bridge

Add `vesc-bms-values-apple-ffi` with exactly an ABI-version query and one decode operation, plus a hand-written `CVescBmsValues` module. The decode operation accepts one borrowed payload, an expected-CAN byte using `255` as the no-expectation sentinel, one caller-owned result region, and caller-owned result-length and unsupported-command outputs. It maps only fixed success, invalid-argument, unsupported-command, malformed-payload, insufficient-output, and contained-internal-failure statuses.

This keeps command-96 evolution and symbol auditing independent of framing, identity, and configuration. Adding the operation to an existing bridge would couple unrelated release surfaces; returning a Rust allocation, C struct, or object handle would introduce layout or lifetime obligations for a stateless bounded transform.

### Cross one explicit version-1 byte record

On success Rust writes a byte record with an eight-byte header: record version, known option flags, cell count, temperature count, reported CAN ID, optional data version value, status-byte length, and a zero reserved byte. It then writes the exact big-endian IEEE-754 bits for the 17 fixed scalar values, followed by counted cell-voltage doubles, one-byte balancing flags, counted temperature doubles, and status UTF-8 bytes. The extended-tail flag distinguishes a legacy result from an extended result even when status is empty. The maximum record is statically bounded at 1,034 bytes.

Swift validates the ABI, version, flags, reserved byte, counts, exact checked record extent, finite doubles, boolean bytes, reported-CAN range, and extended-tail consistency before mechanically constructing `BmsSnapshot` and attaching the caller-supplied `observedAt`. Rust remains responsible for every VESC payload decision. A native-endian C struct was rejected because padding and ABI layout vary; multiple array outputs were rejected because they multiply unsafe pointer regions; textual serialization was rejected because it adds parsing and numeric-format complexity.

### Keep unsafe work mechanical and bounded

The bridge limits input to the safe decoder's 350-byte maximum, output capacity to the fixed record maximum, validates null/length pairs, scalar alignment, arithmetic overflow, and pairwise region disjointness before creating slices, and contains panics before C. It delegates all command, fixed-point, collection, flag, CAN, auto-float, UTF-8, tail, remainder, and numeric validity decisions to `vesc-bms-values`. Diagnostics contain only fixed reasons or status codes, never payloads, values, arrays, status text, addresses, or tokens.

This preserves `#![forbid(unsafe_code)]` and the empty dependency graph in the safe crate. Duplicating validation in the unsafe layer or exposing the borrowed Rust status was rejected because either creates a second protocol implementation or crosses a lifetime boundary.

### Preserve the shared Swift service boundary

Add a thin `VescBmsValuesDecoder` under Apple `protocol/vesc`; `VescPayloadDecoder.bmsValues` delegates to it once. `VescProtocolService`, application scheduling, and callers keep their current signatures and models. The generic Swift `ByteReader` remains for realtime values and controller current limits, while BMS-only count, balancing, reported-CAN, status-tail, and remainder logic is deleted.

This isolates the ownership transfer below application coordination. Moving `BmsSnapshot`, observation timestamps, or polling into Rust now would combine protocol representation with host lifecycle and make regression attribution harder.

### Reuse locked Apple packaging and existing physical evidence

The bridge follows the existing pinned-toolchain build helper, SwiftPM system-library/link settings, and Xcode shell-build/search/link conventions for all Apple target families. Root checks add compiled C ABI coverage, exact two-symbol and ABI-version audits, dependency/layout constraints, clean Apple packaging, one selected Swift call site, parser-deletion checks, and Android isolation.

After focused software checks and `just verify`, run `bms-direct-controller`, `bms-express`, and `bms-standalone` through the connected iOS device and macOS host, where command-96 now executes through Rust, and through connected Android, where Kotlin remains selected. A new physical scenario was rejected because these existing scenarios already distinguish every supported BMS topology. Only a missing location observation within its bounded dwell is deferred; BLE, BMS, routing, polling, state, monotonic, session, and cleanup failures remain blocking.

## Risks / Trade-offs

- **A fourth Apple bridge expands build surface.** → Reuse the established source-build convention, keep exactly two symbols and one safe dependency, and audit all three Apple target families.
- **A record offset or capacity error could corrupt memory or mis-map values.** → Use one bounded output region, checked pairwise region validation, fixed big-endian fields, exact-length validation, Rust and compiled-C adversarial tests, and Swift record-rejection tests.
- **The 1,034-byte maximum record copies more data than a native model.** → Actual records are count-sized, remain below one BLE payload transform's bounded temporary storage, and avoid allocator or lifetime ownership across FFI.
- **Swift's generic reader still contains auto-float code for controller limits.** → Audit absence of BMS-specific collection/tail logic and exact Rust selection rather than incorrectly deleting shared current-limit behavior.
- **Apple and Android temporarily select different implementations.** → Preserve the same host API and fixtures, run identical physical topology scenarios, and make Android its own subsequent reversible OpenSpec change.

## Migration Plan

1. Add and test the bridge crate, fixed record, C header/module/harness, source-build helper, symbol allowlist, and dependency/bounds audits without selecting it.
2. Wire SwiftPM and Xcode builds for macOS, iOS simulator, and iOS device, then add the thin shared Swift adapter and focused mapping/error tests.
3. Select the adapter from `VescPayloadDecoder.bmsValues`, delete the superseded Swift structural parser, and prove source selection, Android isolation, clean packaging, and `just verify`.
4. Run the three existing focused BMS physical scenarios on connected Android, iOS, and macOS against the ESP32 fixture, deferring only bounded location-sample absence.
5. Commit the verified implementation, sync delta specs, and archive the change.

Rollback is a source revert of the coherent implementation commit; there is no runtime dual path, data migration, or persisted format to reverse.

## Open Questions

None. Android production selection and higher-level BMS/application ownership remain deliberately separate later changes.
