## ADDED Requirements

### Requirement: Conformance uses the native port registry
Canonical target adapters SHALL use macOS port `11010`, iOS port `11011`, and Android port `11012`. Mobile target adapters MUST forward the same port number on the host and device and MUST remove only their own forwarding rule during cleanup.

#### Scenario: macOS conformance endpoint
- **WHEN** conformance starts the macOS target
- **THEN** readiness and authenticated WebSocket access use `127.0.0.1:11010`

#### Scenario: iOS conformance forwarding
- **WHEN** conformance starts an iOS target
- **THEN** `iproxy` exposes host port `11011` to device loopback port `11011`

#### Scenario: Android conformance forwarding
- **WHEN** conformance starts an Android target
- **THEN** ADB exposes host port `11012` to device loopback port `11012`

#### Scenario: Forwarding cleanup
- **WHEN** a mobile conformance target stops or fails
- **THEN** its adapter removes the platform-specific same-number forwarding rule without disturbing another platform endpoint
