# Initial target smoke record

Task 1.2 was exercised on 2026-08-07 with Rust 1.93.0 and the locked
workspace. The commands below are reproducible checks, not committed build
artifacts.

## macOS and Apple targets

Native `cargo test --workspace --locked` passed on arm64 macOS and the
`float-view-core-smoke` executable printed `abi=2 build=0.1.0`.

The complete workspace, including the static ABI library and smoke executable,
built for:

- `aarch64-apple-ios`
- `aarch64-apple-ios-sim`
- `x86_64-apple-ios`

The hand-written Swift wrapper links the target-specific static library
directly. Its macOS test exercised two full runtime/pump lifecycles off the main
thread with presentation replacement. The same wrapper compiled and linked as
an arm64 iOS-simulator Mach-O product (platform 7, minimum iOS 17.0). This
confirms the selected direct-link strategy; no XCFramework or binary artifact
is committed.

## Android targets

With NDK 28.2.13676358 and cargo-ndk 4.1.2, the JNI `cdylib` built for:

- `arm64-v8a` (`aarch64-linux-android`)
- `armeabi-v7a` (`armv7-linux-androideabi`)
- `x86` (`i686-linux-android`)
- `x86_64` (`x86_64-linux-android`)

The arm64 `float-view-core-smoke` executable was copied to and executed on an
API-29-or-newer physical Android device and printed `abi=2 build=0.1.0`.

The Gradle-owned JNI spike packages exactly one
`libfloat_view_android_jni.so` for each required ABI. Its explicitly registered
native facade and Kotlin command worker passed the focused instrumentation test
on a physical arm64 Android 10 device, including copied input/output buffers,
off-main-thread blocking retrieval, presentation replacement, matching
completion, stop wakeup, and handle destruction.

The locked Tokio/Axum loopback fixture configures explicit header, body,
message, connection, WebSocket-client, write-buffer, and idle/read bounds. Its
macOS tests cover exact authentication, fragmented and oversized WebSocket
traffic, header rejection, shutdown, and restart. The standalone production
bind/bootstrap/shutdown executable ran on macOS and the physical Android
device, and the same source builds for iOS device/simulator and all Android
ABIs.

The Cargo-bundled SQLite fixture uses SQLite 3.50.2 from one locked
amalgamation. It passed canonical schema creation, WAL write/read,
interruption, checkpoint, close, and reopen on macOS and the physical Android
device; it builds for iOS device/simulator and all four Android ABIs. Exact
identity and compile options are recorded in `SQLITE.md`.

The locked dependency fixture passed SHA-256, secure-random, gzip,
zlib-compatible decoding, XML, CRC32C, and bounded archive checks on macOS and
the physical Android device. The same executable builds for iOS
device/simulator and all four Android ABIs. Direct dependency identities,
responsibilities, and licenses are recorded in `DEPENDENCIES.md`.

The root matrix entry points are `just core-matrix macos|ios|android`.
The macOS job runs the complete Cargo workspace, pure-C ABI harness, and
dependency, bundled-storage, and loopback smoke executables. The iOS job builds
the same workspace for the arm64 simulator, links and launches the Rust-backed
host, and runs canonical conformance through that production composition. The
Android job builds and verifies all four packaged JNI ABIs, runs the focused
instrumentation suite on the selected emulator/device, and then runs canonical
conformance. `just verify` remains the aggregate software gate; physical BLE
acceptance remains a separate explicit checkpoint.

## Physical cutover record

On 2026-08-08 the Rust-backed production adapters passed all 16 macOS physical
BLE scenarios and all 17 iOS scenarios, including reconnect, BMS, riding, and
background location. Android passed the full BLE scenario set after retrying a
failed reconnect from the Rust `Reconnecting` stage. Its Android 10 test phone
reported location authorization and background capability but produced no GPS
observation indoors (`rows=0->0`, `sequence=0->0`). The maintainer explicitly
accepted this as a test-device signal limitation and accepted the Android
background checkpoint on the exercised permission, lifecycle, BLE, and storage
behavior.

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export NDK_HOME="$ANDROID_NDK_HOME"

cd backends/core
cargo ndk --platform 29 \
  --target arm64-v8a --target armeabi-v7a --target x86 --target x86_64 \
  build --package float-view-android-jni --locked
cargo ndk --platform 29 --target arm64-v8a \
  run --package float-view-core-smoke --locked
```
