# Bounded portable backend components

`backends/core` contains narrowly scoped Rust components with explicit host
ownership. It is a locked Cargo workspace, not another backend and not a
production runtime.

The workspace currently has twelve members:

| Component               | Scope                                                                 | Integration state                                                                             |
| ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `vesc-bms-values/`       | Safe bounded VESC command-96 BMS values decoding                     | Selected by Apple and Android through separate host bridges                                    |
| `vesc-bms-values-android-jni/` | Two-symbol primitive-array JNI ABI for BMS values              | Selected only by Android                                                                       |
| `vesc-bms-values-apple-ffi/` | Two-symbol caller-owned-buffer C ABI for BMS values              | Selected only by the shared iOS/macOS adapter                                                  |
| `vesc-custom-configuration/` | Safe bounded VESC commands 92–95 envelope encoding and decoding  | Selected by Apple and Android through separate host bridges                                   |
| `vesc-custom-configuration-apple-ffi/` | Three-symbol caller-owned-buffer C ABI for commands 92–95 | Selected only by the shared iOS/macOS adapter                                                  |
| `vesc-custom-configuration-android-jni/` | Three-symbol primitive-array JNI ABI for commands 92–95 | Selected only by Android                                                                       |
| `vesc-firmware-identity/` | Safe bounded `COMM_FW_VERSION` payload decoding                      | Selected by Apple and Android through separate host bridges                                   |
| `vesc-firmware-identity-apple-ffi/` | Two-symbol caller-owned-buffer C ABI for firmware identity | Selected only by the shared iOS/macOS identity adapter                                        |
| `vesc-firmware-identity-android-jni/` | Two-symbol primitive-array JNI ABI for firmware identity | Selected only by Android                                                                      |
| `vesc-frame/`             | Safe bounded VESC CRC, frame encoding/decoding, and stream scanning | Apple and Android select every framing decision                                 |
| `vesc-frame-apple-ffi/`   | Caller-owned-buffer C ABI                                             | Selected only by the shared iOS/macOS frame-codec adapter                       |
| `vesc-frame-android-jni/` | Primitive-array JNI ABI                                              | Selected by Android for CRC, encoding, exact decoding, and stream-boundary scans |

The twelve crates remain independent components and artifacts. Their repeated
source-build mechanics are centralized in `build-apple-bridge.sh` and
`build-android-bridge.sh`, driven by the closed four-entry
`bridge-build-catalog.sh` allowlist. Callers pass only `frame`,
`firmware-identity`, `custom-configuration`, or `bms-values`; package names,
archive/library names, diagnostics, and exact JNI symbols come from the
catalog. The catalog is deliberately non-executable, while the two builders
retain the pinned offline toolchain, Apple target, Android NDK/API/ABI,
architecture, and symbol checks.

The shared Apple backend retains only bounded Swift stream storage. Rust scans
each borrowed region, reports exact consumption boundaries, validates every
complete frame, and owns outbound encoding without transferring allocator or
object lifetime across the bridge. Android uses the same safe crate through a
separate four-function JNI bridge. Rust selects Android stream boundaries;
Kotlin retains only caller-owned bounded byte storage, mechanical consumption,
and error mapping without transferring allocator or object lifetime.

The firmware identity component has an empty production dependency graph. Its
tests use only the existing safe framing crate to unwrap committed framed
fixtures. Separate Apple and Android bridges depend only on it and their
minimal host ABI support; neither bridge exposes Rust allocation or layout.

The custom-configuration component encodes and decodes the bounded VESC
commands 92–95 envelope while treating compressed schemas and serialized
values as opaque caller-owned bytes. Separate Apple and Android bridges expose
only fixed metadata and caller-owned regions; schema and typed-value semantics
remain native.

The BMS values component decodes only the bounded command-96 payload into a
platform-neutral fixed-capacity model. Apple and Android select it through
separate caller-owned-record bridges. BMS compatibility, discovery, routing,
scheduling, polling, observation time, application state, and live-only policy
remain native.

Do not add empty future crates, generated bindings, a general runtime, host
callbacks, or OS-owned Bluetooth, location, lifecycle, permission, native-file,
theme, or UI behavior here. Unsafe code belongs only in a named audited bridge;
the safe algorithm crate continues to forbid it.
