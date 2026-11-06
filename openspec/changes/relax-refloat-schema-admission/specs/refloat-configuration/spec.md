# Spec Delta

## MODIFIED Requirements

### Requirement: Bounded runtime custom-configuration schema

For every audited or validated limited connection, the backend and simulator SHALL retrieve bounded Qt-compressed custom-configuration index `0` through command 92 and normalize its actual metadata. Audited non-legacy admission SHALL follow the selected shared schema profile; limited connections SHALL match a known read profile without gaining write authority. INFO `1.0` SHALL retain exact full-fingerprint resolution to 1.0.0 or 1.0.1; an unknown legacy fingerprint SHALL remain unavailable.

#### Scenario: Valid Refloat 1.3.0 schema

- **WHEN** a supported Refloat 1.3.0 controller supplies the complete source-derived compressed schema with fingerprint `62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb` and wire signature `0x191a6c1b` in sequential chunks
- **THEN** the backend publishes the exact 172 serialized fields with their ordered groups, defaults, constraints, encodings, and computed identity

#### Scenario: Exact Refloat 1.3.1-beta2 schema

- **WHEN** audited 1.3.1-beta2 supplies its complete source-derived schema with fingerprint `6bc6bfa433f8205519bc931723e39252aa71c9f538a0ed7c200d688486c63e73` and wire signature `0x191a6c1b`
- **THEN** the backend exposes its 172 serialized fields, current and default values, and normal verified-write lifecycle without substituting the 1.3.0 metadata

#### Scenario: Same-version beta2 Tune

- **WHEN** a Tune saved against the exact beta2 configuration identity is applied to a beta2 board with current revision and all safety conditions satisfied
- **THEN** the existing confirmation, complete target-schema write, and mandatory readback rules apply; only Tunes outside its audited compatible family remain unavailable

#### Scenario: Valid dynamic schema

- **WHEN** a supported Refloat 1.1.x or 1.2.x controller supplies its complete audited source-derived compressed schema
- **THEN** the backend publishes that release's unchanged normalized schema and exact identity without substituting 1.3.0 fields

#### Scenario: Valid legacy schema

- **WHEN** a supported Refloat 1.0.x controller supplies its smaller source-derived schema
- **THEN** the schema fingerprint resolves exact source version 1.0.0 or 1.0.1 and the same owner normalizes only that release's fields without fabricating later parameters

#### Scenario: Legacy wire signature collision

- **WHEN** Refloat 1.0.0 and 1.0.1 have the same serialization signature but distinct audited full-schema metadata
- **THEN** their schema fingerprints and saved-configuration namespaces remain distinct

#### Scenario: Malicious expanded document

- **WHEN** a schema exceeds a compressed or expanded bound, declares a DTD/entity, exceeds structural limits, or contains invalid references
- **THEN** schema loading becomes unavailable before values or writes; bounded diagnostics disclose no raw document
- **AND** compressed/expanded bytes, chunks, retries, XML depth/elements, parameters, groups/subgroups, enum choices, names, descriptions and serialized fields remain bounded; DTDs/entities, duplicate fields/order entries, invalid references, unsupported types/encodings, invalid metadata/numbers and trailing data are rejected

#### Scenario: Unsupported package

- **WHEN** an unknown package has not passed public read admission or its syntactically valid schema matches no known read profile
- **THEN** configuration remains unavailable; parsing alone grants neither connection support nor write authority

#### Scenario: Preserve the actual normalized metadata

- **WHEN** a schema passes validation and its applicable admission policy
- **THEN** its identity contains the exact source package version, SHA-256 of complete validated expanded XML and computed wire signature
- **AND** its serialized fields retain order, names, labels, types, encodings, defaults, bounds, step, precision, scale, suffix, enum choices and sanitized descriptions, with actual General group/subgroup ordering

### Requirement: Configuration lifecycle and schema cache

Configuration state SHALL remain generation-scoped and independent of WebView lifetime. Cached raw schema bytes SHALL pass the same validation and profile admission as downloaded bytes. Cache keys/envelopes SHALL retain exact firmware, hardware, package, custom index and nonzero hardware-configuration CRC. Cache reuse SHALL NOT replace actual full-fingerprint identity or establish current-value/write authority.

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated while the native connection and loaded schema remain valid
- **THEN** the frontend can reopen the same normalized schema and current revision without owning protocol state

