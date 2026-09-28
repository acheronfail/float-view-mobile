# Android host

Android is one Kotlin Gradle module. `SharedService` creates the process-owned
Rust service and executes a closed, bounded OS fact/effect exchange through the
existing `application-runtime-android-jni` family. The binding retains no JVM
references or callbacks. Listener handles borrow this service; Activities and
WebViews are disposable.

Kotlin owns BLE/GATT, permissions, location acquisition, the foreground service,
scoped file access, theme and UI integration. All product CRUD, identity,
reconnect, recording, configuration, archives, public dispatch and projection
live in Rust. There is no native SQL catalog or domain-operation bridge.

| Package          | Responsibility                                                |
| ---------------- | ------------------------------------------------------------- |
| `application/`   | Process composition, bounded OS executor and JNI declarations |
| `bluetooth/`     | BLE/GATT, permission presentation and test adapter            |
| `files/`         | Scoped URI import stream and export destination picker/copy   |
| `lifecycle/`     | Application lifetime and foreground recording service         |
| `location/`      | Location acquisition and permission broker                    |
| `protocol/vesc/` | Emulator conformance fixture only                             |
| `theme/`         | System bars and WebView appearance                            |
| `transport/`     | Listener lifetime and authenticated native URI upload         |
| `ui/`            | Disposable Activity and WebView                               |

The URI importer streams at most 128 MiB through 64-KiB buffers into the same
shared HTTP importer used by the frontend. Exports retain their service lease
until the scoped destination stream closes. Native file callbacks have bounded
admission; no entire archive is materialized in Kotlin memory.

Gradle pins NDK `28.2.13676358` and minimum API 29. Builds package one
`libfloat_view_application_runtime_android_jni.so` for each of `arm64-v8a`,
`armeabi-v7a`, `x86`, and `x86_64`. `just test android` runs linked JVM checks;
`just android-rust-packaging` verifies clean Debug/Release packages and exact
JNI exports. The physical checkpoint covers BLE, background work, Activity
recreation and file presentation on a real Android device.
