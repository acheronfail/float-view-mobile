# Apple backend layout

The Apple backend is native Swift shared by iOS and macOS. The capability names
below match Android. Rust under `../core/vesc-frame/` is the production owner
for VESC CRC, outbound framing, exact frame decoding, and notification-stream
boundary interpretation through the bounded `vesc-frame-apple-ffi/` bridge.
Swift retains only bounded caller-owned stream bytes and mechanical consumption
of Rust-returned regions alongside VESC payload routing, remaining Refloat
behavior, application behavior, and every OS-owned capability. Rust also owns
command-10 legacy and command-31/32 dynamic Refloat telemetry decoding and
their canonical records through temporary focused boundaries. There is no
generated API, Rust stream handle, or Swift production framing fallback.
All eight focused Rust bridge crates remain separate. SwiftPM recipes and the
eight Xcode phases select them through the shared allowlisted Apple builder in
`../core/`, so bridge-specific ABI code is not coupled to duplicated build
scripts. Their protocol-neutral pointer validation, panic containment, and
bounded output publication come from `../core/apple-ffi-support/`; exported
symbols, records, headers, archives, domain mappings, and Swift call sites
remain explicit per bridge, without generated bindings or export macros.
Firmware-identity record layout and serialization are the exception to record
ownership: `../core/vesc-firmware-identity/` owns the one platform-neutral
version-1 record consumed by both host families. Its Apple bridge retains only
the independent two-symbol C boundary, pointer-region validation, fixed status
mapping, and caller-owned publication; the header and Swift model mapping stay
explicit here.

The same canonical-record ownership applies to legacy and dynamic Refloat
telemetry. The dynamic adapter retains Rust's validated layout record only for
the current connection; both adapters perform only strict record validation
and domain mapping. Their bridges are migration scaffolding planned for
deletion with the unified Rust application runtime.

SwiftPM and Xcode use one shared implementation root and two sibling platform
host roots:

- `Sources/FloatViewBackend/` — backend behavior shared by iOS and macOS
- `ios/float-view/` — iOS application host and disposable WebView
- `macos/` — macOS application host and disposable WebView

The host roots are deliberately the siblings `ios/` and `macos/`. SwiftPM
builds the `FloatViewMac` executable from the explicit `macos/` target path;
platform-specific code must not be reintroduced under `Sources/`.

Each root uses the same capability vocabulary for the responsibilities it
owns:

| Capability                            | Shared Apple backend                                                            | iOS host       | macOS host                                     |
| ------------------------------------- | ------------------------------------------------------------------------------- | -------------- | ---------------------------------------------- |
| Application coordination              | `application/`                                                                  | `application/` | `application/`                                 |
| Bluetooth and BLE policy              | `bluetooth/`                                                                    | —              | —                                              |
| Native import/export presentation     | —                                                                               | `files/`       | `files/`                                       |
| Process and background lifecycle      | —                                                                               | `lifecycle/`   | `lifecycle/`                                   |
| Location provider                     | `location/`                                                                     | —              | —                                              |
| VESC and Refloat wire protocol        | `protocol/vesc/`, `protocol/refloat/`; exact VESC frames delegate to `../core/` | —              | —                                              |
| SQLite and archive validation         | `storage/`                                                                      | —              | —                                              |
| Native appearance synchronization     | —                                                                               | `theme/`       | system appearance is consumed by the WebView   |
| Authenticated loopback transport      | `transport/`                                                                    | —              | `transport/` for host readiness only           |
| Disposable WebView/window composition | —                                                                               | `ui/`          | `ui/` plus the lifecycle-owned window delegate |

Session archives are validated and imported by `storage/SQLiteLogStore.swift`.
Explicit exports are presented by the platform owners under `files/`. Native
session selection uses the standard WebKit file-input presentation; any custom
document-picker or open-panel integration belongs under `files/`, not in the
application service or storage validator.

Dependencies point down from lifecycle/UI composition through loopback and
application coordination to Bluetooth, location, storage, and protocol code.
Bluetooth and protocol components emit typed events and do not know about
WebViews, HTTP, or platform presentation.

Addresses, launch tokens, private device identity, raw packets, and full
telemetry or location data stay out of ordinary logs and diagnostics. File and
location data cross the UI boundary only through bounded authenticated reads or
an explicit user import/export action.

For a compatible VESC or Refloat version, extend the exact compatibility entry
and source-derived fixtures in both native backends. For a new wire family,
change or add the focused codec below `protocol/vesc` or `protocol/refloat` and
map exact versions to it; do not fork application coordination or polling.

Moving Swift-owned stream storage, VESC payload behavior, Refloat, application
behavior, SQLite, or the loopback server requires a separate incremental
OpenSpec change. Android integration is also independent of this shared Apple
cutover.
