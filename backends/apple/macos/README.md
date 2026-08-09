# macOS native host

The macOS executable is a sibling of the iOS host. Shared Apple adapters live
in `../common`; this directory contains the direct macOS capability entries,
window/WebView shell, and macOS bundle resources.

| Entry                                | Status       | Concrete owner                       | Focused evidence            | Physical checkpoint                          |
| ------------------------------------ | ------------ | ------------------------------------ | --------------------------- | -------------------------------------------- |
| `FloatViewBluetooth.swift`           | `implemented` | `CoreBluetoothAdapter`               | CoreBluetooth mapping tests | physical 16-scenario matrix passes |
| `FloatViewBackgroundExecution.swift` | `implemented` | application process lifecycle        | lifecycle tests             | normal process eligibility; mobile service is unavailable |
| `FloatViewLocation.swift`            | `implemented` | `CoreLocationAdapter`                | authorization/observation tests | physical authorization/provider transition passes |
| `FloatViewDocuments.swift`           | `implemented` | `AppleSystemPresentation`            | bounded presentation tests | physical open/save roundtrip passes |
| `FloatViewAppearance.swift`          | `implemented` | frontend `prefers-color-scheme`      | frontend theme tests        | physical live change, return, and recreation passed |
| `FloatViewLifecycle.swift`           | `implemented` | `MacAppDelegate`                     | lifecycle tests             | physical close/reopen and inactive/active transitions pass |
| `FloatViewWebView.swift`             | `implemented` | `MacAppDelegate` window presentation | lifecycle tests             | physical WebContent replacement passes |
| `FloatViewRuntimeBridge.swift`       | `implemented` | shared `FloatViewAppleRuntime`       | bridge/runtime tests        | physical process runtime continuity passes |

The Swift package compiles every entry in this directory. Shared owners are
referenced directly; there is no second macOS adapter layer. The executable
embeds `../FloatViewMac-Info.plist` so Bluetooth and the narrower when-in-use
Core Location usage descriptions are present in SwiftPM builds.
