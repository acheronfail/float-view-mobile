## Context

The preceding Android cutover selected Rust for VESC CRC, frame encoding, and exact frame decoding while deliberately leaving notification-stream reassembly in Kotlin. Production ownership is therefore split at a framing boundary: `VescFrameStreamDecoder` still decides where noise ends, which header form is present, how much input is incomplete, the complete candidate extent, how malformed or oversized input makes progress, and when aggregate storage overflows, while Rust independently validates every selected frame.

The safe `backends/core/vesc-frame` crate already exposes a bounded, stateless `scan_next` operation used by its tested `StreamDecoder` and by both Apple production hosts. Android already source-builds and packages a small versioned JNI bridge for the same crate on the host JVM and four application ABIs. This makes Android stream-boundary selection the smallest next vertical cut: it reuses an existing production algorithm and delivery path without moving Bluetooth, payload interpretation, application state, persistence, transport, or a long-lived object across JNI.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole source of Android VESC stream-boundary decisions while preserving the public Kotlin codec/stream interfaces and observable error and recovery behavior.
- Extend the existing audited JNI bridge by one bounded stateless operation using JVM-owned primitive arrays and fixed integer statuses.
- Keep Kotlin responsible only for bounded byte storage, mechanical slicing/consumption, reset, and stable Kotlin error mapping.
- Prove parity through safe-crate, bridge, host-JVM, packaging, canonical, and representative physical Android background-fragmentation evidence before deleting the Kotlin parser.
- Keep Apple framing, the frontend contract, stored data, and OS capability ownership unchanged.

**Non-Goals:**

- Moving VESC command or payload interpretation, firmware identity, BMS, Refloat, application behavior, SQLite, the loopback server, Bluetooth, location, lifecycle, files, theme, permissions, or any other OS capability into Rust.
- Introducing Rust-owned stream state, opaque handles, callbacks, returned native pointers, generated bindings, or allocator ownership across JNI.
- Changing Android's public protocol errors, connection-generation reset behavior, notification ordering, product bounds, Gradle module structure, packaged ABI set, or user-visible behavior.
- Changing the Apple C ABI, Apple packaging, or Apple stream adapter.
- Adding a dependency or committed native artifact.

## Decisions

### Reuse the existing safe stateless scanner

Android will call the existing safe `scan_next` primitive rather than add an Android-specific parser or expose the Rust `StreamDecoder` as a native object. The scanner borrows the current bounded byte region and returns one deterministic outcome: more input is needed, one candidate frame is complete, the declared payload is oversized, or aggregate input is oversized. Its result contains only validated frame offset, frame length, and consumed-prefix length values.

Rust owns leading-noise disposal, short/long header interpretation, declared-length bounds, incomplete suffix preservation, complete-frame extent, malformed progress, and the 8,192-byte aggregate bound. A complete candidate is still passed to the existing Rust exact decoder for footer and CRC validation. The reported consumed prefix includes leading noise and the candidate frame, and Kotlin consumes that prefix before propagating an exact-decode error, preserving current recovery. An oversized declaration consumes leading noise plus the offending header byte; aggregate overflow consumes the complete stored region.

Reusing the scanner is preferred over copying the Apple bridge implementation into Kotlin because it retains one safe algorithm. A Rust-owned `StreamDecoder` handle was rejected because it would introduce native lifetime, destructor, connection-reset, and allocator contracts solely for temporary storage ownership that a later application migration is likely to replace.

### Add one fixed-shape JNI scanner operation and bump the ABI

The Android JNI contract will advance from version 1 to version 2 and add exactly one `scanNext` application entry point beside version, encode, and exact decode. Kotlin supplies a JVM-owned input `ByteArray` and an exact three-element `IntArray`; the result fields are frame offset, frame length, and consumed length. Product bounds fit safely in signed JVM integers. The operation returns the existing fixed statuses plus `NEED_MORE_INPUT`, matching the semantic vocabulary already used by the Apple bridge.

The bridge validates the environment, pending-exception state, nullability, exact result-array shape, reference identity, input length, output initialization, and all returned ranges. Inputs at or below the stream bound are copied call-locally before invoking safe Rust. Aggregate input above the bound is rejected without copying its contents and reports its full length for caller clearing. Semantic success, incomplete, and oversized outcomes write only validated primitive results; invalid caller contracts and contained internal failures expose no partial result. No reference or allocation survives the call.

One three-element result array is preferred over three one-element arrays because it reduces JNI calls and alias combinations while retaining explicit caller ownership and fixed layout. Returning a Java/Kotlin object was rejected because it adds allocation and class/marshalling surface; packing fields into the integer status was rejected because it obscures bounds and error semantics.

