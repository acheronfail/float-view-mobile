## Context

The prior `extract-rust-vesc-framing` change established a safe, dependency-free Rust candidate for CRC-16, exact frame encoding/decoding, and stream reassembly. Swift still owns all Apple production framing, Kotlin still owns all Android production framing, and the Rust crate is reached only by component tests. The remaining uncertainty for the smallest useful cutover is therefore the Apple build and language boundary rather than framing semantics.

Apple is the best first host because iOS and macOS already share one `FloatViewBackend` implementation. A single adapter can place the same selected Rust code beneath both hosts while leaving CoreBluetooth, mobile background execution, location, files, theme, lifecycle, the loopback server, and the frontend contract untouched. The stateless operations are the safest first boundary: Swift stream reassembly can continue to identify complete frames, then call Rust for exact validation and payload extraction.

This change is intentionally not a general native core. It introduces one bridge for one already-proven component and deletes only the corresponding Apple algorithms after the complete gate passes.

## Goals / Non-Goals

**Goals:**

- Make Rust the sole Apple production implementation of VESC CRC-16, frame encoding, and exact frame decoding on both iOS and macOS.
- Preserve the current `VescFrameCodec` Swift call surface, error cases, payload/frame bounds, and stream behavior.
- Prove a small auditable C ABI and deterministic source build for macOS, iOS device, and iOS simulator architectures.
- Keep unsafe code and platform packaging outside the safe `vesc-frame` crate.
- Delete the superseded Swift CRC/encode/exact-decode algorithms and prohibit a runtime fallback.
- Produce enough software, packaging, canonical, and physical evidence to make the ownership transfer unambiguous.

**Non-Goals:**

- Moving `VescFrameStreamDecoder` state or the Rust `StreamDecoder` across FFI.
- Adding JNI, Android NDK configuration, or changing Kotlin framing ownership.
- Moving VESC payload decoding/routing, VBMS, Refloat, Bluetooth, application behavior, SQLite, the local server, or frontend behavior.
- Introducing callbacks, an opaque runtime handle, Rust-owned buffers returned to Swift, generated bindings, `cbindgen`, committed native archives, an XCFramework, or a third-party Rust dependency.
- Creating a generic cross-language API layer for later migration work.

## Decisions

### 1. Cut over one stateless vertical slice

The production slice consists only of `encode(payload)` and `decode_exact(frame)`. CRC remains an internal implementation detail of those Rust operations; it is not exported as an additional ABI function. The existing Swift stream decoder continues to own its bounded buffer, leading-noise removal, fragmentation, and multiple-frame loop. Once it has isolated one complete candidate frame, it calls the Rust exact decoder, so CRC, footer, exact length, and payload extraction are decided by Rust.

The public Swift `VescFrameCodec` name, maximum payload constant, function signatures, and `VescProtocolError` results remain stable. `VescProtocolError` moves to its own protocol source file because payload and higher-protocol decoders also use it. `VescFrameStreamDecoder` may move to its own source file for clear retained ownership. During the checkpoint, the superseded algorithms may exist only as a private test-target reference that is not linked into either application host; production has no feature flag or fallback path. That temporary reference, the old public `VescCRC` type, and every Swift encode/decode body are deleted after the physical gate.

This boundary proves a real production call without making the first host bridge stateful. Moving stream state can be considered in a later OpenSpec change after this boundary is stable.

### 2. Isolate the ABI in a second Rust crate

Add `backends/core/vesc-frame-apple-ffi` as a workspace member with static-library output. It depends only on the existing safe `vesc-frame` crate and the standard library. `vesc-frame` retains `forbid(unsafe_code)` and does not gain platform or ABI concepts. The bridge crate contains the minimum pointer conversion and exported functions, denies warnings and unsafe operations outside explicit unsafe blocks, and contains no framing algorithm.

Every exported operation catches unwinding at the ABI boundary and converts it to a fixed failure status. No panic, Rust reference, slice, `Vec`, string, allocator ownership, or Rust layout crosses the boundary. A symbol audit permits exactly the documented Float View ABI entry points; normal toolchain/runtime symbols in the archive are not treated as application ABI.

### 3. Use a hand-written, versioned, caller-owned C contract

The hand-written header and Rust implementation define exactly these application symbols:

- `float_view_vesc_frame_abi_version`
- `float_view_vesc_frame_encode`
- `float_view_vesc_frame_decode_exact`

Encode and decode accept byte pointers plus lengths, a caller-owned output pointer plus capacity, and an output-length pointer. A null byte pointer is accepted only for a zero-length input or zero-capacity output. Input and output regions must not overlap. The maximum payload is 4,096 bytes and the maximum encoded frame is 4,102 bytes. The functions validate bounds before dereferencing or copying, never write beyond the supplied capacity, and report the required/produced length only according to the documented contract.

