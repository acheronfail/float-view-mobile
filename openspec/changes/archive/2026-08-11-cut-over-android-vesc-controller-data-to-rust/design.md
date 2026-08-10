## Context

`backends/core/vesc-controller-data` is a safe, dependency-free decoder for the consumed VESC `COMM_GET_VALUES` command-4 prefix and `COMM_GET_MCCONF` command-14 current-limit subset. Both Apple hosts already select it through a versioned two-symbol C ABI. Android still selects the compact parser in `VescProtocolService.kt`, where the byte reader, fixed-point scales, VESC auto-float algorithm, supported serialization-signature table, optional series-cell lookup, and fault classification are used only by these two decode methods.

Android already has a locked source-built JNI pattern for four other protocol components. The shared bridge catalog and Android builder own the pinned toolchain/NDK, host-JVM artifact, four Android ABI artifacts, architecture validation, and exact symbol auditing. This change should add one catalogued bridge and thin adapter while leaving framing, routing, scheduling, telemetry, application state, and the proven Apple bridge untouched.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of command-4 realtime and command-14 current-limit structural decoding.
- Keep Kotlin limited to bridge loading, fixed status/result validation, bounded tagged-record validation, primitive model conversion, and established error mapping.
- Build and package the focused bridge from locked offline source for host JVM tests and all four supported Android ABIs in Debug and Release.
- Delete the superseded Kotlin parser and fault classification after compiled-boundary, packaging, software, and all-host physical evidence passes.
- Preserve the existing Apple production path and every higher-level controller/OS owner.

**Non-Goals:**

- Moving request construction, compatibility, VESC framing, direct/CAN routing, scheduling, telemetry polling, fault response, application behavior, storage, SQLite, loopback transport, BLE, background work, location, lifecycle, files, theme, permissions, WebViews, or the frontend contract.
- Changing the safe decoder's protocol behavior or either Apple bridge/adapter/artifact.
- Sharing Apple's C artifact with Android, adding controller operations to another JNI library, introducing generated bindings, callbacks, Rust-owned handles, or a general native runtime.
- Deduplicating host record encoders or JNI plumbing in this cutover; that remains a separately reviewable migration/tooling concern.

## Decisions

### Use a separate two-symbol Android controller-data JNI crate

Add `vesc-controller-data-android-jni` beside the safe decoder and Apple bridge. It depends only on the safe component, Rust standard library, and the already locked minimal `jni-sys` binding. Its Kotlin bridge exposes exactly `abiVersion(): Int` and `decode(operation, input, output, result): Int`. The shared catalog gains the exact package, library stem, JNI prefix, and two-symbol allowlist.

Adding methods to the framing, identity, BMS, custom-configuration, or Refloat libraries was rejected because it would couple unrelated ownership and symbol allowlists. Calling the Apple C ABI from Android was rejected because JNI caller validation, packaging, and JVM-owned arrays are platform-specific.

### Cross one bounded caller-owned tagged record

`decode` accepts operation `1` for realtime or `2` for current limits, a nullable JVM byte-array input, a nullable output byte array of at most 60 bytes, and an exact two-element integer result. Rust validates pending exceptions, nulls, array lengths, pairwise aliasing, operation, maximum 4,096-byte input, and output capacity before copying. It initializes result metadata, invokes the safe decoder once, and copies a successful record only after complete encoding. `result[0]` is the produced or required record length and `result[1]` is the unsupported command byte.

The record matches the already audited version-1 Apple semantics: an eight-byte version/kind/flags/fault/cell/reserved header, followed by big-endian IEEE-754 values and signed big-endian ERPM. Realtime is exactly 60 bytes; current limits are exactly 40 bytes. Fixed statuses distinguish success, invalid caller contract, unsupported command, truncated/invalid payload, insufficient output, and contained internal failure.

Returning Rust structs, strings, direct buffers, native pointers, or opaque handles was rejected because it would transfer layout or lifetime ownership. A generated schema was rejected because two operations and two fixed records are smaller and easier to audit by hand.

