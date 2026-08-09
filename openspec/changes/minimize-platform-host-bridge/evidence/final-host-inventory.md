# Final host inventory and verification

Updated after the reconciled host implementation on 2026-08-09. `just
host-audit` excludes generated sources, dependencies, and build output and
reports production and test files separately. Line counts use the same baseline
scope and include `Package.swift` in Swift production.

| Host   | Baseline production | Final production |     Removed | Final tests |
| ------ | ------------------: | ---------------: | ----------: | ----------: |
| Swift  |               3,703 |            3,250 | 453 (12.2%) |       1,143 |
| Kotlin |               3,368 |            2,963 | 405 (12.0%) |       1,224 |

The production hosts retain 30 Swift and 27 Kotlin source files, plus the Swift
package declaration, including the predictable per-capability entry files. The
audit rejects native software peripherals, reverse bridge codecs, random
transport aliases, unused saved-device APIs, obsolete layout paths, and any
retained command missing from the 13-kind reachability manifest.

## Apple ownership

| Remaining files                                                                                                                                                         | Why they remain native                                                                                                                                                                                                                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CoreBluetoothAdapter.swift`, `VescBluetoothPolicy.swift`                                                                                                               | CoreBluetooth discovery/GATT callbacks, notification setup, negotiated writes, backpressure, and stale callback rejection.                                                                                                                                                              |
| `CoreLocationAdapter.swift`, `FloatViewLocationPort.swift`                                                                                                              | Core Location authorization and focused observation delivery.                                                                                                                                                                                                                           |
| `CoreBluetoothAdapter.swift`, `FloatViewBluetoothPort.swift`, `FloatViewCoreRuntime.swift`, `FloatViewPlatformBridgeCodec.swift`, `FloatViewRuntimeBridgeAdapter.swift` | OS transport primitives, caller-owned C buffers, the single off-main pull worker, decoding of the 13 Rust commands, focused event submission, and presentation replacement. `FloatViewAppleRuntime` is the one shared iOS/macOS composition owner. No reverse production codec remains. |
| `FloatViewRuntimeHost.swift`, `AppleSystemPresentation.swift`, per-host `FloatViewDocuments.swift`                                                                      | Canonically contained staging paths, native open/save/picker presentation, security-scoped bounded copies, and permission/settings presentation.                                                                                                                                        |
| iOS and macOS `FloatView*` capability entries                                                                                                                           | Direct discoverable ownership seams for process/app lifecycle, startup configuration, WebKit ownership, native capabilities, and disposable frontend presentation.                                                                                                                      |

`Package.swift` embeds `FloatViewMac-Info.plist` in the executable so the
Bluetooth and location usage descriptions belong to the same shipped binary.
The macOS location entry requests only when-in-use access; normal process
lifetime, rather than a mobile background launch service, is its eligibility
mechanism.

## Android ownership

| Remaining files                                                                                                                                                                              | Why they remain native                                                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AndroidBluetoothAdapter.kt`, `AndroidBluetoothPermissionPolicy.kt`, `VescBluetoothPolicy.kt`, `VescGattProfile.kt`                                                                          | Android permission/radio/GATT APIs, notification descriptor setup, negotiated writes, bounded queues, and stale callback rejection.                                                                               |
| `AndroidLocationProvider.kt`, `backend/LocationPort.kt`                                                                                                                                      | Android location authorization and focused observation delivery.                                                                                                                                                  |
| `NativeCore.kt`, `CoreBridgeWorker.kt`, `PlatformBridgeCodec.kt`, `RustBluetoothPlatformAdapter.kt`, `FloatViewRuntimeBridgeAdapter.kt`, `RustBackendRuntime.kt`, `backend/BluetoothPort.kt` | Explicit JNI registration, copied arrays, the single blocking non-main pull worker, decoding of the 13 Rust commands, focused event submission, and the one process runtime. No reverse production codec remains. |
| `FloatViewRuntimeHost.kt`, `AndroidPresentationBroker.kt`, `FloatViewDocuments.kt`, `PermissionBrokers.kt`                                                                                   | Bounded ContentResolver import, short-lived FileProvider share copies, and permission/settings presentation owned by Android.                                                                                     |
| `FloatViewApplication.kt`, `MainActivity.kt`, `LoggingForegroundService.kt`                                                                                                                  | Process/Activity/WebView lifecycle, permission result routing, and Android foreground-service ownership.                                                                                                          |

The bridge/runtime files are larger than simple OS adapters because they contain the private memory-safety boundary: fixed-width ABI identity checks, bounded copying, wait-buffer growth without dequeue, generation checks, shutdown wake, and one-way command dispatch. Those responsibilities cannot move below C/JNI without recreating callbacks or sharing allocators.

## Final bridge and gate evidence

- `backends/core/HOST_COMMANDS.tsv` accounts for all 13 retained command kinds, their Rust requester, host dispatcher, completion bound, and conformance owner.
- The C header is generated from ABI 4 and checked by checksum; the pure-C harness covers focused events, invalid values, sizes/alignment, resize-without-dequeue, stop/wake, stale handles, and panic containment.
- Rust receives focused Bluetooth, location, and lifecycle inputs; stale order is rejected within each channel and connection generation remains Bluetooth-owned.
- The retained Swift/Kotlin Bluetooth callback value is now limited to five OS observations and three terminal connection states (`ready`, `failed`, `disconnected`). Replacing that single callback sink with five forwarding protocol methods would add host boilerplate without removing another mapping layer; all portable connection intent and intermediate state remain in Rust.
- The Rust software host completes writes before delivering notifications, rejects prior VESC-generation responses, and shares the production actor/server/storage/shutdown graph.
- The refreshed focused gates pass the Rust workspace, frontend, pure-C,
  Swift, Kotlin JVM, browser/mock, iOS Simulator, and Android emulator targets.
  Android's 14 connected instrumentation tests pass on both the API-29 physical
  device and API-36 emulator. The physical run found and fixed a late completion
  racing bridge shutdown; completions invalidated by close are now ignored
  without weakening duplicate/stale rejection while the runtime is active.
  Full `just verify` remains the final task after the physical matrix closes.

The explicit physical BLE, reconnect, background-runtime, frontend-replacement,
stale-callback, Android/macOS document, and three-host appearance evidence is in
`physical-checkpoints.md`. The only unresolved physical evidence is the iOS
picker/share roundtrip and a persisted iPhone coordinate; neither is inferred
from software transport or authorization alone.
