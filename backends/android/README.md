# Android backend layout

Android remains one native Kotlin Gradle application module. Package names
mirror the Apple capability names for navigation. Kotlin remains the
production protocol owner; `../core/vesc-frame/` is a component-test-only Rust
candidate with no Android bridge, JNI layer, generated API, native library, or
Gradle module per capability.

Production Kotlin lives under
`app/src/main/java/fail/acheron/floatview/`:

| Capability | Package |
| --- | --- |
| Domain, application coordination, and process runtime | `application/` |
| BLE adapter, GATT policy, and Bluetooth permission presentation | `bluetooth/` |
| Native file chooser, bounded import read, and export share sheet | `files/` |
| Application/process lifetime and foreground background-logging service | `lifecycle/` |
| Location provider and location permission presentation | `location/` |
| VESC and Refloat wire protocol | `protocol/vesc/`, `protocol/refloat/` |
| SQLite, archive validation, and storage policy | `storage/` |
| Native system-bar and WebView-canvas appearance synchronization | `theme/` |
| Authenticated loopback server and port/startup policy | `transport/` |
| Disposable Activity and WebView composition | `ui/` |

The application ID remains `fail.acheron.floatview`; the deeper packages are
implementation ownership, not separate applications. Tests mirror the owning
production package when Kotlin package visibility matters.

Dependencies point down from lifecycle/UI composition through transport and
application coordination to Bluetooth, location, storage, and protocol code.
Bluetooth and protocol components emit typed events and do not know about
WebViews, HTTP, Activities, or native presentation. The lifecycle foreground
service keeps the process-level BLE/logger owner eligible without becoming a
second application-service owner.

Addresses, launch tokens, private device identity, raw packets, and full
telemetry or location data stay out of ordinary logs and diagnostics. File and
location data cross the UI boundary only through bounded authenticated reads or
an explicit user import/export action.

For a compatible VESC or Refloat version, extend the exact compatibility entry
and source-derived fixtures in both native backends. For a new wire family,
change or add the focused codec below `protocol/vesc` or `protocol/refloat` and
map exact versions to it; do not fork application coordination or polling.

An Android cutover requires its own OpenSpec change, audited bridge and
packaging, complete Android gate, and required physical checkpoint. Only that
verified cutover may delete the corresponding Kotlin implementation; candidate
parity alone does not transfer ownership.
