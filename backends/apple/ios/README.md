# iOS native host

The iOS target contains only operating-system integration. Product state,
protocols, persistence, imports/exports, and contract policy live in Rust; the
WebView owns presentation.

| Entry                                | Status       | Concrete owner                                | Focused evidence            | Physical checkpoint                            |
| ------------------------------------ | ------------ | --------------------------------------------- | --------------------------- | ---------------------------------------------- |
| `FloatViewBluetooth.swift`           | `implemented` | `CoreBluetoothAdapter`                        | CoreBluetooth mapping tests | physical matrix and background notifications pass |
| `FloatViewBackgroundExecution.swift` | `implemented` | iOS background modes plus the process runtime | lifecycle and adapter tests | physical background BLE continuity recorded   |
| `FloatViewLocation.swift`            | `implemented` | `CoreLocationAdapter`                         | authorization/observation tests | physical authorization and background availability pass |
| `FloatViewDocuments.swift`           | `incomplete` | `AppleSystemPresentation`                     | presentation tests required | picker/share success and cancellation          |
| `FloatViewAppearance.swift`          | `implemented` | frontend `prefers-color-scheme`               | frontend theme tests        | physical live change, return, and recreation passed |
| `FloatViewLifecycle.swift`           | `implemented` | `RustAppBackend`                              | lifecycle UI tests          | physical background/foreground and scene recreation pass |
| `FloatViewWebView.swift`             | `implemented` | `WebView`                                     | lifecycle UI tests          | physical disposable-WebView recovery passes |
| `FloatViewRuntimeBridge.swift`       | `implemented` | shared `FloatViewAppleRuntime`                | bridge/runtime tests        | physical process runtime continuity passes |

Every named file is compiled by the Xcode synchronized source group. An
The native document row remains visibly incomplete until the physical iOS
picker/share roundtrip is confirmed; it must not be mistaken for unsupported.
