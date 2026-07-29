## ADDED Requirements

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
