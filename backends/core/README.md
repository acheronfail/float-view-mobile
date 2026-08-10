# Bounded portable backend components

`backends/core` contains narrowly scoped Rust components with explicit host
ownership. It is a locked Cargo workspace, not another backend and not a
production runtime.

The workspace currently has three members:

| Component               | Scope                                                                 | Integration state                                                                             |
| ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `vesc-frame/`               | Safe bounded VESC CRC, frame encoding/decoding, and stream scanning | Apple selects every framing decision; Android selects stateless framing only                |
| `vesc-frame-apple-ffi/`     | Caller-owned-buffer C ABI and locked Apple source-build helper      | Selected only by the shared iOS/macOS frame-codec adapter                                   |
| `vesc-frame-android-jni/`   | Primitive-array JNI ABI and locked host/Android source builds       | Selected by Android for CRC, frame encoding, and exact decoding; Kotlin retains stream state |

The shared Apple backend retains only bounded Swift stream storage. Rust scans
each borrowed region, reports exact consumption boundaries, validates every
complete frame, and owns outbound encoding without transferring allocator or
object lifetime across the bridge. Android uses the same safe crate through a
separate three-function JNI bridge for stateless framing. Kotlin continues to
own stream accumulation, boundary parsing, and recovery; moving that ownership
is the next independent OpenSpec change.

Do not add empty future crates, generated bindings, a general runtime, host
callbacks, or OS-owned Bluetooth, location, lifecycle, permission, native-file,
theme, or UI behavior here. Unsafe code belongs only in a named audited bridge;
the safe algorithm crate continues to forbid it.