### Keep the Kotlin decoder stateful but algorithm-free

`VescFrameStreamDecoder` continues to own its Kotlin byte collection so its API, reset behavior, and protocol-service call sites remain stable. After appending notification bytes, it repeatedly presents the current bytes to Rust, validates the returned primitive ranges defensively, mechanically slices any reported candidate, removes exactly the reported prefix, and calls the existing Rust exact decoder. It stops on the Rust incomplete result, removes any reported disposable noise, maps fixed failures onto established `VescProtocolException` cases, and clears or advances storage exactly as Rust reports.

Kotlin MUST NOT inspect header marker bytes, calculate declared payload length or frame extent, search for noise boundaries, choose incomplete suffixes, enforce a parallel framing overflow rule, or implement alternative recovery. Mechanical checks that Rust-returned indices fit the caller-owned collection are retained as an FFI safety boundary, not a framing interpretation. No production fallback invokes the old parser.

The current storage representation will remain unless a mechanical change is needed to preserve bounded operation; changing it for performance is outside this ownership cutover. Two JNI calls per complete frame—scan followed by exact decode—are accepted because notification storage is capped at 8,192 bytes, the split reuses the already-audited exact decoder, and the same shape is proven on Apple.

### Preserve an additive checkpoint before final deletion

Implementation will first add and verify the JNI operation without selecting it. Production Kotlin will then select Rust scanning with no fallback while the previous parser may exist only as a private test-source parity reference. Focused software, clean packaging, canonical Android, unchanged Apple, and `just verify` gates run through the selected path before the physical checkpoint.

The required physical checkpoint is the production Android BLE fragmented-notification ride with background/foreground recovery. Location permission and background-capability validation remain part of the checkpoint, but a device that produces no location sample during the fixed automated dwell reports that persistence coverage as deferred to manual end-to-end verification rather than failing this framing migration. Only after the BLE/session checkpoint passes will the temporary reference and superseded Kotlin parsing code be removed and repository audits tightened to reject header/length/noise/extent/overflow decisions outside Rust. The complete software and packaging gates run again after deletion.

## Risks / Trade-offs

- **JNI scanner outputs and Kotlin consumption could disagree.** → Use fixed-shape results, bridge-side range construction, Kotlin defensive range validation, and immutable parity vectors covering exact offsets and consumption for noise, split headers, multiple frames, malformed candidates, oversized declarations, and overflow.
- **A fourth JNI entry point expands the unsafe bridge surface.** → Reuse the current validation, panic-containment, dependency, host-JVM, and exact-symbol-audit patterns; add only borrowed input and one fixed caller-owned result array.
- **Scanning and exact decoding use two JNI calls per complete frame.** → Keep buffers strictly bounded, measure through canonical and physical paths, and avoid adding stateful native lifetime merely to combine calls.
- **Kotlin continues to own stream bytes temporarily.** → Treat storage as caller-owned mechanics only; defer native lifetime until a later application-owner change can absorb it coherently.
- **Aggregate overflow is reported only after Kotlin receives the fragment.** → Preserve the existing public append surface and immediately call the no-copy oversized JNI path; production BLE notifications are already bounded, and retained storage is cleared using Rust's result.
- **Physical evidence can delay deletion.** → Select Rust additively with no production fallback, retain any comparison parser only in tests, and do not complete or archive the change until deletion and the final rerun succeed.

## Migration Plan

1. Extend the Android bridge's internal result/status model and JNI operation, bump the fixed ABI, and prove safe-scanner parity, caller validation, panic containment, exact symbols, and host/four-ABI source builds without changing production Kotlin selection.
2. Add the Kotlin bridge declaration and thin scanner adapter, select it from `VescFrameStreamDecoder`, and preserve established storage, reset, ordering, errors, and recovery while quarantining any comparison parser to tests only.
3. Run focused Rust/JNI/Kotlin tests, clean Debug/Release packaging, APK and ownership audits, canonical Android execution, unchanged Apple gates, and `just verify` through Rust scanning with no fallback.
4. Run the representative physical Android fragmented BLE ride and background/foreground recovery checkpoint.
5. Delete the superseded Kotlin stream parser and temporary parity reference, tighten source and symbol audits and ownership documentation, then rerun the complete software, packaging, canonical, and unchanged Apple gates.

Rollback before final deletion is a source revert to the preceding Android stateless-codec cutover. There is no frontend, database, session, stored-data, or user-data migration. After deletion, rollback requires reverting this complete OpenSpec implementation rather than enabling a dormant fallback.

## Open Questions

None. The change intentionally fixes the bridge and ownership boundary narrowly; Rust-owned stream lifetime and all higher-layer migrations remain deferred.
