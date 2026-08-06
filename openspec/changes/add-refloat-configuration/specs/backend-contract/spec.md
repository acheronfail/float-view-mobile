## ADDED Requirements

### Requirement: Bounded Refloat configuration contract

Every supported backend SHALL implement mandatory `refloat.configuration.open`, `refloat.configuration.schema`, `refloat.configuration.read`, and `refloat.configuration.write` methods plus `refloat.tune.list`, `refloat.tune.get`, `refloat.tune.create`, `refloat.tune.update`, `refloat.tune.delete`, and `refloat.tune.apply`. Open SHALL return the small active identity/status summary; schema SHALL return one cursor-bounded group/subgroup page; read SHALL accept exactly `current` or `default`; write SHALL accept exact package version, schema signature, base current revision, and a complete bounded typed value map. Tune mutation methods SHALL use opaque IDs and revisions. All results, fields, strings, maps, pages, cursors, and errors MUST fit documented bounds and use the stable error vocabulary.

#### Scenario: Supported backend

- **WHEN** the shared frontend uses Refloat configuration on mock, Android, iOS, or macOS
- **THEN** every mandatory method has identical request, result, bounds, revision, and error semantics

#### Scenario: Paged schema

- **WHEN** a normalized schema and its descriptions would exceed one control message
- **THEN** the frontend retrieves deterministic bounded pages using opaque cursors without receiving raw XML or compressed bytes

#### Scenario: Stale write

- **WHEN** write package identity, signature, connection generation, or base current revision is stale
- **THEN** the backend returns `conflict` or `invalid_state` without sending a board command

#### Scenario: Oversized values or metadata

- **WHEN** any configuration or Tune request/result exceeds its field, string, collection, cursor, or encoded-message limit
- **THEN** the backend rejects it before unbounded parsing, allocation, persistence, or Bluetooth work

### Requirement: Small authoritative configuration availability

`ActiveConnection.refloatConfiguration` SHALL be null before Refloat discovery begins and otherwise contain only bounded status from `loading`, `ready`, or `unavailable`; writability; nullable exact package version until legacy schema identity resolves; nullable schema fingerprint and wire signature; current/default-loaded flags; and nullable pending operation from `schema`, `readCurrent`, `readDefault`, `write`, or `applyTune`. It MUST NOT contain schema fields, raw XML, raw values, saved configurations, private controller identity, or cache keys. Entry into flywheel mode SHALL clear the current-loaded flag and writability without discarding validated schema/default availability; confirmed exit SHALL not restore them until a fresh current read succeeds. Availability and pending-operation changes SHALL publish newer complete authoritative snapshots, while bulk schema/value results remain request-scoped.

#### Scenario: Ready configuration

- **WHEN** schema validation and a fresh current read complete for a ready writable connection
- **THEN** the next authoritative snapshot reports ready and writable with matching version/signature and loaded current values

#### Scenario: Reconnecting connection

- **WHEN** the active device reconnects
- **THEN** configuration becomes loading and non-writable while retained bulk request results cannot authorize a write

#### Scenario: Slow frontend

- **WHEN** telemetry snapshots continue while a schema page or value request is outstanding
- **THEN** authoritative state remains bounded and conflated without embedding or repeatedly delivering the bulk result

#### Scenario: Flywheel configuration contract

- **WHEN** authoritative package mode is flywheel
- **THEN** `read(current)`, `write`, and `tune.apply` return `invalid_state`, schema/default reads may remain available, and the summary reports no loaded authoritative current values
