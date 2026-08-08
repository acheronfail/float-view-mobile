# macOS native host

The macOS executable is a sibling of the iOS host. Shared Apple adapters live
in `../common`; this directory contains the direct macOS capability entries,
window/WebView shell, and macOS bundle resources.

| Entry                                | Status       | Concrete owner                       | Focused evidence            | Physical checkpoint                          |
| ------------------------------------ | ------------ | ------------------------------------ | --------------------------- | -------------------------------------------- |
| `FloatViewBluetooth.swift`           | `incomplete` | `CoreBluetoothAdapter`               | CoreBluetooth mapping tests | physical reconnect and notifications         |
| `FloatViewBackgroundExecution.swift` | `incomplete` | application process lifecycle        | lifecycle tests             | inactive-window logging continuity           |
| `FloatViewLocation.swift`            | `incomplete` | `CoreLocationAdapter`                | location adapter tests      | authorization/provider transitions           |
| `FloatViewDocuments.swift`           | `incomplete` | `AppleSystemPresentation`            | presentation tests required | open/save success and cancellation           |
| `FloatViewAppearance.swift`          | `incomplete` | frontend `prefers-color-scheme`      | frontend theme tests        | live appearance change and reopen            |
| `FloatViewLifecycle.swift`           | `incomplete` | `MacAppDelegate`                     | lifecycle tests             | close/reopen and inactive/active transitions |
| `FloatViewWebView.swift`             | `incomplete` | `MacAppDelegate` window presentation | lifecycle tests             | WebContent process replacement               |
| `FloatViewRuntimeBridge.swift`       | `incomplete` | shared `FloatViewAppleRuntime`       | bridge/runtime tests        | process runtime continuity                   |

The Swift package compiles every entry in this directory. Shared owners are
referenced directly; there is no second macOS adapter layer.