#### Scenario: Valid cache hit

- **WHEN** an exact connection identity and nonzero hardware CRC match a previously validated cached schema
- **THEN** the backend revalidates its bytes, recomputes full fingerprint, admission key and wire signature, checks the applicable profile and connection support mode, and proceeds to a fresh current-value read only with authoritative normal mode

#### Scenario: Corrupt cache

- **WHEN** cached bytes fail decompression, XML validation, signature reconstruction or selected-profile admission
- **THEN** that entry is discarded and the backend attempts a bounded command-92 fetch without enabling writes from the cache

#### Scenario: Refloat replacement

- **WHEN** reconnect identifies a different exact package version, full schema fingerprint or wire signature
- **THEN** the previous draft, values, schema pages, and pending operation cannot be used for the replacement connection

#### Scenario: Flywheel lifecycle

- **WHEN** telemetry enters flywheel mode and later confirms return to normal mode
- **THEN** loaded current values and their write authority are invalidated on entry and remain unavailable after exit until a fresh normal-mode command-93 read succeeds

#### Scenario: Missing or changed cache identity

- **WHEN** the CRC is absent/zero or any cache identity component changes, is invalid or has no matching entry
- **THEN** the backend performs a bounded schema fetch and grants no authority from the previous cached schema

#### Scenario: Cosmetic schema change has a new full identity

- **WHEN** a cosmetic schema variant passes admission after a cache miss or replacement
- **THEN** its current labels/help and full fingerprint become authoritative; old schema pages, draft bases, pending operations and Tunes cannot authorize writes under the previous identity

### Requirement: Exact beta3 configuration support

Exact Refloat 1.3.1-beta3 SHALL use its selected shared schema-admission profile and the normal bounded codec, cache, verified-write and Tune policies. Beta2, beta3 and stable 1.3.1 SHALL retain their audited shared `1.3.1` Tune family, with matching full fingerprint and wire signature. Supported stored data SHALL remain readable.

#### Scenario: Beta3 schema and verified writes

- **WHEN** beta3 supplies its audited 172-field schema and complete current/default values
- **THEN** configuration becomes available with beta3 identity; eligible edited writes and same-version portable Tune application require fresh authority and complete verified readback

#### Scenario: Matching schema does not authorize another release's Tune

- **WHEN** beta3 is connected and a saved unaudited beta4 Tune has the same schema fingerprint and wire signature
- **THEN** that Tune remains retained but is not offered or applied as compatible with beta3

#### Scenario: Audited beta and stable Tunes are interchangeable

- **WHEN** beta3 is connected and a saved beta2 Tune has the same schema fingerprint and wire signature
- **THEN** that Tune is presented as `1.3.1` and may be applied with normal confirmation, fresh authority and verified readback

#### Scenario: Reconnect invalidates current authority

- **WHEN** beta3 disconnects and reconnects
- **THEN** old revisions cannot authorize writes and fresh current-generation identity, timeout evidence and current values are required

### Requirement: Authoritative edited-write readiness reason

Every configuration summary SHALL include nullable `writeBlockReason`, identifying the highest-priority backend condition blocking an edited configuration write. Reasons SHALL be bounded closed codes shared across hosts. Existing capability-level `writable` and supported riding Tune admission SHALL retain their meanings.

#### Scenario: Actionable blocked state

- **WHEN** schema availability, connection readiness, read-only firmware, competing work, flywheel, motion, unknown telemetry, package state, package mode, or missing current values blocks an edited write
- **THEN** the complete snapshot reports the corresponding reason without the frontend guessing from generic writable state

#### Scenario: Ready state and recovery

- **WHEN** all backend edited-write preconditions are satisfied
- **THEN** the reason is null and subsequent snapshots update it atomically as readiness changes without sending a write

#### Scenario: Riding Tune compatibility

- **WHEN** riding blocks a general configuration write on a release supporting riding Tune application
- **THEN** the edited-write reason identifies riding without changing eligibility for that Tune application

