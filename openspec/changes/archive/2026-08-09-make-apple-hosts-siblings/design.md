## Context

The Apple backend has three build concerns: a shared Swift library in `Sources/FloatViewBackend`, an iOS Xcode host in `ios`, and a macOS SwiftPM executable currently inferred from `Sources/FloatViewMac`. The inferred SwiftPM convention makes the macOS platform root look like shared implementation and prevents the two platform hosts from being immediate siblings.

## Goals / Non-Goals

**Goals:**

- Establish `backends/apple/ios` and `backends/apple/macos` as sibling platform-host roots.
- Preserve the shared `FloatViewBackend` library and the `FloatViewMac` executable/product identity.
- Keep all macOS capability directories and resources intact beneath the new root.
- Make build scripts and audits enforce the new structure.

**Non-Goals:**

- Moving `Sources/FloatViewBackend` or `Sources/CSQLite` out of conventional SwiftPM source roots.
- Converting iOS to SwiftPM or macOS to an Xcode project.
- Changing native behavior, APIs, persistence, dependencies, capabilities, or ownership.

## Decisions

### Use an explicit SwiftPM target path

`Package.swift` will set the `FloatViewMac` executable target's `path` to `macos`. SwiftPM permits explicit target paths, so the product and module names remain unchanged while the filesystem represents platform ownership directly. Renaming the module or creating a nested package was rejected because either would add churn or a second dependency graph without improving ownership.

### Move the complete macOS host root

All macOS source capability directories and `Resources` move together from `Sources/FloatViewMac` to `macos`. This keeps relative resource layout stable and avoids a split host whose source and assets live in different ownership roots.

### Keep shared Apple implementation conventional

The shared backend remains under `Sources/FloatViewBackend`, and CSQLite remains under `Sources/CSQLite`. These are package targets shared by both hosts, not platform-host directories, so moving them would obscure rather than clarify ownership.

### Enforce both positive and negative layout rules

Documentation and `just audit` will identify the two sibling host roots and reject a reintroduced `Sources/FloatViewMac`. Build, development, icon-generation, resource-copy, and conformance references will point only to the new macOS root.

## Risks / Trade-offs

- [A stale path remains in tooling] → Search the retained repository for `Sources/FloatViewMac`, update every live reference, and make the audit reject the old directory.
- [SwiftPM resource discovery changes] → Move `Resources` with the target and run the macOS build plus canonical conformance.
- [Historical OpenSpec artifacts retain old paths] → Leave immutable archives unchanged; searches and audits exclude historical documentation where appropriate.

## Migration Plan

Move the host directory, declare the explicit SwiftPM target path, update all live path consumers, then run focused macOS and iOS builds followed by `just verify`. Rollback is a single directory move plus removal of the explicit target path.

## Open Questions

None.
