# Portable backend candidates

`backends/core` contains narrowly scoped Rust components being proven before
platform integration. It is a locked Cargo workspace, not another backend and
not a production runtime.

The workspace currently has one member:

| Component     | Scope                                                            | Integration state                                |
| ------------- | ---------------------------------------------------------------- | ------------------------------------------------ |
| `vesc-frame/` | Bounded VESC CRC, frame encoding/decoding, and stream reassembly | Candidate tests only; no Apple or Android bridge |

Apple and Android remain the production owners of VESC framing. A candidate
passes only component-parity checks; it does not count as a supported backend
or production implementation. Separate Apple and Android OpenSpec changes must
add and verify each narrow host bridge. After a host's complete gate and
required physical checkpoint pass, that cutover change deletes the host's
superseded implementation instead of retaining a fallback.

Do not add empty future crates, generated bindings, a general runtime, host
callbacks, mobile build targets, or OS-owned Bluetooth, location, lifecycle,
permission, native-file, theme, or UI behavior here.