#### Scenario: Readable unverified firmware

- **WHEN** a limited connection has readable configuration but lacks verified mutation semantics
- **THEN** the summary reports `unverifiedFirmware` when not superseded by discovery/loading/unavailable conditions, and the UI does not infer write or Tune permission from a successful read

## ADDED Requirements

### Requirement: Profile-based cosmetic schema admission

For a usable non-legacy connection, schema admission SHALL compare a known shared profile key that excludes only contents of text-only `ConfigParams/Params/<parameter>/longName` and `description` leaves. All other expanded XML bytes SHALL remain protected. The full fingerprint and wire signature SHALL remain the active schema identity. A match SHALL NOT grant mutation semantics to an unaudited release.

#### Scenario: Label or help edit

- **WHEN** a usable connection supplies a valid schema differing from its applicable profile only in those parameter text values
- **THEN** configuration reads and, on verified releases only, eligible writes remain available and publish the actual new labels/help and full fingerprint without requiring another full-fingerprint catalogue entry

#### Scenario: Same CRC with changed semantics

- **WHEN** a schema changes a field identity/order, type/encoding, scale, bounds, default, step, precision, units, enum choices, grouping, an ignored codec property such as `maxLen`, or any other protected XML byte
- **THEN** profile admission rejects it even if the wire CRC is unchanged; a source audit and explicit profile update are required before values or writes become available

#### Scenario: Excluded text still validates

- **WHEN** a proposed label/help edit exceeds limits, introduces prohibited XML or uses a nested-element shape instead of a supported text-only leaf
- **THEN** the schema fails validation or admission before any value load/write; matching the protected remainder cannot bypass validation

#### Scenario: Lookalike elements stay protected

- **WHEN** XML changes a `description` or `longName` outside the specified parameter-leaf paths, changes an attribute/tag, or inserts/deletes an element
- **THEN** the change is not hidden by the cosmetic exception and fails the existing selected-profile match

#### Scenario: Full identity still isolates saved work

- **WHEN** an accepted cosmetic variant has the same release and wire signature but a different full fingerprint
- **THEN** saved Tunes and editor drafts for the previous full identity remain preserved but cannot be automatically applied, migrated or treated as current

#### Scenario: Configuration rejection remains local

- **WHEN** an admitted controller has a valid INFO/telemetry profile but its schema fails selected-profile admission
- **THEN** configuration remains unavailable without values or configuration-write traffic while independently admitted telemetry and controls retain their existing policy

### Requirement: Limited configuration reads without write authority

A limited connection matching a known shared schema profile SHALL expose the actual schema and validated default/current reads under existing mode and value rules. Configuration failure SHALL remain local to configuration. Matching a profile, wire signature or full fingerprint SHALL NOT authorize device writes, Tune application or controls.

#### Scenario: Unknown version with a familiar schema

- **WHEN** an unknown version has validated public telemetry and a fully valid schema matches a known read profile
- **THEN** schema/default reads become available and current reads require fresh authoritative normal mode; all payloads retain exact signature, extent, type and bounds validation
- **AND** the actual observed version/full fingerprint is published, configuration remains non-writable, riding-Tune support is false and board write/apply requests fail without mutation traffic

#### Scenario: Unknown or flywheel mode

- **WHEN** a limited connection has unknown or flywheel package mode
- **THEN** current reads cannot advance an authoritative revision or supply saved-current values, while validated schema/default availability may remain usable

#### Scenario: Configuration is incompatible

- **WHEN** a limited connection has usable core telemetry but its schema cannot be fetched or admitted, or its value payload is invalid
- **THEN** no invalid configuration result or write authority is published and core telemetry/recording continue independently

#### Scenario: Cache is not permission

- **WHEN** cached XML matches a limited connection's exact cache identity and passes validation
- **THEN** read compatibility is recomputed and no cached verified-release decision or previous generation's write authority is reused

#### Scenario: Saved identity remains exact

- **WHEN** configuration is read or saved from a limited version or cosmetic full-fingerprint variant
- **THEN** existing exact identity and fresh-current rules govern app-local data operations, stored Tunes remain preserved and no conversion or application outside the audited Tune family is enabled