### Validate the complete record again in Kotlin

Add `VescControllerDataDecoder` beneath Android `protocol/vesc`. It verifies the linked ABI, status metadata, exact record length/kind, known flags, reserved and irrelevant fields, fault-code/classification consistency, optional-cell consistency, finite doubles, nonzero current limits, and complete cursor consumption before constructing the existing models. Unsupported commands preserve `VescProtocolException.UnsupportedCommand`; all other bridge, payload, or record failures preserve `TruncatedPayload`.

The adapter supplies `VescFault(code, kind)` directly. The production `fromCode` classifier and `RealtimeValues` default are removed so Kotlin cannot silently retain a second classification algorithm. Tests may construct explicit expected faults.

### Reuse shared build tooling while keeping bridge artifacts independent

The shared catalog and Android builder gain the controller-data JNI identity. Gradle adds disjoint host/debug/release output directories, bridge-specific inputs, a host-library property for JVM tests, four-ABI packaging dependencies, and cleanup ordering. Existing bridges and Apple outputs remain separate. Root audits verify workspace membership, locked dependency depth, exact symbols, ABI agreement, four architectures in Debug/Release APKs, no generated/committed binary, exactly one production adapter selection, Kotlin algorithm deletion, and Apple isolation.

### Gate deletion on software and physical production evidence

Focused Rust and JVM tests cover normal/faulted realtime records, current-limit signatures and optional cells, every safe-decoder status, invalid operation/null/shape/alias/capacity/exception cases, ABI and record corruption, finite numeric and fault/cell consistency, panic containment, and payload-safe diagnostics. Clean host and four-ABI Debug/Release packaging plus `just verify` remain required.

Physical `identity`, `control-direct`, and `control-can` scenarios run on Android, iOS, and macOS, with `background` on both mobile platforms. Android must execute the new JNI decoder; Apple remains on its existing C bridge. Only absence of a location observation during the bounded dwell is deferred; permission, capability, monotonic boundary, BLE, routing, controller data, telemetry, state, session, control, background, and cleanup failures remain blocking.

## Risks / Trade-offs

- **[Risk] A sixth Android JNI library increases packaging surface.** → Keep a separate exact two-symbol allowlist, disjoint outputs, locked dependency depth, and four-ABI Debug/Release package-content audits.
- **[Risk] Apple and Android record encoders could drift.** → Use identical fixed version-1 bytes and maximums and assert exact records independently through Rust, C, Swift, and JVM tests; defer encoder deduplication to a bounded later change.
- **[Risk] Kotlin could trust a malformed native record.** → Validate every header, flag, reserved byte, extent, finite number, fault relation, cell option, and unused field before model construction.
- **[Risk] JNI misuse or panic could cross the boundary.** → Validate references, shapes, aliasing, primitive bounds, and pending exceptions; contain panics; retain no references; write output only after successful work.
- **[Risk] Parser deletion could alter higher-level behavior.** → Keep public models and error classes stable, select the adapter only inside the existing two methods, run the canonical suite and all-host physical identity/control/background checks.
- **[Risk] Physical delay could be mistaken for a location waiver.** → Defer only the missing sample observation; every other lifecycle and controller assertion stays mandatory.

## Migration Plan

1. Add the Android JNI crate, catalog identity, compiled-boundary tests, and source/dependency/symbol audits without changing the Kotlin production call site.
2. Wire disjoint host/debug/release outputs into Gradle and prove clean host and four-ABI packaging.
3. Add the thin Kotlin adapter, select it from the two existing decode methods, extend JVM tests, and delete the Kotlin structural algorithms and classifier.
4. Pass focused checks, clean packaging, strict OpenSpec validation, and `just verify`.
5. Run focused ESP32 physical identity, direct/CAN control, and mobile background regressions on Android, iOS, and macOS; rerun the complete software gate if physical work exposes any change.
6. Commit implementation, sync canonical specs, and archive the change.

Rollback is one repository revert because no contract, database, persisted representation, or migration changes. Generated native artifacts remain disposable and untracked.

## Open Questions

None.
