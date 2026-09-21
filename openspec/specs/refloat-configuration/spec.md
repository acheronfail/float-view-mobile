# Refloat Configuration Specification

## Purpose

Define bounded, exact-version discovery, editing, persistence, and verification of Refloat custom configuration.

## Requirements
### Requirement: Bounded runtime custom-configuration schema

For each audited stable Refloat 1.0.0–1.3.0 connection, the native backend and conformance simulator SHALL discover custom configuration index `0`, retrieve its Qt-compressed `ConfigParams` document through source-correct VESC command 92 chunks, and normalize the exact active schema. The normalized schema SHALL contain the exact source package version, SHA-256 fingerprint of canonical validated full-schema metadata, computed wire configuration signature, ordered writable parameter identity, supported type and wire encoding, default, bounds, precision, scale, suffix, enum choices, sanitized description, and General group/subgroup ordering. Refloat wire INFO `1.0` SHALL resolve to exact 1.0.0 or 1.0.1 only by matching this fingerprint to the corresponding immutable audited schema; later exact INFO versions SHALL also require their matching audited fingerprint. Compressed and expanded bytes, chunks, retries, XML depth/elements, parameters, groups, enum choices, names, descriptions, and serialized fields MUST be bounded. DTDs, entities, duplicate names or order entries, missing referenced parameters, unsupported types or encodings, invalid metadata, trailing data, and unaudited fingerprints MUST fail closed for configuration.

#### Scenario: Valid Refloat 1.3.0 schema

- **WHEN** a supported Refloat 1.3.0 controller supplies the complete source-derived compressed schema with fingerprint `62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb` and wire signature `0x191a6c1b` in sequential chunks
- **THEN** the backend publishes the exact 172 serialized fields with their ordered groups, defaults, constraints, encodings, and computed identity

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
- **THEN** schema loading becomes unavailable, no configuration values or write are accepted, and bounded diagnostics disclose no raw document

#### Scenario: Unsupported package

- **WHEN** an unknown or prerelease Refloat package supplies a syntactically valid custom schema
- **THEN** Float View does not use runtime parsing to broaden compatibility or enable configuration

### Requirement: Exact signed value codec

The native backends and simulator SHALL decode current command 93 and default command 94 responses only against the active normalized schema. A command-93 current read MUST be issued and accepted only while authoritative package mode is normal; entering flywheel SHALL invalidate any loaded current values, and no current response observed during flywheel may advance a revision or become an editable or saved source. Schema reads and command-94 default reads MAY remain available in flywheel because they do not serialize Refloat's temporary in-memory calibration. Each accepted response MUST contain the exact CRC32C configuration signature, one complete value in serialization order using the declared VESC wire encoding, and no remainder. Writes SHALL serialize the same signature followed by a complete validated value set. Numeric values MUST be finite, within declared bounds, and encodable; integers, enums, booleans, bitfields, and strings MUST satisfy their exact type and bounds. Unknown, missing, duplicate, stale, or extra values MUST be rejected before a command is sent.

#### Scenario: Current values

- **WHEN** command 93 returns a complete payload matching the active schema signature
- **THEN** the backend returns one typed current value for every serialized field and advances the connection-local current revision

#### Scenario: Package defaults

- **WHEN** command 94 returns a complete payload matching the active schema signature
- **THEN** the backend returns one typed default value for every serialized field without changing the device or current-value revision

#### Scenario: Current values requested during flywheel

- **WHEN** authoritative package mode is flywheel and current configuration is requested or a late command-93 response arrives
- **THEN** the backend returns `invalid_state` or discards the response, keeps current values unloaded, and sends or accepts no command-93 current snapshot

#### Scenario: Defaults requested during flywheel

- **WHEN** authoritative package mode is flywheel and package defaults are requested
- **THEN** command 94 may return validated immutable defaults without creating current write authority

#### Scenario: Signature mismatch

- **WHEN** a values response or proposed write identifies a different schema signature
- **THEN** the operation fails closed without partially decoding, updating authoritative values, or sending a write

#### Scenario: Invalid proposed value

- **WHEN** a draft contains a non-finite number, out-of-range value, invalid enum, oversized string, missing serialized field, or unknown field
- **THEN** the complete write is rejected as invalid without board traffic

### Requirement: Safe persistent write with mandatory readback

A Refloat configuration write SHALL require the active ready read/write connection generation, selected route, exact supported package version, matching schema signature, latest normal-mode current-value base revision, authoritatively normal package mode, exactly idle ride state, no engaged footpad, and no competing Refloat mutation. While flywheel mode is active the centralized mutation authority MUST reject command 95 and Tune apply before scheduler submission; the only permitted device mutation is an eligible semantic command 22 that disables flywheel. The backend SHALL validate and serialize the complete draft, submit command 95 through the connection-wide request scheduler, treat its acknowledgement as non-authoritative, then reread command 93 and compare every canonical value after documented firmware normalization. Success SHALL be returned only after a complete match. Timeout, rejection, disconnect, route or generation replacement, malformed readback, or mismatch MUST NOT return success.

