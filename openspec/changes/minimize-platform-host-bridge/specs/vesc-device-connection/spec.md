## ADDED Requirements

### Requirement: VESC firmware UUID is the stable saved-board identity

After bounded UART setup and a valid VESC firmware response, Rust SHALL use the meaningful VESC firmware device UUID bytes themselves as the stable saved-board logical identity for now. Rust SHALL parse and validate the fixed-width firmware field, encode it as 24 lowercase hexadecimal characters for application surfaces, and use that value directly for both `savedDeviceId` and `deviceHistoryId`. An all-zero or otherwise unavailable value lacks meaningful identity and uses the existing random fallback. Rust MUST NOT substitute a per-process discovery token or derived UUID hash as durable board identity. Apple peripheral UUIDs and Android BLE addresses SHALL remain private local transport associations used only to rediscover and connect before firmware identity is available.

#### Scenario: First meaningful firmware UUID

- **WHEN** a saved-mode connection returns a valid non-zero VESC firmware device UUID
- **THEN** Rust uses those exact UUID bytes as the stable saved-board logical key and their canonical hexadecimal representation as `savedDeviceId` and `deviceHistoryId`, attaches the private platform transport association, and does not derive identity from the runtime discovery handle

#### Scenario: Process recreation

- **WHEN** a saved VESC is reconnected after the native process and Rust runtime are recreated and returns the same firmware UUID
- **THEN** Rust resolves the existing saved board by those UUID bytes without heuristic matching, random-token migration, or a duplicate board

#### Scenario: Reconnect generation

- **WHEN** a UUID-identified connection fails and reconnects through its private platform association
- **THEN** Rust assigns a fresh connection generation, revalidates the firmware UUID, and retains the selected saved or guest mode only for the matching device

#### Scenario: Firmware UUID unavailable

- **WHEN** an otherwise valid firmware response contains an all-zero or unavailable device UUID
- **THEN** Rust follows the existing bounded no-meaningful-identity fallback for saved mode, makes no cross-device identity claim, and never promotes the discovery handle or platform association to logical board identity

### Requirement: VESC UUID is shared application metadata

The canonical VESC firmware/controller/BMS UUID MAY be used anywhere application identity or correlation requires it, including Rust firmware parsing, routing, saved-board identity, frontend state, local persistence, logs, bounded diagnostics, portable archives, exported ride metadata, fixtures, and conformance. Every representation of the same meaningful UUID SHALL use the same 24-character lowercase hexadecimal encoding. Platform transport associations remain available only to the matching native adapter, private bridge, and private local association and MUST NOT be substituted for VESC identity.

#### Scenario: UUID-bearing connection lifecycle

- **WHEN** discovery, connection, failure, persistence, diagnostics, export, and archive operations execute for a UUID-associated board
- **THEN** public and diagnostic outputs may contain the canonical VESC UUID but contain neither the Apple peripheral UUID nor the Android BLE address in byte or textual form

#### Scenario: Forget saved board

- **WHEN** the user successfully forgets an inactive saved board
- **THEN** Rust removes its saved UUID identity and board-owned metadata and does not affect another board
