## ADDED Requirements

### Requirement: Authoritative active device kind

Every non-null active connection snapshot SHALL contain required `deviceKind` equal to `null` while the direct endpoint is not yet classified, `board` when a motor-controller route owns the board experience, or `bms` when an exact directly connected supported VBMS owns a BMS-only experience. The value MUST be generation-scoped and MUST NOT be inferred by the frontend from display names, platform identifiers, firmware labels, nullable controller fields, or BMS status.

#### Scenario: Identification is pending

- **WHEN** a connection exists before direct firmware classification completes
- **THEN** its complete authoritative snapshot contains `deviceKind: null`

#### Scenario: Direct VBMS is classified

- **WHEN** the direct firmware identity matches the exact supported VBMS catalog
- **THEN** the same generation publishes `deviceKind: bms` before or with BMS-only readiness

#### Scenario: Controller route is established

- **WHEN** a direct or CAN-routed motor controller becomes the selected board controller
- **THEN** the active connection publishes `deviceKind: board`

#### Scenario: Stale classification arrives

- **WHEN** firmware or polling work from an old generation completes after disconnect or reconnect
- **THEN** it cannot change the current active connection's device kind