#### Scenario: Verified direct write

- **WHEN** an eligible direct controller acknowledges a valid full configuration and command 93 reads back the expected canonical values
- **THEN** the request succeeds, publishes the newer complete current values, and reports no pending operation

#### Scenario: Verified CAN write

- **WHEN** an eligible selected CAN controller receives the same operation
- **THEN** commands 95 and 93 are wrapped exactly once for the selected CAN ID and only matching-route readback can complete the write

#### Scenario: Acknowledged but rejected write

- **WHEN** command 95 is acknowledged but Refloat rejects or ignores the configuration
- **THEN** the required readback mismatch returns a bounded failure and Float View replaces any stale current values with the newly observed device state

#### Scenario: Unsafe board state

- **WHEN** a write is requested while riding, a footpad is engaged, package mode is flywheel or otherwise not normal, firmware is read-only, or another Refloat mutation is pending
- **THEN** the backend returns `invalid_state`, sends no command 95, and preserves current values

#### Scenario: Indeterminate interruption

- **WHEN** the device disconnects after command 95 but before verified readback
- **THEN** the request fails without claiming success and a fresh current read is required before any later write

### Requirement: Exact-version saved configurations

Native persistence SHALL store bounded application-wide named complete Refloat configurations with opaque ID, revision, name, optional description, package name, exact package version, full-schema fingerprint, wire signature, complete typed values, and timestamps. List and apply workflows SHALL match the active exact package version, fingerprint, and signature; configurations MUST NOT be automatically migrated, merged, or offered as applicable across versions. Create or update from device current values SHALL require a fresh authoritative normal-mode current revision and MUST reject stale or flywheel-temporary values. Create, update, and delete SHALL otherwise use bounded values and revision conflict semantics. Applying a saved configuration SHALL pass through the same fresh-base, safety, serialization, scheduler, and readback lifecycle as an edited draft.

#### Scenario: Save current configuration

- **WHEN** a user saves a complete validated Refloat 1.3.0 value set under a bounded name
- **THEN** native storage creates one revisioned record keyed to exact version 1.3.0, its full schema fingerprint, and wire signature

#### Scenario: Exact-version list

- **WHEN** Tunes are opened for Refloat 1.1.2
- **THEN** applicable results contain only bounded records for exact version 1.1.2 and the active signature

#### Scenario: Package upgrade

- **WHEN** a board changes from Refloat 1.2.3 to 1.3.0
- **THEN** 1.2.3 configurations are not applied, migrated, or shown as compatible with 1.3.0

#### Scenario: Revision conflict

- **WHEN** a saved configuration update supplies a stale record revision
- **THEN** the backend returns `conflict` and leaves the durable record unchanged

### Requirement: Configuration lifecycle and schema cache

Configuration state SHALL be connection-generation scoped and independent of WebView lifetime. A valid nonzero firmware hardware-configuration CRC MAY key a bounded native schema cache together with exact controller firmware, package version, and custom-config index; cache contents MUST pass the same schema/signature validation as downloaded bytes. Missing, zero, changed, corrupt, or mismatched cache identity SHALL cause a bounded device fetch. Reconnect or entry into flywheel SHALL make current values unloaded and writes unavailable. Confirmed return from flywheel to normal mode SHALL require a fresh command-93 read before editing from current, saving current, command 95, or Tune apply; a replacement version or signature SHALL invalidate the old draft and pending operation.

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated while the native connection and loaded schema remain valid
- **THEN** the frontend can reopen the same normalized schema and current revision without owning protocol state

#### Scenario: Valid cache hit

- **WHEN** an exact connection identity and nonzero hardware CRC match a previously validated cached schema
- **THEN** the backend may use it after recomputing its signature and proceeds to a fresh current-value read

#### Scenario: Corrupt cache

- **WHEN** cached bytes fail decompression, XML validation, or signature reconstruction
- **THEN** that entry is discarded and the backend attempts a bounded command-92 fetch without enabling writes from the cache

#### Scenario: Refloat replacement

- **WHEN** reconnect identifies a different exact package version or schema signature
- **THEN** the previous draft, values, schema pages, and pending operation cannot be used for the replacement connection

#### Scenario: Flywheel lifecycle

- **WHEN** telemetry enters flywheel mode and later confirms return to normal mode
- **THEN** loaded current values and their write authority are invalidated on entry and remain unavailable after exit until a fresh normal-mode command-93 read succeeds
