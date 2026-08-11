# Android backend layout

Android remains one native Kotlin Gradle application module. Package names
mirror the Apple capability names for navigation. Rust owns VESC CRC, frame
encoding, exact frame decoding, and stream-boundary decisions through the
four-function `../core/vesc-frame-android-jni/` bridge. Rust also owns
firmware-identity structure, commands 92–95 custom-configuration envelopes,
and command-96 BMS values through separate focused JNI bridges. Kotlin retains
bounded caller-owned storage, primitive-result validation, error mapping,
observation-time attachment, and the remaining VESC, BMS application, and
Refloat semantics. Apple integration is unchanged; there is no generated API
or Gradle module per capability.
All six JNI crates remain separate. Each depends on its domain crate and the
protocol-neutral `../core/android-jni-support/` crate for call-scoped JVM
exception, array, alias, copy, publication, and panic safety. The support owns
no JNI export or protocol behavior and is the only direct `jni-sys` consumer.
Gradle has one closed typed six-entry integration catalog beside the application
build. Each entry supplies the stable shell-builder identifier plus the existing
task suffix, domain/JNI source roots, generated directories, host library, and
JVM test property. Two generic factories register the twelve Android package
tasks and six host-test tasks, and the same entries drive JNI source sets, test
dependencies, runtime properties, merge dependencies, and clean ordering. This
catalog intentionally remains separate from the shell build catalog: Gradle owns
task integration metadata, while the shell catalog owns Cargo packages,
artifacts, and JNI symbols. Adding a later selected bridge requires one entry in
each relevant catalog rather than another task-registration block.

Production Kotlin lives under
`app/src/main/java/fail/acheron/floatview/`:

| Capability                                                             | Package                               |
| ---------------------------------------------------------------------- | ------------------------------------- |
| Domain, application coordination, and process runtime                  | `application/`                        |
| BLE adapter, GATT policy, and Bluetooth permission presentation        | `bluetooth/`                          |
| Native file chooser, bounded import read, and export share sheet       | `files/`                              |
| Application/process lifetime and foreground background-logging service | `lifecycle/`                          |
| Location provider and location permission presentation                 | `location/`                           |
| VESC and Refloat wire protocol                                         | `protocol/vesc/`, `protocol/refloat/` |
| SQLite, archive validation, and storage policy                         | `storage/`                            |
| Native system-bar and WebView-canvas appearance synchronization        | `theme/`                              |
| Authenticated loopback server and port/startup policy                  | `transport/`                          |
| Disposable Activity and WebView composition                            | `ui/`                                 |

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

Gradle pins NDK `28.2.13676358`. Local JVM tests build the host JNI library from
the locked Rust workspace and load its exact absolute path; application builds
source-build and package `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` from
ignored outputs below `../core/target/android`. These builds are offline and
fail when the pinned Rust toolchain, targets, NDK, or linkers are absent.

Use `just test android` for the complete linked JVM suite and
`just android-rust-packaging` for clean Debug/Release four-ABI package
inspection. The scanner JNI call is stateless and retains no JVM reference or
Rust-owned stream lifetime; broader protocol and application migration remains
separately scoped.
