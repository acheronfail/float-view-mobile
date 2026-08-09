## 1. Freeze the Apple frame-codec ABI

- [x] 1.1 Add `vesc-frame-apple-ffi` as a static-library workspace member that depends only on the safe `vesc-frame` crate and preserves `forbid(unsafe_code)` in the algorithm crate.
- [x] 1.2 Add the hand-written Apple C header with the fixed ABI version, status integers, three-symbol application allowlist, pointer/length contract, non-overlap rule, and 4,096-byte payload and 4,102-byte frame bounds.
- [x] 1.3 Implement version, encode, and exact-decode exports with explicit unsafe blocks, pre-dereference validation, caller-owned outputs, stable error mapping, and panic containment.
- [x] 1.4 Add Rust ABI tests and a compiled C harness covering immutable short/long/maximum vectors, null, overlapping, and undersized buffers, oversized and malformed input, CRC failure, output lengths, status parity, and contained internal failure.

## 2. Integrate locked Apple builds

- [x] 2.1 Add one repository build helper that maps macOS, iOS device, and Intel/Apple-Silicon simulator SDK/architecture/configuration inputs to pinned `cargo build --locked --offline` invocations and fails actionably for missing targets or unsupported inputs.
- [x] 2.2 Add the SwiftPM C system-library target and link the Rust archive into `FloatViewBackend`, then make root macOS build, test, run, and development recipes build and pass the matching archive first.
- [x] 2.3 Add the iOS Xcode build phase, declared inputs/outputs, library search settings, and Debug/Release mapping needed to build and link target-specific Rust archives from source.
- [x] 2.4 Add automated audits for the exact three application ABI symbols, the bridge dependency boundary, ignored build outputs, absence of tracked native archives/generated headers, and actionable ABI-version mismatch.
- [x] 2.5 Prove clean macOS Debug/Release, iOS simulator, and unsigned iOS device compile/link paths before changing production Swift codec ownership.

## 3. Cut over the shared Apple codec

- [x] 3.1 Move `VescProtocolError` and retained `VescFrameStreamDecoder` ownership into clearly named `protocol/vesc` source files without changing their public behavior or bounds.
- [x] 3.2 Replace the shared Swift `VescFrameCodec` internals with a thin ABI-version-checking adapter that allocates bounded caller-owned buffers and maps fixed statuses to established errors; if retained for the checkpoint, move the superseded implementation to a private test-only reference that is not linked into either host and cannot be a production fallback.
- [x] 3.3 Ensure the retained Swift stream decoder performs only bounded boundary accumulation and complete-frame extraction, with every complete frame sent through Rust exact decode.
- [x] 3.4 Extend focused Swift tests to prove the unchanged codec API and stream behavior for short/long/maximum frames, malformed header/length/footer/CRC/trailing bytes, fragmentation, noise, multiple frames, and aggregate overflow through the linked Rust library.
- [x] 3.5 Update Apple and core ownership documentation to identify Rust CRC/encode/exact-decode ownership, Swift stream-boundary ownership, Kotlin Android ownership, and the next-change boundary.

## 4. Verify the selected production path

- [x] 4.1 Run Rust formatting, warnings-denied linting/tests, the C ABI harness, focused Swift codec/stream tests, ABI/dependency/artifact audits, and clean Apple packaging checks.
- [x] 4.2 Run canonical macOS execution and an iOS simulator build/launch and verify that both sibling hosts select the shared linked adapter with no Swift production fallback.
- [x] 4.3 Run `just verify` with the Apple adapter selected and resolve every software, canonical, ownership, and packaging regression.

## 5. Complete the physical deletion checkpoint

- [x] 5.1 Run the representative physical iOS BLE ride with fixture notification fragmentation and mobile background/foreground recovery, proving production CoreBluetooth uses Rust encode/exact-decode while the same session gains samples.
- [x] 5.2 Delete the superseded Swift CRC type, any temporary test-only reference, and all Swift frame construction and exact-decoding algorithms, leaving only the thin adapter and retained stream-boundary state; make the ownership audit reject their return.
- [x] 5.3 Rerun the focused Rust/C/Swift checks, exact symbol and source audits, clean macOS/iOS Debug and Release packaging matrix, and canonical macOS/iOS execution after deletion.
- [x] 5.4 Run the final `just verify` software gate with Android still using Kotlin and no JNI, Rust Android packaging, frontend contract, database, or user-visible behavior change.
