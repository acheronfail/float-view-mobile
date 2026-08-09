## Context

Apple and Android already implement equivalent native responsibilities independently, but their source trees describe those responsibilities differently. Apple has reasonably focused shared-backend folders while location remains under `application`, loopback transport is called `server`, and iOS/macOS presentation concerns are concentrated in large host files. Android has matching protocol, Bluetooth, location, and storage packages, but application code is named `backend` and runtime, lifecycle, transport, theme, and document handling are mixed in the root package and `MainActivity`.

This change is a behavior-preserving reorganization. The native build systems still require different outer source roots: Swift Package Manager/Xcode targets on Apple and the conventional Gradle application source set on Android. The common structure therefore begins at each production target's language source root rather than forcing identical repository paths above those roots.

## Goals / Non-Goals

**Goals:**

- Make the same capability name answer “where is Bluetooth/location/file/lifecycle/theme/transport code?” on both backends.
- Keep native host integrations next to the capability they implement instead of in catch-all activity, view, or runtime files.
- Preserve the current dependency direction and independent Swift/Kotlin implementations.
- Enforce the layout in the normal repository audit so later changes cannot silently reintroduce ambiguous root sources or the old package names.
- Keep all native builds, tests, and conformance behavior unchanged.

**Non-Goals:**

- Sharing implementation code, introducing Rust or FFI, generating a contract, or adding native modules.
- Redesigning application services, protocols, persistence, loopback transport, WebViews, permission flows, native file presentation, or theme behavior.
- Adding missing runtime capabilities or changing platform background eligibility.
- Renaming public wire types, changing package/application identifiers, or migrating stored data.

## Decisions

### 1. Standardize a capability vocabulary below native source roots

Production code uses these names wherever the capability exists:

- `application`: domain types, application coordination, process runtime composition, schedulers, and polling
- `bluetooth`: BLE adapters, policies, permission presentation, and GATT ownership
- `files`: native file selection, bounded import reading, export staging presentation, and share/save UI
- `lifecycle`: process/application lifetime and background execution services
- `location`: provider adapters and location permission presentation
- `protocol`: VESC and Refloat wire implementations, retaining `vesc` and `refloat` subtrees
- `storage`: SQLite stores, schema access, and storage policy
- `theme`: native chrome/appearance synchronization driven by authoritative settings and system appearance
- `transport`: the authenticated loopback server and transport startup errors/ports
- `ui`: disposable WebView and native window/activity/view composition

Apple keeps separate source roots for the shared `FloatViewBackend`, the iOS host, and the macOS host; Android keeps one Gradle application source root. A target only needs the capability folders it actually owns, but it must use the canonical name rather than inventing a platform-specific synonym. This preserves build-system conventions while making the path vocabulary match.

Alternatives considered:

- Making `backends/apple` and `backends/android` literally identical above their build roots would fight Xcode, SwiftPM, and Gradle conventions and make ordinary platform tooling harder to use.
- Introducing separate modules per capability would turn a navigation problem into new API and build boundaries, contrary to the one-module Android and one-shared-backend Apple requirements.
- Documentation without file moves would leave the ambiguous root files and mixed responsibilities that caused the navigation problem.

### 2. Split mixed platform host files only at existing ownership seams

Android extracts the existing permission brokers, file chooser/import/export flow, theme application, foreground service, loopback server, runtime, and WebView activity into their owning packages. Apple extracts the existing iOS/macOS runtime, file presentation, lifecycle, theme helpers, and UI/entry composition into focused files and moves shared location and loopback implementations to the canonical folders.

The split does not create new interfaces solely for folder purity. Small helpers may remain close to the owner that exclusively uses them, and UI/lifecycle framework callbacks remain in the framework type required to receive them while delegating capability work to focused collaborators.

### 3. Mirror test locations and use package declarations as the Android boundary

Swift tests remain in the existing test target because Swift does not derive namespaces from folders. Kotlin production and test packages change from `fail.acheron.floatview.backend` or the root package to the corresponding canonical capability package, and imports are updated mechanically. Android application IDs and manifest component identities remain unchanged externally; the manifest uses the new fully qualified implementation classes.

### 4. Enforce the source shape through `just audit`

The audit checks allowed direct production capability directories and rejects superseded Apple `server`, Android `backend`, and ambiguous Android root Kotlin production files. Backend-local README files provide the source-root mapping, dependency direction, and a capability-to-path table. Build/test compilation remains the authority for reference completeness.

## Risks / Trade-offs

- **Large rename diff can hide accidental edits** → perform moves first, keep extraction mechanical, and compare behavior with native unit tests and canonical conformance.
- **Android package renames can miss string-based component references** → update the manifest and development launch command explicitly, then assemble and run Android conformance.
- **Xcode may omit moved host files** → retain filesystem-synchronized groups and verify the simulator build.
- **Folder symmetry can be mistaken for shared abstractions** → document that names match while implementations, types, and build targets remain platform-owned.
- **Capability code can still meet in framework-owned activity/view callbacks** → require those callbacks to delegate file and theme work to focused owners rather than creating artificial modules or callback layers.

## Migration Plan

1. Commit the OpenSpec artifacts independently.
2. Move shared Apple and Android implementation files into the canonical capability locations, updating packages/imports and project launch references.
3. Extract mixed native host concerns into focused capability files without changing logic.
4. Add backend layout documentation and audit checks.
5. Run focused Apple/Android tests and builds, then `just verify`.

Rollback is a normal source revert because there are no contract, data, dependency, or deployed-state changes.

## Open Questions

None. Further capability work intentionally starts only after this structural slice is verified.
