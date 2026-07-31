## ADDED Requirements

### Requirement: Stable native loopback port registry

Supported native backends SHALL bind only to loopback and SHALL use stable default ports: macOS `11010`, iOS `11011`, and Android `11012`. A newly supported backend MUST receive the next unassigned port beginning at `11013`.

#### Scenario: Supported backend starts

- **WHEN** a macOS, iOS, or Android backend starts
- **THEN** its loopback server listens on its assigned platform port

#### Scenario: Assigned port unavailable

- **WHEN** the assigned loopback address and port cannot be bound
- **THEN** the native host fails startup with an actionable diagnostic and does not silently select another port

#### Scenario: New backend allocation

- **WHEN** another native backend becomes supported
- **THEN** it receives one documented unused port from the consecutive registry beginning at `11013`

### Requirement: Browser-development native host seams

Each native platform SHALL provide only the launch, readiness, forwarding, process lifetime, and native-presentation seams required for host-browser development. Those seams MUST use the production Bluetooth, VESC protocol, application-service, storage, permission, lifecycle, and export implementations.

#### Scenario: macOS browser host

- **WHEN** macOS browser development starts
- **THEN** a native process listens on `127.0.0.1:11010` without creating a WKWebView and retains CoreBluetooth, SQLite, permission, and save-panel ownership

#### Scenario: iOS browser host

- **WHEN** iOS browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11011` and a monitored `iproxy` exposes host loopback port `11011`

#### Scenario: Android browser host

- **WHEN** Android browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11012` and ADB exposes host loopback port `11012`

#### Scenario: Production host behavior

- **WHEN** a native application runs outside browser-development orchestration
- **THEN** it continues serving the embedded frontend and owning native state with the same backend contract
