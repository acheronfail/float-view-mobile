## 1. Android capability layout

- [x] 1.1 Move Android application, transport, lifecycle, and UI production sources into canonical capability packages and update package declarations, imports, manifest component names, tests, and launch tooling.
- [x] 1.2 Extract Android Bluetooth/location permission presentation, native session import/export handling, and native theme synchronization from mixed runtime/activity files into their canonical capability packages without changing behavior.

## 2. Apple capability layout

- [x] 2.1 Move the shared Apple location and loopback transport implementations into canonical capability directories and preserve Swift package and Xcode builds.
- [x] 2.2 Split the iOS and macOS host sources into focused application, files, lifecycle, theme, transport, and UI files beneath their native target roots without changing behavior.

## 3. Layout contract and documentation

- [x] 3.1 Document the canonical capability vocabulary, native source roots, dependency direction, privacy boundary, and firmware-family extension rule for both backends.
- [x] 3.2 Extend `just audit` to reject superseded or ambiguous native production source locations while allowing build-system metadata and resources.

## 4. Verification

- [x] 4.1 Run focused Android and Apple unit tests plus Android, macOS, and iOS builds.
- [x] 4.2 Run `just verify` and resolve any layout, build, test, or canonical conformance regressions.