The ABI uses explicit integer status constants for success, invalid argument, input too large, invalid frame, invalid CRC, output too small, and contained internal failure. The ABI version is checked by the Swift adapter before use. Semantic statuses map to the existing `payloadTooLarge`, `invalidFrame`, and `invalidCRC` cases. Impossible adapter/ABI failures fail closed as `invalidFrame` with a bounded byte-free diagnostic, preserving the frontend-visible behavior and avoiding telemetry or frame logging.

A C compilation/runtime harness validates that the header is usable independently of Swift, that status values and ABI version match, that undersized and null buffers fail safely, and that the same immutable vectors cross the compiled boundary.

### 4. Build target-specific archives from locked source

The repository builds the static archive with the pinned toolchain, `--locked`, and `--offline`; it does not commit `.a` files. The supported matrix is the host macOS target, `aarch64-apple-ios` for devices, and the active `aarch64-apple-ios-sim` or `x86_64-apple-ios` simulator architecture. Missing toolchains or targets fail with an actionable diagnostic rather than downloading during a build.

`Package.swift` gains a small C system-library target for the hand-written header and links it into `FloatViewBackend`. Root `just` recipes build the matching Rust archive before macOS Swift build, test, run, or development commands and pass the target-specific library search path. The iOS Xcode project gains a declared build phase that invokes the same repository build helper for the active SDK, architecture set, and Debug/Release configuration, then links the produced archive from derived build output. The helper owns target/configuration mapping so SwiftPM, Xcode, and verification do not grow independent shell logic.

Repository verification builds from a clean artifact state for macOS, iOS simulator, and an unsigned iOS device build. It also performs a release-mode archive/link smoke check. Build outputs remain ignored and recoverable; no generated source or header is required.

### 5. Make production ownership observable through deletion and gates

Focused Rust tests retain the full framing parity vectors and add ABI status/buffer/panic-containment checks. Swift tests exercise the unchanged codec API and retained stream decoder through the linked Rust library, including short/long boundaries, malformed frames, fragmentation, leading noise, multiple frames, CRC failures, and aggregate bounds. A repository audit fails if the deleted Swift CRC polynomial/loop or independent frame construction/exact-decoding logic reappears outside tests, or if the application ABI symbol set expands.

`just verify` remains the software gate and includes the Rust workspace, C harness, Swift tests, clean Apple packaging matrix, and existing canonical software conformance. Canonical macOS execution and an iOS simulator build/launch prove both host compositions select the shared adapter. Before deletion is finalized, physical iOS conformance must run production CoreBluetooth traffic with fragmented fixture notifications through the no-fallback Rust path, including a ride/background-recovery scenario already required by the canonical matrix. After that checkpoint, the Swift algorithms are removed and the complete software gate is rerun. Successful operation after source deletion is the strongest evidence that no Swift implementation remains selected.

## Risks / Trade-offs

- **Apple build integration can drift across SwiftPM and Xcode.** A single target/configuration helper and clean matrix checks reduce drift; the root `just` entry points remain authoritative.
- **A C ABI introduces unsafe pointer handling.** The unsafe surface is isolated to a tiny bridge crate, all lengths are bounded before access, ownership never crosses, and C/Rust negative tests cover null, capacity, overlap-contract, malformed, and panic paths.
- **Keeping stream reassembly in Swift leaves temporary split ownership.** The split is explicit and narrow: Swift owns boundary accumulation, Rust owns exact frames. This avoids stateful FFI in the first production bridge and is intended to be reassessed in a later change.
- **Cross-compilation prerequisites may not exist on a new workstation.** Builds fail with the exact missing pinned target and setup instruction; they do not silently fetch tools or use stale committed binaries.
- **A physical checkpoint can delay completion.** The change remains incomplete and the Swift algorithms remain available for rollback until the signed iOS device and fixture checkpoint succeeds.

## Migration Plan

1. Add the bridge crate, header, build helper, and focused Rust/C tests while Swift production still uses its existing implementation.
2. Add the C target and target-specific Apple build/link integration; prove clean macOS, iOS simulator, and unsigned device builds.
3. Replace `VescFrameCodec` internals with the Rust adapter, split retained error/stream ownership into clear files, quarantine any superseded algorithm only as an unlinked private test reference, and run focused plus canonical software gates with no production fallback.
4. Run canonical macOS execution and the required physical iOS BLE fragmentation/background checkpoint through the selected production path.
5. Delete `VescCRC`, the temporary test reference, and every Swift encode/exact-decode algorithm, add the source/symbol ownership audit, and rerun `just verify` and Apple packaging checks.

Rollback before step 5 is a normal source revert to the Swift codec. No storage, contract, or data migration occurs. After step 5, rollback requires reverting this whole change rather than activating a dormant fallback.

## Open Questions

None. Stateful stream ownership and Android integration are deliberately deferred to separate future OpenSpec changes.
