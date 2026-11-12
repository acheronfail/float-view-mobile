## RENAMED Requirements

- FROM: `### Requirement: Conformance uses the native port registry`
- TO: `### Requirement: Conformance discovers native endpoints`

## MODIFIED Requirements

### Requirement: Conformance discovers native endpoints

Canonical native target adapters SHALL discover the current process's assigned loopback port. Mobile forwarding SHALL use an available host port and the actual device port while preserving the device origin for authentication. Cleanup SHALL remove only forwarding resources owned by the target.

#### Scenario: macOS conformance endpoint

- **WHEN** conformance starts the macOS target
- **THEN** readiness supplies its actual origin and authenticated WebSocket endpoint

#### Scenario: iOS conformance forwarding

- **WHEN** conformance starts an iOS target
- **THEN** it discovers the current device port and exposes it through a monitored loopback proxy or connects directly on the simulator

#### Scenario: Android conformance forwarding

- **WHEN** conformance starts an Android target
- **THEN** ADB allocates a host port forwarding to the discovered device port

#### Scenario: Forwarding cleanup

- **WHEN** a mobile conformance target stops or fails
- **THEN** its adapter removes only its own forwarding rule or proxy without disturbing other endpoints

## ADDED Requirements

### Requirement: Assigned port and startup appearance regression coverage

Focused checks SHALL verify simultaneous independently authenticated listeners, published bound origins, and saved theme/accent before application modules run with missing or stale browser storage. Native host and tooling checks SHALL exercise endpoint discovery and scoped cleanup.

#### Scenario: Listeners coexist

- **WHEN** multiple listeners bind with OS assignment
- **THEN** they report distinct live ports, serve their own bootstrap and reject another listener's authentication token

#### Scenario: First script uses saved appearance

- **WHEN** native HTML embeds saved appearance and browser storage is missing, inaccessible or stale
- **THEN** the earliest startup script uses saved preferences, including system appearance resolution, and app initialization preserves them
