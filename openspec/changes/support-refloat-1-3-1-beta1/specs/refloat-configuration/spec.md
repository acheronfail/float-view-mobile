## MODIFIED Requirements

### Requirement: Bounded runtime custom-configuration schema

For each audited stable Refloat 1.0.0–1.3.0 or exact 1.3.1-beta1 connection, the native backend and conformance simulator SHALL discover custom configuration index `0`, retrieve its Qt-compressed `ConfigParams` document through source-correct VESC command 92 chunks, and normalize the exact active schema. The normalized schema SHALL contain the exact source package version, SHA-256 fingerprint of canonical validated full-schema metadata, computed wire configuration signature, ordered writable parameter identity, supported type and wire encoding, default, bounds, precision, scale, suffix, enum choices, sanitized description, and General group/subgroup ordering. Refloat wire INFO `1.0` SHALL resolve to exact 1.0.0 or 1.0.1 only by matching this fingerprint to the corresponding immutable audited schema; later exact INFO versions SHALL also require their matching audited fingerprint. Compressed and expanded bytes, chunks, retries, XML depth/elements, parameters, groups, enum choices, names, descriptions, and serialized fields MUST be bounded. DTDs, entities, duplicate names or order entries, missing referenced parameters, unsupported types or encodings, invalid metadata, trailing data, and unaudited fingerprints MUST fail closed for configuration.

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

- **WHEN** an unknown release or a prerelease other than exact audited 1.3.1-beta1 supplies a syntactically valid custom schema
- **THEN** Float View does not use runtime parsing to broaden compatibility or enable configuration

## ADDED Requirements

### Requirement: Exact beta configuration identity

Beta configuration SHALL require exact package identity `1.3.1-beta1`, its source-derived full-schema fingerprint and computed wire signature. Matching 1.3.0 serialization alone SHALL NOT authorize the beta schema. Configuration reads, writes, verified readback, cache keys and saved Tunes SHALL preserve the suffix and all existing safety, lifecycle and size bounds.

#### Scenario: Audited beta schema

- **WHEN** an admitted beta supplies its complete audited schema and matching values
- **THEN** the normalized metadata reflects that exact source and eligible writes succeed only after mandatory readback

#### Scenario: Same wire signature with different metadata

- **WHEN** beta and stable schemas share a serialization signature but have different full metadata
- **THEN** each requires its own fingerprint and exact package identity and neither borrows the other's cached authority

#### Scenario: Tune version isolation

- **WHEN** Tunes are listed or applied for 1.3.1-beta1
- **THEN** only records matching its full exact identity qualify; 1.2.x, 1.3.0 and other betas are not converted, rewritten or offered as compatible

#### Scenario: Package replacement during a write

- **WHEN** reconnect or route replacement changes the exact package or schema before verified readback
- **THEN** the pending write cannot succeed and the old draft and current revision cannot authorize a new write

#### Scenario: Existing saved Tunes

- **WHEN** beta support is installed over supported owned storage
- **THEN** existing Tunes and baseline archive imports remain intact and usable with their original exact releases
