# Android native host

This package is the predictable entry surface for Android OS integration.
Product state, VESC policy, persistence, import/export semantics, and the
frontend contract remain in Rust or the shared WebView frontend.

| Entry                             | Status       | Concrete owner                               | Focused evidence            | Physical checkpoint                            |
| --------------------------------- | ------------ | -------------------------------------------- | --------------------------- | ---------------------------------------------- |
| `FloatViewBluetooth.kt`           | `implemented` | `AndroidBluetoothAdapter`                    | JVM Bluetooth mapping tests | physical matrix and background notifications pass |
| `FloatViewBackgroundExecution.kt` | `implemented` | `LoggingForegroundService`                   | service lifecycle tests     | physical background BLE/GPS ride passes |
| `FloatViewLocation.kt`            | `implemented` | `AndroidLocationProvider`                    | provider/permission tests   | physical background GPS confirmed |
| `FloatViewDocuments.kt`           | `implemented` | `AndroidPresentationBroker`                  | bounded presentation tests | physical SAF/share roundtrip passes |
| `FloatViewAppearance.kt`          | `implemented` | frontend media query plus native system bars | frontend/activity tests     | physical live change, return, and recreation passed |
| `FloatViewLifecycle.kt`           | `implemented` | `FloatViewApplication` and `MainActivity`    | lifecycle tests             | physical process/Activity recreation passes |
| `FloatViewWebView.kt`             | `implemented` | `MainActivity` disposable WebView owner      | lifecycle tests             | physical renderer replacement passes |
| `FloatViewRuntimeBridge.kt`       | `implemented` | `RustBackendRuntime`                         | bridge/runtime tests        | instrumentation and physical process continuity pass |

Every entry is a compiled implementation or direct forwarding seam. No missing
entry is treated as unsupported; all Android capability rows have focused and
physical evidence.
