# iOS native host

The iOS target contains only operating-system integration. Product state,
protocols, persistence, imports/exports, and contract policy live in Rust; the
WebView owns presentation.

| Entry                                | Status       | Concrete owner                                | Focused evidence            | Physical checkpoint                            |
| ------------------------------------ | ------------ | --------------------------------------------- | --------------------------- | ---------------------------------------------- |
| `FloatViewBluetooth.swift`           | `incomplete` | `CoreBluetoothAdapter`                        | CoreBluetooth mapping tests | background connection and notification logging |
| `FloatViewBackgroundExecution.swift` | `incomplete` | iOS background modes plus the process runtime | lifecycle tests             | background BLE/GPS ride                        |
| `FloatViewLocation.swift`            | `incomplete` | `CoreLocationAdapter`                         | location adapter tests      | authorization and background updates           |
| `FloatViewDocuments.swift`           | `incomplete` | `AppleSystemPresentation`                     | presentation tests required | picker/share success and cancellation          |
| `FloatViewAppearance.swift`          | `incomplete` | frontend `prefers-color-scheme`               | frontend theme tests        | live appearance change and foreground return   |
| `FloatViewLifecycle.swift`           | `incomplete` | `RustAppBackend`                              | lifecycle UI tests          | background/foreground and scene recreation     |
| `FloatViewWebView.swift`             | `incomplete` | `WebView`                                     | lifecycle UI tests          | WebContent process replacement                 |
| `FloatViewRuntimeBridge.swift`       | `incomplete` | shared `FloatViewAppleRuntime`                | bridge/runtime tests        | process runtime continuity                     |

Every named file is compiled by the Xcode synchronized source group. An
`incomplete` entry is intentionally visible until its focused and physical
evidence is complete; it must not be mistaken for an unsupported capability.
