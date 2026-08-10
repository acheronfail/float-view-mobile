## Why

Apple now proves the bounded Rust VESC framing implementation in production, but Android still duplicates CRC, frame construction, and exact frame validation in Kotlin and has no Rust host boundary. The next increment should establish Android's narrow JNI/NDK delivery path using already-proven stateless behavior before moving stream scanning or any new protocol semantics.

## What Changes

- Make the existing Rust `vesc-frame` component the only Android production implementation of VESC CRC-16, frame encoding, and exact frame decoding.
- Add one small Android-only JNI bridge with a fixed version/status contract, bounded JVM-owned byte arrays, panic containment, and an exact exported-symbol allowlist; no generated bindings, callbacks, or Rust-owned object lifetime cross the boundary.
- Build the bridge from locked source for host JVM unit tests and every Android ABI currently packaged by the application, without committing native binaries or introducing another Gradle module.
- Preserve the public Kotlin `VescFrameCodec` and protocol-error surfaces as thin adapters, while retaining Kotlin-owned stream accumulation, header/length/noise parsing, and recovery for a separate follow-up change.
- Delete the Kotlin CRC, encoder, and exact-decoder algorithms only after focused Rust/JNI/Kotlin tests, clean Android Debug/Release packaging, canonical Android execution, a representative physical Android fragmented-background BLE checkpoint, and `just verify` pass through the no-fallback production path.
- Keep the complete Apple framing path unchanged and verified.
- Non-goals: moving Android stream scanning or Rust-owned stream state; changing Apple ABI or packaging; moving firmware identity, VESC routing, BMS, Refloat, Bluetooth, application behavior, SQLite, the loopback server, frontend behavior, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the stateless Android frame-codec ownership transfer while Kotlin stream parsing remains explicitly retained.
- `platform-hosts`: Record the narrow Android JNI bridge, Kotlin adapter boundary, and unchanged native OS-capability ownership.
- `native-development-loop`: Require deterministic source-built host-JVM and Android-ABI bridge artifacts across Gradle build, test, run, and verification paths.
- `backend-conformance`: Require software, packaging, canonical, and focused physical evidence before deleting Android's superseded Kotlin frame-codec algorithms.

## Impact

This affects the `backends/core` workspace, one new Android JNI bridge crate and build helper, Android Gradle native-library packaging, the Kotlin VESC frame codec and tests, root `justfile` build/audit recipes, ownership documentation, and OpenSpec requirements. A minimal locked JNI ABI binding dependency may be added only to the Android bridge after dependency audit; the safe framing crate remains dependency-free. There is no frontend-contract, database, stored-data, Apple runtime, user-visible, Bluetooth, location, lifecycle, file-presentation, theme, or permission behavior change. Superseded Kotlin CRC/encode/exact-decode code is removed at the verified deletion checkpoint; Kotlin stream parsing remains until the next OpenSpec change.
