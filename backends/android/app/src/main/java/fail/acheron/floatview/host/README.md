# Android native host

This package is the predictable entry surface for Android OS integration.
Product state, VESC policy, persistence, import/export semantics, and the
frontend contract remain in Rust or the shared WebView frontend.

| Entry                             | Status       | Concrete owner                               | Focused evidence            | Physical checkpoint                            |
| --------------------------------- | ------------ | -------------------------------------------- | --------------------------- | ---------------------------------------------- |
| `FloatViewBluetooth.kt`           | `incomplete` | `AndroidBluetoothAdapter`                    | JVM Bluetooth mapping tests | background connection and notification logging |
| `FloatViewBackgroundExecution.kt` | `incomplete` | `LoggingForegroundService`                   | service lifecycle tests     | background BLE/GPS ride                        |
| `FloatViewLocation.kt`            | `incomplete` | `AndroidLocationProvider`                    | provider/permission tests   | provider and background transitions            |
| `FloatViewDocuments.kt`           | `incomplete` | `AndroidPresentationBroker`                  | presentation tests required | SAF/share success and cancellation             |
| `FloatViewAppearance.kt`          | `incomplete` | frontend media query plus native system bars | frontend/activity tests     | live appearance change and Activity recreation |
| `FloatViewLifecycle.kt`           | `incomplete` | `FloatViewApplication` and `MainActivity`    | lifecycle tests             | process/Activity recreation                    |
| `FloatViewWebView.kt`             | `incomplete` | `MainActivity` disposable WebView owner      | lifecycle tests             | renderer replacement                           |
| `FloatViewRuntimeBridge.kt`       | `incomplete` | `RustBackendRuntime`                         | bridge/runtime tests        | process runtime continuity                     |

Every entry is a compiled implementation or direct forwarding seam. No missing
entry is treated as unsupported, and every `incomplete` row must become
`implemented` or an explanatory compiling `unsupported` stub before the final
host audit passes.
