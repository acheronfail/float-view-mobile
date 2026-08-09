# Apple backend layout

The Apple backend is native Swift shared by iOS and macOS. The capability
names below match Android. Swift remains the production protocol owner;
`../core/vesc-frame/` is a component-test-only Rust candidate with no Apple
bridge, FFI layer, generated API, or packaged library.

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

| Capability | Shared Apple backend | iOS host | macOS host |
| --- | --- | --- | --- |
| Application coordination | `application/` | `application/` | `application/` |
| Bluetooth and BLE policy | `bluetooth/` | — | — |
| Native import/export presentation | — | `files/` | `files/` |
| Process and background lifecycle | — | `lifecycle/` | `lifecycle/` |
| Location provider | `location/` | — | — |
| VESC and Refloat wire protocol | `protocol/vesc/`, `protocol/refloat/` | — | — |
| SQLite and archive validation | `storage/` | — | — |
| Native appearance synchronization | — | `theme/` | system appearance is consumed by the WebView |
| Authenticated loopback transport | `transport/` | — | `transport/` for host readiness only |
| Disposable WebView/window composition | — | `ui/` | `ui/` plus the lifecycle-owned window delegate |

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

An Apple cutover requires its own OpenSpec change, audited bridge and
packaging, complete Apple gate, and required physical checkpoint. Only that
verified cutover may delete the corresponding Swift implementation; candidate
parity alone does not transfer ownership.
