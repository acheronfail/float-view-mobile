## RENAMED Requirements

- FROM: `### Requirement: Stable native loopback port registry`
- TO: `### Requirement: OS-assigned native loopback endpoints`

## MODIFIED Requirements

### Requirement: OS-assigned native loopback endpoints

Supported native backends SHALL bind only to loopback using an OS-assigned port. Hosts SHALL load the frontend from the actual bound origin, and bootstrap endpoints and authentication SHALL use that origin. A listener SHALL retain its assigned socket for its lifetime without probing ports before binding.

#### Scenario: Supported backend starts

- **WHEN** stable and beta native processes start concurrently
- **THEN** each receives an available loopback port and its frontend connects to its own service

#### Scenario: Assigned port unavailable

- **WHEN** a loopback listener cannot be created
- **THEN** startup fails visibly without loading another process's backend

#### Scenario: New backend allocation

- **WHEN** another native backend becomes supported
- **THEN** it uses OS assignment without allocating a fixed registry port

### Requirement: Browser-development native host seams

Each native platform SHALL provide only the launch, readiness, forwarding, process lifetime, and native-presentation seams required for host-browser development. Those seams MUST use the production Bluetooth, VESC protocol, application-service, storage, permission, lifecycle, and export implementations.

#### Scenario: macOS browser host

- **WHEN** macOS browser development starts
- **THEN** a native process reports its OS-assigned loopback origin and listens there without creating a WKWebView and retains CoreBluetooth, SQLite, permission, and save-panel ownership

#### Scenario: iOS browser host

- **WHEN** iOS browser development starts for a selected device
- **THEN** the installed application reports its OS-assigned device loopback port and a monitored `iproxy` exposes an available host loopback port

#### Scenario: Android browser host

- **WHEN** Android browser development starts for a selected device
- **THEN** the installed application reports its OS-assigned device loopback port and ADB exposes an OS-assigned host loopback port

#### Scenario: Production host behavior

- **WHEN** a native application runs outside browser-development orchestration
- **THEN** it continues serving the embedded frontend and owning native state with the same backend contract

## ADDED Requirements

### Requirement: Saved appearance before frontend startup

Native-served HTML SHALL carry the current saved theme and accent before frontend scripts run. The first canvas SHALL use those preferences, resolving system theme against current appearance. Browser storage SHALL remain an optional hint and SHALL NOT override embedded saved appearance.

#### Scenario: A new origin has no cached preference

- **WHEN** the assigned port changes and saved preferences differ from defaults or system appearance
- **THEN** the initial frontend canvas and subsequent app startup use the saved theme and accent without an intermediate default flash

#### Scenario: Preferences change before frontend recreation

- **WHEN** saved appearance changes and the HTML is requested again
- **THEN** that response carries the current saved values rather than listener-start values
