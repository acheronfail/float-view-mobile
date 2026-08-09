## MODIFIED Requirements

### Requirement: Thin Apple hosts

The iOS and macOS hosts SHALL occupy the explicit sibling roots `/backends/apple/ios` and `/backends/apple/macos`. Those hosts SHALL contain only application entry, lifecycle, WebView, permission presentation, export presentation, platform resources, and other platform wiring not owned by the shared backend. Shared Apple behavior SHALL remain outside both host roots in the `FloatViewBackend` library.

#### Scenario: Locate an Apple platform host

- **WHEN** a developer inspects `/backends/apple`
- **THEN** the iOS and macOS application hosts are found at the sibling `ios/` and `macos/` roots while shared backend code remains separately identified

#### Scenario: WebView replacement

- **WHEN** an Apple host replaces its WebView
- **THEN** it reconnects to the same native backend state without reimplementing backend behavior

#### Scenario: Build the macOS sibling host

- **WHEN** SwiftPM builds the `FloatViewMac` executable
- **THEN** it compiles the host and copies its resources from `/backends/apple/macos` without recreating a platform host under `Sources`
