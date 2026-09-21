## MODIFIED Requirements

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
