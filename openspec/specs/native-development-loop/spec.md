# Native Development Loop Specification

## Purpose

Define browser-based Vite hot-reload development against deterministic mock and selected native backends while preserving native ownership and production transport security.

## Requirements

### Requirement: Platform-selected native development

The repository SHALL provide `just dev mock|macos|ios|android [target]` as the public hot-reload entry point. A native selection MUST run the shared frontend in a development-host browser against exactly one selected real native backend, while `mock` MUST run the deterministic mock backend.

#### Scenario: macOS frontend iteration

- **WHEN** a developer runs `just dev macos`
- **THEN** Vite hot module replacement serves the shared frontend in the host browser while the macOS process owns the production CoreBluetooth, application-service, storage, permission, and export paths

#### Scenario: Selected mobile target

- **WHEN** a developer runs `just dev ios <target>` or `just dev android <target>`
- **THEN** the selected installed application owns its native backend while the host browser uses that backend through the platform forwarding transport

#### Scenario: Deterministic mock

- **WHEN** a developer runs `just dev mock`
- **THEN** Vite starts with the deterministic mock backend and no native application is built or launched

### Requirement: Authenticated Vite proxy

Native browser development SHALL proxy bootstrap and WebSocket traffic through loopback Vite without changing the backend wire contract. The proxy MUST preserve the random launch token and MUST present the selected backend's exact expected Origin on the upstream WebSocket upgrade.

#### Scenario: Native bootstrap

- **WHEN** the Vite frontend loads bootstrap for a ready native backend
- **THEN** the browser opens its WebSocket through Vite with the current launch token and the backend accepts the rewritten exact Origin

#### Scenario: Production bootstrap

- **WHEN** the inline production frontend consumes a backend bootstrap document
- **THEN** it uses the backend-provided endpoint without development proxy normalization

#### Scenario: Invalid token

- **WHEN** a development WebSocket omits or changes the current launch token
- **THEN** the unchanged native backend authentication rejects the connection

### Requirement: Loopback-only development frontend

Vite SHALL bind only to `127.0.0.1:11002`, use `127.0.0.1:11003` for the deterministic mock, and proxy only to the explicitly selected loopback backend. Development tooling MUST NOT expose Vite on a LAN interface.

#### Scenario: Development server starts

- **WHEN** a development session becomes ready
- **THEN** its browser frontend and HMR endpoint are available at `http://127.0.0.1:11002`

#### Scenario: Development port occupied

- **WHEN** port `11002` is unavailable
- **THEN** startup fails with an actionable diagnostic instead of selecting another port

### Requirement: Bounded orchestration and cleanup

Development orchestration SHALL bound backend, forwarding, proxy, and Vite readiness; supervise every child process it starts; and clean up only resources it owns. It MUST preserve installed applications, OS permissions, and app-owned data across ordinary development sessions.

#### Scenario: Ready ordering

- **WHEN** a native development session starts
- **THEN** the selected backend and any required forwarding are ready before the browser frontend attempts bootstrap

#### Scenario: Child process failure

- **WHEN** Vite, the selected native host, ADB transport, or `iproxy` exits unexpectedly
- **THEN** the session terminates its remaining owned children and reports the first actionable failure within a bounded deadline

#### Scenario: Developer interruption

- **WHEN** the developer interrupts `just dev`
- **THEN** Vite and owned proxy processes stop, owned forwarding rules are removed, and the installed application, permissions, and persisted Float View data remain available

### Requirement: Browser development preserves native ownership

The development browser SHALL remain a disposable client. Bluetooth, application state, persistence, background work, permissions, lifecycle, and export presentation MUST remain owned by the selected native process.

#### Scenario: Browser reload

- **WHEN** Vite hot replacement or a manual browser reload replaces the frontend
- **THEN** the replacement client recovers the authoritative native state without restarting Bluetooth or backend jobs

#### Scenario: Native presentation

- **WHEN** the development browser requests an operation with native permission or export presentation
- **THEN** the selected native host performs that presentation without a frontend platform branch

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
