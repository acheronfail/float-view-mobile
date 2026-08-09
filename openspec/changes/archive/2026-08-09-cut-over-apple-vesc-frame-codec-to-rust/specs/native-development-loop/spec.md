## ADDED Requirements

### Requirement: Locked source-built Apple Rust frame codec

The repository SHALL build the Apple Rust frame-codec static library from the committed workspace and lockfile with the pinned toolchain, locked dependency resolution, and offline mode. It SHALL support the current macOS host architecture, iOS device architecture, and active Intel or Apple Silicon iOS simulator architecture without committing native archives, generating bindings, or downloading tools during an Apple build.

Root `just` build, test, verification, run, and development paths that compile the Apple backend SHALL build the matching Rust target and configuration before Swift linkage. The iOS Xcode build SHALL invoke the same target/configuration mapping for its active SDK and architectures. Missing Rust targets, mismatched ABI versions, absent archives, unsupported architecture sets, and link failures SHALL stop with actionable diagnostics rather than silently selecting Swift behavior or a stale binary.

#### Scenario: Clean macOS development build

- **WHEN** a developer builds, tests, runs, or starts browser development for macOS from a clean ignored-artifact state
- **THEN** the pinned locked Rust host archive is built first and the shared Apple backend links and executes it

#### Scenario: Clean iOS simulator build

- **WHEN** Xcode or canonical tooling builds the iOS simulator host on Intel or Apple Silicon
- **THEN** the helper selects every active simulator architecture, builds the matching locked Rust archive or archives, and links the shared backend without a committed binary

#### Scenario: Clean iOS device build

- **WHEN** canonical tooling performs an unsigned device compile or Xcode performs a signed Debug or Release device build
- **THEN** the `aarch64-apple-ios` Rust archive is built from source for the matching configuration and linked before application launch

#### Scenario: Missing compilation prerequisite

- **WHEN** the pinned toolchain or selected Apple target is unavailable
- **THEN** the Apple build fails before Swift linkage and reports the exact prerequisite instead of fetching it, using an incompatible archive, or falling back to Swift

#### Scenario: Audit repository artifacts

- **WHEN** repository verification inspects tracked files and the linked archive
- **THEN** no generated header or native binary is tracked and the application ABI exports only its approved version, encode, and exact-decode entry points
