## Why

Apple production framing is still split: Swift owns notification-stream boundary parsing while Rust owns exact frame validation and decoding. The next safe increment should remove that duplicated framing decision logic through the already-proven Apple bridge before broader VESC payload, BMS, Refloat, application, SQLite, or loopback-server ownership moves.

## What Changes

- Make Rust the sole Apple production implementation of VESC stream boundary interpretation, including leading-noise disposal, short/long header parsing, declared-length bounds, incomplete-frame retention decisions, complete-frame extent, ordered multiple-frame consumption, and aggregate overflow behavior.
- Add a safe stateless Rust next-frame scan primitive and make the existing Rust `StreamDecoder` use it so component and Apple production behavior share one algorithm.
- Extend and version the hand-written Apple C ABI with one scanner operation that reports only fixed status and caller-owned primitive lengths; retain Swift-owned bounded byte storage without introducing Rust-owned handles, callbacks, returned allocations, or generated bindings.
- Keep the public Swift `VescFrameStreamDecoder` surface as a thin buffer adapter that delegates every boundary decision to Rust and every complete frame to the existing Rust exact decoder.
- Delete the Swift VESC stream header/length/noise parsing algorithm after focused, packaging, canonical, and representative physical iOS fragmentation/background checkpoints pass, then make repository audits reject its return.
- Correct ownership documentation that still describes the entire Rust framing crate as an unlinked candidate.
- Retain Android's Kotlin CRC, encode, exact-decode, and stream implementation for a later independent Android host change.
- Non-goals: moving VESC payload decoding or routing, firmware identity, BMS, Refloat, Bluetooth, application behavior, SQLite, the local server, the frontend contract, Android NDK/JNI packaging, or Rust-owned stream state across FFI.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Complete Apple framing-algorithm ownership by selecting Rust stream boundary scanning while deferring Rust-owned stream lifetime and Android integration.
- `platform-hosts`: Narrow the retained Swift framing role to caller-owned bounded byte storage beneath the shared iOS/macOS backend.
- `native-development-loop`: Require the versioned scanner ABI and its source-built Apple linkage to remain deterministic across existing build paths.
- `backend-conformance`: Require software, packaging, canonical, and focused physical evidence before deleting Swift stream parsing.

## Impact

This affects the safe `backends/core/vesc-frame` API, the Apple FFI crate and hand-written C header, the shared Swift stream adapter and tests, ABI/source ownership audits, Rust and Apple ownership documentation, and OpenSpec requirements. It changes no frontend-visible API, user behavior, database schema, stored data, third-party dependency, Android build, native OS capability owner, or committed build artifact.
