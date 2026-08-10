## Context

The safe `vesc-firmware-identity` crate already decodes every committed THOR300, VESC Express, stock VBMS, and VBMS32 identity and rejects the established malformed classes. It is deliberately isolated from hosts. The shared Apple backend still selects a 79-line Swift structural parser beneath both iOS and macOS, while Android selects an equivalent Kotlin parser.

Apple is the best first production host because one `FloatViewBackend` adapter serves both sibling Apple applications and the repository already has a proven pattern for locked Rust static-library builds, hand-written C modules, SwiftPM linkage, Xcode build phases, ABI tests, and symbol audits. This change transfers only firmware identity structure to Rust. Compatibility admission, route attribution, application coordination, OS services, and Android remain outside the boundary.

## Goals / Non-Goals

**Goals:**

- Make the safe Rust decoder the sole shared Apple production implementation of `COMM_FW_VERSION` structure on iOS and macOS.
- Preserve `FirmwareInfo`, replacement-decoded names, private UUID bytes, optional CRCs, constants, and existing Swift error classes.
- Add one narrow Apple-specific ABI with caller-owned memory, fixed statuses, panic containment, bounded data, no private diagnostics, and exact symbols.
- Build and link the bridge deterministically for macOS, iOS device, and iOS simulator from locked source.
- Delete the superseded Swift parser and prohibit a runtime or source fallback.
- Prove the transfer with focused Rust/C/Swift checks, clean packaging, `just verify`, and physical ESP32 identity runs on Android, iOS, and macOS.

**Non-Goals:**

- Cutting over Android or adding JNI; Kotlin remains Android's production identity owner.
- Combining the identity bridge with the existing framing bridge or creating a general native runtime.
- Moving firmware compatibility policy, CAN routing, request scheduling, framing, BMS values, Refloat, BLE, background/location/files/theme/lifecycle, application behavior, SQLite, loopback transport, or frontend code.
- Returning Rust allocations, exposing Rust layouts, adding callbacks/opaque handles/generated bindings, committing archives, or adding third-party dependencies.
- Changing the frontend contract, stored data, database schema, exports, or user-visible behavior.

## Decisions

### Use a separate focused Apple bridge

Add `backends/core/vesc-firmware-identity-apple-ffi`, depending only on `vesc-firmware-identity` and the standard library. Add a matching hand-written `CVescFirmwareIdentity` module and target-specific build helper. The safe crate retains `forbid(unsafe_code)` and host independence; unsafe pointer conversion exists only in the named bridge.

Extending `vesc-frame-apple-ffi` was rejected because identity payload semantics are not framing and would make the crate, header, symbol audit, and maintainer ownership misleading. Renaming the existing framing bridge into a general Apple runtime was rejected because it would disturb a verified production boundary and recreate the broad shared-core coupling this incremental migration avoids.

### Cross one versioned byte-record ABI

Expose exactly two application symbols: an ABI-version query and one decode operation. Decode accepts a borrowed bounded payload, a caller-owned result buffer and capacity, a result-length pointer, and a one-byte unsupported-command output. It returns fixed integer statuses for success, invalid arguments, unsupported command, structural/truncation failure, insufficient output, and contained internal failure. Every call is panic-contained; pointers are validated for null, alignment, range, and overlap before use; Rust retains nothing.

The successful result is a documented version-1 byte record rather than a C/Rust struct: five scalar identity bytes, flags, two big-endian name lengths, the 12-byte UUID, a four-byte big-endian optional CRC field, then the replacement-decoded UTF-8 hardware and firmware names. Name lengths are bounded for the worst-case UTF-8 expansion of the established 64-byte inputs, and the total record has a fixed maximum. Swift validates flags, lengths, exact record extent, and option presence before mechanically constructing `FirmwareInfo`.

A `repr(C)` result struct was rejected because it would cross language layout and padding rules. A long list of scalar/name output pointers was rejected because it creates a larger unsafe alias-validation surface. Returning raw Rust strings or allocations was rejected because ownership and lifetime would cross the bridge.

### Preserve Swift protocol surfaces and error mapping

