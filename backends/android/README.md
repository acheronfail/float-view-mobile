# Android backend layout

Android is one native Kotlin Gradle application module. The shared Rust
`application-runtime` owns VESC framing and decoding, discovery, request
scheduling, Refloat and BMS behavior, configuration and mutation policy,
session writes, historical reads/reduction, and pruning. Kotlin reaches it
through one callback-free `application-runtime-android-jni` library.

The JNI boundary is deliberately coarse and versioned. Kotlin submits bounded
JSON commands and facts, drains ordered effects, and copies results into
caller-owned byte arrays. The binding retains no JVM references and exports
only lifecycle, command, result, and interruption operations. It does not expose
the Rust allocator, internal records, or per-protocol entry points.

Production Kotlin lives under
`app/src/main/java/fail/acheron/floatview/`:

| Capability | Package |
| --- | --- |
| Runtime adapter, effect execution, and authoritative state projection | `application/` |
| BLE adapter, GATT policy, and Bluetooth permission presentation | `bluetooth/` |
| Native file chooser, bounded import read, and export share sheet | `files/` |
| Application/process lifetime and foreground background-logging service | `lifecycle/` |
| Location provider and location permission presentation | `location/` |
| Software-fixture VESC peripheral used by emulator conformance | `protocol/vesc/` |
| SQLite catalogue, archive validation, and Android storage policy | `storage/` |
| Native system-bar and WebView-canvas appearance synchronization | `theme/` |
| Authenticated loopback server and port/startup policy | `transport/` |
| Disposable Activity and WebView composition | `ui/` |

Kotlin continues to own Android OS work: Bluetooth and location adapters,
lifecycle, permissions, foreground work, file presentation, the session
catalogue, loopback transport, and UI composition. It does not contain a second
protocol or application state machine. The software peripheral is a private
test fixture, not a production fallback.

The application ID remains `fail.acheron.floatview`; packages express ownership,
not separate applications. Dependencies point down from lifecycle and UI
composition through transport and the application adapter to OS services and
storage. WebViews remain disposable and never own native state.

Addresses, launch tokens, private device identity, raw packets, and full
telemetry or location data stay out of ordinary logs and diagnostics. File and
location data cross the UI boundary only through bounded authenticated reads or
an explicit user import/export action.

Gradle pins NDK `28.2.13676358`. Local JVM tests build the runtime host library
from the locked Rust workspace. Application builds source-build and package
exactly one `libfloat_view_application_runtime.so` for each of `arm64-v8a`,
`armeabi-v7a`, `x86`, and `x86_64` from ignored outputs below
`../core/target/android`. Builds are offline and fail when the pinned Rust
toolchain, targets, NDK, or linkers are absent.

Use `just test android` for the linked JVM suite and
`just android-rust-packaging` for clean Debug/Release four-ABI package
inspection. Add new cross-platform behavior to the safe runtime or one of its
domain dependencies; do not add another JNI bridge or parallel Kotlin decision
owner.
