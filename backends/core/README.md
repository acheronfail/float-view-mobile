# Bounded portable backend components

`backends/core` contains narrowly scoped Rust components with explicit host
ownership. It is a locked Cargo workspace, not another backend and not a
production runtime.

The workspace currently has two members:

| Component               | Scope                                                                 | Integration state                                                                             |
| ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `vesc-frame/`           | Safe bounded VESC CRC, frame encoding/decoding, and stream scanning   | Apple selects every framing decision; stream storage and Android integration remain deferred  |
| `vesc-frame-apple-ffi/` | Caller-owned-buffer C ABI and locked Apple source-build helper        | Selected only by the shared iOS/macOS frame-codec adapter                                     |

The shared Apple backend retains only bounded Swift stream storage. Rust scans
each borrowed region, reports exact consumption boundaries, validates every
complete frame, and owns outbound encoding without transferring allocator or
object lifetime across the bridge. Android remains the independent Kotlin
production owner. Any later stream-storage or Android cutover requires its own
OpenSpec change and complete gate.

Do not add empty future crates, generated bindings, a general runtime, host
callbacks, or OS-owned Bluetooth, location, lifecycle, permission, native-file,
theme, or UI behavior here. Unsafe code belongs only in a named audited bridge;
the safe algorithm crate continues to forbid it.