Keep the public `VescFirmwareIdentityDecoder` name, `decode(_:)`, motor/BMS constants, and all callers. Its body becomes a thin `CVescFirmwareIdentity` adapter. The unsupported-command status maps to `.unsupportedCommand(observedByte)`; structural and bounded bridge failures map to `.truncatedPayload`. ABI mismatch or impossible result shape logs only a bounded status/reason without payload, names, UUID, address, or full identity.

Compatibility catalogs and route attribution continue after the returned `FirmwareInfo`, unchanged. Swift may parse only the private ABI record; it SHALL contain no `COMM_FW_VERSION` prefix, terminator, hardware-tail, capability, or CRC-tail algorithm and no fallback feature flag.

### Reuse the established Apple packaging pattern without coupling bridges

The new helper maps macOS, iOS device, and active simulator SDK/architectures to pinned Rust targets, uses `--locked --offline`, and writes an uncommitted static archive to the existing target-specific build area. SwiftPM adds the C module and library. Xcode adds declared source inputs, one output archive, include path, library search path, and link flag through a separate build phase so either bridge can fail independently with an actionable error.

Root recipes build both Apple bridges wherever Swift build/test/run/package requires them. Focused C tests compile against the public header and archive. Symbol audits allow exactly the two identity symbols and keep the existing four framing symbols unchanged. Repository audits reject committed native artifacts, generated bindings, unexpected dependencies, and Apple production references to the safe crate outside the bridge.

### Treat deletion plus physical execution as ownership evidence

Rust tests retain source-derived/adversarial parity and add bridge buffer/status/panic tests. A C harness verifies header constants, version, source-derived decode, invalid pointers/capacities/overlap, unsupported-command propagation, truncation, and required output length. Swift tests exercise the unchanged decoder API through Rust for motor, Express, stock/custom BMS, private fields, malformed payloads, invalid UTF-8, and ABI/result validation.

After focused and clean packaging checks, `just verify` proves every software path. Physical `identity` runs on iOS and macOS must traverse the selected Rust adapter over real BLE fragmentation; Android must pass through unchanged Kotlin ownership. The Swift structural parser is deleted and audits prevent it from reappearing. Location sample absence during a bounded dwell remains deferred manual coverage, but identity-only runs do not require a location observation.

## Risks / Trade-offs

- **A second Apple static bridge adds packaging surface.** → Keep it two-symbol, reuse the proven helper/module pattern, and audit both archives independently rather than creating a premature general runtime.
- **Variable-length names can expand during UTF-8 replacement.** → Bound each result length to three times the 64-byte input limit, document the exact maximum record, and validate every offset before Swift slicing.
- **Swift still maps a private ABI record.** → Limit mapping to primitive representation conversion; all VESC structure, terminator, tail, and CRC decisions remain Rust-owned and source audits prohibit duplication.
- **ABI bugs could expose private UUID/name data.** → Use caller-owned bounded memory, never format the result, keep errors byte-free, and test redacted diagnostics and failure paths.
- **Apple succeeds while Android remains duplicated temporarily.** → Make Android ownership explicit, require its physical regression, and follow with a separate Android bridge/cutover change.
- **A physical Apple failure could block deletion.** → Keep the change uncommitted and reversible until both Apple hosts pass; no contract or data migration requires rollback work.

## Migration Plan

1. Add the focused bridge crate, hand-written header/module, build helper, unit/C tests, dependency and symbol audits while Swift remains selected.
2. Add SwiftPM and Xcode source-build/link integration for macOS, iOS device, and simulator; prove clean Debug/Release packaging.
3. Replace the shared Swift decoder internals with the Rust adapter, add bridge-driven Swift parity/failure tests, and remove the structural Swift algorithm.
4. Run focused checks and the complete `just verify` gate with no fallback.
5. Run focused physical identity conformance on Android, the connected iOS device, and macOS with the ESP32 fixture; Apple exercises Rust and Android proves no regression.
6. Confirm source/symbol/dependency audits after deletion, commit the implementation, then sync and archive the OpenSpec change.

Rollback reverts the isolated bridge, C module, packaging entries, adapter, and Swift deletion together. There is no persistence, contract, or data migration.

## Open Questions

None. Android bridge shape and cutover remain deliberately deferred to the next independent change.
