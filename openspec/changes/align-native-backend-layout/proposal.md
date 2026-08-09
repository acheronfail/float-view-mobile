## Why

The native implementations expose the same platform responsibilities through visibly different source layouts, so maintainers cannot reliably locate Bluetooth, location, lifecycle/background, file import/export, theme, transport, or application code by following the same path on Apple and Android. Before making further backend changes, establish one capability-oriented layout that keeps every implementation native and makes ownership obvious without another rewrite.

## What Changes

- Give the Apple and Android production source trees the same named capability boundaries for application coordination, Bluetooth, file import/export, lifecycle/background execution, location, protocol, storage, theme, transport, and host/UI composition.
- Move existing files and split mixed host files only where necessary to place platform integrations under their owning capability, preserving current behavior and native APIs.
- Add backend-local layout documentation and a repository check that prevents new production sources from drifting back into ambiguous or platform-specific top-level locations.
- Update native project references, package declarations, tests, and maintainer documentation for the new locations.
- Keep Swift and Kotlin implementations independent; do not introduce Rust, FFI, generated APIs, a shared implementation directory, or new Gradle/Swift package modules.
- Do not change the frontend wire contract, runtime behavior, persistence format, background eligibility, native file presentation, theme resolution, supported platforms, or dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-hosts`: Replace the currently asymmetric source-location requirement with one matching capability-oriented layout for Apple and Android, including explicit locations for Bluetooth, location, files, lifecycle, theme, transport, protocol, storage, application coordination, and host composition.

## Impact

- Affects production and test source locations under `backends/apple` and `backends/android`, Apple Xcode project references, Kotlin package declarations/imports, backend architecture documentation, repository verification, and the `platform-hosts` specification.
- Does not affect contract APIs, SQL, fixtures, conformance semantics, platform entitlements/permissions, build dependencies, or user-visible behavior.
