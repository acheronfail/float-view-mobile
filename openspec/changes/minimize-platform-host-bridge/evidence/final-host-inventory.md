# Final host inventory and verification

Recorded after implementation on 2026-08-08. `just host-audit` excludes generated sources, dependencies, and build output and reports production and test code separately.

| Host | Baseline production | Final production | Removed | Final tests |
| --- | ---: | ---: | ---: | ---: |
| Swift | 3,703 | 2,844 | 859 (23.2%) | 704 |
| Kotlin | 3,368 | 2,697 | 671 (19.9%) | 636 |

The production hosts retain 18 Swift and 24 Kotlin source files. The audit rejects native software peripherals, reverse bridge codecs, random transport aliases, unused saved-device APIs, deleted command models, and any retained command missing from the 11-kind reachability manifest.

## Apple ownership

| Remaining files | Why they remain native |
| --- | --- |
| `CoreBluetoothAdapter.swift`, `VescBluetoothPolicy.swift` | CoreBluetooth discovery/GATT callbacks, notification setup, negotiated writes, backpressure, and stale callback rejection. |
| `CoreLocationAdapter.swift`, location types in `PlatformPorts.swift` | Core Location authorization and observation delivery. |
| `FloatViewCoreRuntime.swift`, `FloatViewPlatformBridgeCodec.swift`, `RustCorePlatformAdapter.swift`, bridge types in `PlatformPorts.swift` | Caller-owned C buffers, the single off-main pull worker, decoding of the 11 Rust commands, focused event submission, and presentation replacement. No reverse production codec remains. |
| `ApplePlatformHostServices.swift`, `AppleSystemPresentation.swift` | Bundle/document URLs and native permission/settings presentation. Portable storage, randomness, clocks, and resource loading are no longer command round trips. |
| `FloatViewMac.swift`, `FloatViewApp.swift`, `RustContentView.swift`, `WebView.swift` | Process/app lifecycle, startup configuration, WebKit ownership, and disposable frontend presentation. The macOS conformance root is explicitly supplied to Rust and never aliases user data. |

## Android ownership

| Remaining files | Why they remain native |
| --- | --- |
| `AndroidBluetoothAdapter.kt`, `AndroidBluetoothPermissionPolicy.kt`, `VescBluetoothPolicy.kt`, `VescGattProfile.kt` | Android permission/radio/GATT APIs, notification descriptor setup, negotiated writes, bounded queues, and stale callback rejection. |
| `AndroidLocationProvider.kt` and the location portion of `backend/Domain.kt` | Android location authorization and observation delivery. |
| `NativeCore.kt`, `CoreBridgeWorker.kt`, `PlatformBridgeCodec.kt`, `RustBluetoothPlatformAdapter.kt`, `RustAndroidPlatformAdapter.kt`, `RustBackendRuntime.kt` and Bluetooth types in `backend/Domain.kt` | Explicit JNI registration, copied arrays, the single blocking non-main pull worker, decoding of the 11 Rust commands, focused event submission, and the one process runtime. No reverse production codec remains. |
| `AndroidPlatformHostServices.kt`, `AndroidPresentationBroker.kt`, `PermissionBrokers.kt` | Content-URI/document and permission/settings presentation owned by Android. |
| `FloatViewApplication.kt`, `MainActivity.kt`, `LoggingForegroundService.kt` | Process/Activity/WebView lifecycle, permission result routing, and Android foreground-service ownership. |

The bridge/runtime files are larger than simple OS adapters because they contain the private memory-safety boundary: fixed-width ABI identity checks, bounded copying, wait-buffer growth without dequeue, generation checks, shutdown wake, and one-way command dispatch. Those responsibilities cannot move below C/JNI without recreating callbacks or sharing allocators.

## Final bridge and gate evidence

- `backends/core/HOST_COMMANDS.tsv` accounts for all 11 retained command kinds, their Rust requester, host dispatcher, completion bound, and conformance owner.
- The C header is generated from ABI 3 and checked by checksum; the pure-C harness covers focused events, invalid values, sizes/alignment, resize-without-dequeue, stop/wake, stale handles, and panic containment.
- Rust orders Bluetooth, location, lifecycle, logging, and theme callback channels independently; stale order is rejected within each channel and connection generation remains Bluetooth-owned.
- The Rust software host completes writes before delivering notifications, rejects prior VESC-generation responses, and shares the production actor/server/storage/shutdown graph.
- `just verify` passed Rust workspace tests, frontend checks/tests, canonical mock/macOS/iOS/Android conformance, Swift tests/builds, Android JVM tests/build, iOS simulator build, locked inputs, the C harness, and both audits.
- `./gradlew connectedDebugAndroidTest` passed all 5 Android instrumentation tests on the conformance emulator.

The only hardware-only evidence remains the explicit physical BLE checkpoint; it requires the three real platform targets and flashed fixture and is intentionally not inferred from software transport results.
