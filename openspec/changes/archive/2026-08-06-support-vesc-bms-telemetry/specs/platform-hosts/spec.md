## ADDED Requirements

### Requirement: Mirrored native BMS ownership

Apple and Android SHALL implement matching owners for connection-wide VESC request scheduling, common and hardware-specific firmware identity parsing, auxiliary-node discovery, bounded BMS values decoding, and slow BMS polling. Application coordination SHALL remain above protocol routing and Bluetooth, and no BMS protocol owner SHALL depend on WebSocket, frontend, persistence, or view code.

#### Scenario: Cross-platform source layout

- **WHEN** a maintainer traces BMS discovery or decoding on either backend
- **THEN** the matching Swift and Kotlin owner is available under the established application/backend and `protocol/vesc` trees with equivalent bounds and state transitions

#### Scenario: One write owner

- **WHEN** controller and BMS work both need the VESC stream
- **THEN** each platform routes typed requests through its one connection-wide scheduler rather than writing from independent polling timers

#### Scenario: Hand-written contract mapping

- **WHEN** native BMS state is serialized for the frontend
- **THEN** Apple and Android explicitly map every bounded contract field with equivalent null, unit, array, and status semantics and no generated API layer

### Requirement: Native lifecycle and privacy boundary

Native services SHALL continue auxiliary discovery and BMS polling while the frontend is disposable and while background execution is permitted, cancel every BMS generation and queued request on connection replacement, and exclude BMS UUIDs and full BMS telemetry from default logs and durable files.

#### Scenario: WebView disposal

- **WHEN** the frontend is destroyed while a selected BMS remains attached
- **THEN** native polling continues where the OS permits and a replacement frontend recovers the latest authoritative BMS state

#### Scenario: Connection generation changes

- **WHEN** disconnect, reconnect, preference change, or device replacement invalidates auxiliary work
- **THEN** late callbacks and queued work from the older generation cannot select a route or publish a snapshot

#### Scenario: Default diagnostics

- **WHEN** BMS discovery, polling, timeout, or malformed input is diagnosed normally
- **THEN** logs retain only bounded high-level route/status codes and do not contain UUIDs, full cell arrays, or complete raw telemetry frames
