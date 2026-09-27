# Apple backend layout

The Apple backend is native Swift shared by iOS and macOS. The capability names
below match Android. `../core/application-runtime/` is the production owner of
VESC and Refloat protocol behavior, connection coordination, telemetry,
configuration, mutations, session writing, historical reduction, and pruning.
Swift reaches it through the single bounded, callback-free
`application-runtime-apple-ffi` registry, executes native effects and supplies
complete domain facts to the shared loopback server. Rust projects the
hand-written frontend contract. There is no generated API,
per-protocol Apple bridge, Swift protocol fallback, or duplicated application
decision owner.

Swift retains OS capabilities: Bluetooth transport and permissions, lifecycle
and background work, logging, location acquisition, native session
catalog and archive presentation, and disposable WebViews. Session file queries
and mutations execute in Rust; Swift supplies validated file paths and reconciles
the native catalog after a prune. Runtime interruption is an explicit bounded FFI
operation rather than a callback into Swift.

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

| Capability                            | Shared Apple backend                                  | iOS host       | macOS host                                     |
| ------------------------------------- | ----------------------------------------------------- | -------------- | ---------------------------------------------- |
| Application coordination              | `application/`                                        | `application/` | `application/`                                 |
| Bluetooth and BLE policy              | `bluetooth/`                                          | —              | —                                              |
| Native import/export presentation     | —                                                     | `files/`       | `files/`                                       |
| Process and background lifecycle      | —                                                     | `lifecycle/`   | `lifecycle/`                                   |
| Location provider                     | `location/`                                           | —              | —                                              |
| VESC and Refloat wire protocol        | Rust runtime; `protocol/` contains fixture facts only | —              | —                                              |
| SQLite and archive validation         | `storage/`                                            | —              | —                                              |
| Native appearance synchronization     | —                                                     | `theme/`       | system appearance is consumed by the WebView   |
| Shared transport host adapter         | `transport/`                                          | —              | `transport/` for host readiness only           |
| Disposable WebView/window composition | —                                                     | `ui/`          | `ui/` plus the lifecycle-owned window delegate |

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

For a compatible VESC or Refloat version, extend the exact Rust compatibility
entry and source-derived fixtures.
For a new wire family, add it below the coarse Rust runtime and map exact versions
to it; do not fork application coordination or polling in Swift.

The thin transport adapter supplies HTML, platform launch metadata, native
operations and complete domain facts to `backends/core/loopback-server`. Rust owns
routes, authentication, public projection, historical reads and compression;
FlyingFox and the Swift history payload builders have been removed. Moving an
additional OS capability requires a separate incremental OpenSpec change.
