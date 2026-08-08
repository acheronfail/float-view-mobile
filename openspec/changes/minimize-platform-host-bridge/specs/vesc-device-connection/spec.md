## ADDED Requirements

### Requirement: VESC firmware UUID is the stable saved-board identity
After bounded UART setup and a valid VESC firmware response, Rust SHALL use the meaningful VESC firmware device UUID bytes themselves as the stable saved-board logical identity for now. Rust SHALL parse and validate the fixed-width firmware field, treat the all-zero or otherwise unavailable value as lacking meaningful identity, and MUST NOT substitute a per-process discovery token as durable board identity. Apple peripheral UUIDs and Android BLE addresses SHALL remain private local transport associations used only to rediscover and connect before firmware identity is available.

#### Scenario: First meaningful firmware UUID
- **WHEN** a saved-mode connection returns a valid non-zero VESC firmware device UUID
- **THEN** Rust uses those exact UUID bytes as the stable saved-board logical key, attaches the private platform transport association, and does not derive identity from the runtime discovery handle

#### Scenario: Process recreation
- **WHEN** a saved VESC is reconnected after the native process and Rust runtime are recreated and returns the same firmware UUID
- **THEN** Rust resolves the existing saved board by those UUID bytes without heuristic matching, random-token migration, or a duplicate board

#### Scenario: Reconnect generation
- **WHEN** a UUID-identified connection fails and reconnects through its private platform association
- **THEN** Rust assigns a fresh connection generation, revalidates the firmware UUID, and retains the selected saved or guest mode only for the matching device

#### Scenario: Firmware UUID unavailable
- **WHEN** an otherwise valid firmware response contains an all-zero or unavailable device UUID
- **THEN** Rust follows the existing bounded no-meaningful-identity fallback for saved mode, makes no cross-device identity claim, and never promotes the discovery handle or platform association to logical board identity

### Requirement: VESC UUID remains private native metadata
The VESC firmware UUID SHALL be available only to Rust firmware parsing, saved-board identity resolution, and private durable metadata. Platform transport associations SHALL be available only to the matching native adapter, private bridge, and private local association. Neither identifier SHALL appear in ordinary logs, bounded diagnostics, frontend state, portable archives, exported ride metadata, or public `deviceHistoryId`; those surfaces SHALL retain their existing privacy-preserving identities.

#### Scenario: UUID-bearing connection lifecycle
- **WHEN** discovery, connection, failure, persistence, diagnostics, export, and archive operations execute for a UUID-associated board
- **THEN** none of their public or diagnostic outputs contain the firmware UUID, Apple peripheral UUID, or Android BLE address in byte or textual form

#### Scenario: Forget saved board
- **WHEN** the user successfully forgets an inactive saved board
- **THEN** Rust removes its local UUID association along with the board-owned private metadata and does not affect another board
