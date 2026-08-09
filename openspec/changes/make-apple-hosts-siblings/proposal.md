## Why

The iOS host already has a first-class `backends/apple/ios` root, while the macOS host is hidden inside SwiftPM's shared `Sources` tree. Making the two platform hosts siblings gives maintainers an immediate, symmetric place to find platform-specific code without changing the shared Apple backend.

## What Changes

- Move the macOS application host from `backends/apple/Sources/FloatViewMac` to `backends/apple/macos`, beside `backends/apple/ios`.
- Configure SwiftPM to build the existing `FloatViewMac` executable from the explicit macOS host path.
- Update resources, development tooling, conformance references, documentation, and layout audits to use the sibling roots.
- Remove the superseded `Sources/FloatViewMac` path.
- Preserve all runtime behavior, native ownership boundaries, executable/product names, and the shared `Sources/FloatViewBackend` implementation.
- Non-goals: changing the frontend contract, moving shared Apple backend code, altering iOS project structure, introducing shared cross-language code, or changing BLE, backgrounding, location, file, or theme behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-hosts`: Require the iOS and macOS platform hosts to occupy explicit sibling roots beneath `backends/apple`.

## Impact

Affected areas are the Apple Swift package target path, macOS host sources/resources, the root `justfile`, icon generation, Apple ownership documentation, and repository layout verification. There is no cross-platform API, persistence, dependency, or compatibility behavior change.
