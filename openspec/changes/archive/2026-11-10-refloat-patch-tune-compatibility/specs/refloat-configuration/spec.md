## MODIFIED Requirements

### Requirement: Bounded runtime custom-configuration schema

For each audited stable Refloat 1.0.0–1.3.0 or exact 1.3.1-beta2 connection, the native backend and conformance simulator SHALL discover custom configuration index `0`, retrieve its Qt-compressed `ConfigParams` document through source-correct VESC command 92 chunks, and normalize the exact active schema. The normalized schema SHALL contain the exact source package version, SHA-256 fingerprint of canonical validated full-schema metadata, computed wire configuration signature, ordered writable parameter identity, supported type and wire encoding, default, bounds, precision, scale, suffix, enum choices, sanitized description, and General group/subgroup ordering. Refloat wire INFO `1.0` SHALL resolve to exact 1.0.0 or 1.0.1 only by matching this fingerprint to the corresponding immutable audited schema; later exact INFO versions SHALL also require their matching audited fingerprint. Compressed and expanded bytes, chunks, retries, XML depth/elements, parameters, groups, enum choices, names, descriptions, and serialized fields MUST be bounded. DTDs, entities, duplicate names or order entries, missing referenced parameters, unsupported types or encodings, invalid metadata, trailing data, and unaudited fingerprints MUST fail closed for configuration.

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
- **THEN** their schema fingerprints remain distinct while audited stable-minor Tunes may be shared with target-schema validation

#### Scenario: Malicious expanded document

- **WHEN** a schema exceeds a compressed or expanded bound, declares a DTD/entity, exceeds structural limits, or contains invalid references
- **THEN** schema loading becomes unavailable, no configuration values or write are accepted, and bounded diagnostics disclose no raw document

#### Scenario: Unsupported package

- **WHEN** an unaudited package, including another prerelease or stable 1.4.0, supplies a syntactically valid custom schema
- **THEN** Float View does not use runtime parsing to broaden compatibility or enable configuration

### Requirement: Exact-version saved configurations

Native persistence SHALL store bounded revisioned named Tunes with opaque ID, optional description, package name, Tune-family version, full-schema fingerprint, wire signature, typed values and timestamps. List/apply/overwrite SHALL match supported stable minor, audited schema equivalence and wire signature. Cross-minor and unknown-schema Tunes SHALL remain isolated. Stored stable versions and fingerprints SHALL remain unchanged. Create/update/delete SHALL retain revision-conflict rules.

#### Scenario: Apply Tune while riding

- **WHEN** a supported Refloat 1.1.0 or later board is running in normal mode with a known riding state and a valid fresh-base Tune application is requested
- **THEN** the shared service permits the write with engaged footpads and reports success only after verified readback

#### Scenario: Older Refloat retains disengaged application

- **WHEN** Tune application is requested on Refloat 1.0.0 or 1.0.1 while riding
- **THEN** the shared service rejects the request before scheduler submission

#### Scenario: Save current configuration

- **WHEN** a user saves a complete validated Refloat 1.3.0 value set under a bounded name
- **THEN** native storage creates one revisioned record keyed to exact version 1.3.0, its full schema fingerprint, and wire signature

#### Scenario: Exact-version list

- **WHEN** Tunes are opened for Refloat 1.1.2
- **THEN** applicable results contain bounded compatible records from stable 1.1.x with the active signature

#### Scenario: Package upgrade

- **WHEN** a board changes from Refloat 1.2.3 to 1.3.0
- **THEN** 1.2.3 configurations are not applied, migrated, or shown as compatible with 1.3.0

#### Scenario: Revision conflict

- **WHEN** a saved configuration update supplies a stale record revision
- **THEN** the backend returns `conflict` and leaves the durable record unchanged

### Requirement: Exact beta3 configuration support

The backend and simulator SHALL additionally support exact Refloat 1.3.1-beta3 with its audited full-schema fingerprint and wire signature using the existing bounded schema/value codec, configuration write/readback, cache and Tune policies. Beta2, beta3 and stable 1.3.1 SHALL share the audited `1.3.1` Tune family, with matching wire signature and audited schema equivalence; this family extends across supported stable 1.3.x. Supported stored data SHALL remain readable.

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

### Requirement: Stable Refloat 1.3.1 configuration

Exact stable 1.3.1 SHALL use the audited beta3 schema fingerprint, wire signature and bounded configuration lifecycle. Its cache and configuration drafts SHALL retain exact observed versions. Tunes for audited beta2, beta3 and stable SHALL share canonical version `1.3.1`, while matching package, wire signature and audited schema equivalence across stable 1.3.x.

#### Scenario: Stable schema and Tune

- **WHEN** stable 1.3.1 supplies the audited 172-field schema
- **THEN** current/default reads, safe writes, verified readback and compatible-family Tunes are available
- **AND** existing beta Tunes remain readable, listable, editable and applicable on all three audited versions without data migration

### Requirement: Bounded Tune family compatibility

Stable patches within each supported 1.0.x–1.3.x minor SHALL share Tunes across audited schemas with equal wire signatures. Exact beta2/beta3 SHALL join 1.3.x. Stored stable versions and fingerprints SHALL remain unchanged; existing beta public-version normalization SHALL remain. Filtering and pagination SHALL use the same compatibility policy as apply and overwrite. Complete portable values MUST validate against the target schema before mutation.

#### Scenario: Existing beta Tune on stable

- **WHEN** a supported stored beta2 or beta3 Tune is listed, read, updated or applied on stable 1.3.1
- **THEN** its public Tune version is `1.3.1` and family matching succeeds only with matching wire signature and audited schema equivalence
- **AND** fresh current revision, confirmation, mutation safety and verified readback remain required

#### Scenario: Incompatible neighbor

- **WHEN** beta1, beta4, another minor, an unknown mismatched schema or a mismatched signature is supplied
- **THEN** the Tune remains incompatible without a device write

### Requirement: Stable minor configuration admission

Supported stable patches SHALL accept audited full-schema fingerprints and wire signatures from their stable minor family, replacing exact patch matching. Unknown schemas and cross-minor schemas SHALL fail closed for configuration. Configuration identity and caches SHALL retain the exact observed patch. Legacy INFO 1.0 SHALL retain fingerprint-selected source identity. Exact prerelease schema admission SHALL remain unchanged; saved Tunes SHALL follow stable-minor Tune compatibility.

#### Scenario: Later patch reuses known schema

- **WHEN** stable 1.2.255 or 1.3.2 supplies an audited schema from its own stable minor
- **THEN** configuration reads and guarded writes are available with the observed patch version and unchanged full fingerprint/signature
- **AND** reconnect and cached-schema validation apply the same policy

#### Scenario: Schema validation remains local and strict

- **WHEN** a supported patch supplies an unknown fingerprint or signature, a different minor's schema or malformed XML
- **THEN** configuration remains unavailable without broadening admission from the version guarantee alone

#### Scenario: Tune identity remains exact

- **WHEN** a stable patch uses a saved Tune from another patch of its supported minor
- **THEN** the Tune retains its exact saved version and fingerprint while compatibility follows the audited stable-minor family

#### Scenario: Cross-patch Tune application

- **WHEN** a 1.3.0 Tune is opened on 1.3.1 or a later supported stable 1.3.x board with an audited schema and equal wire signature, or the direction is reversed
- **THEN** it is available for review, confirmed application and overwrite without rewriting its saved identity
- **AND** the target's exact configuration identity authorizes a complete target-schema write with fresh-base checks and mandatory readback

#### Scenario: Preserved data and bounded pagination

- **WHEN** existing patch-labelled Tunes are listed in small pages, overwritten or exported and imported
- **THEN** compatible records are neither omitted nor duplicated and IDs, source identities, values and timestamps remain intact except for explicit user edits

#### Scenario: Invalid portable values

- **WHEN** a compatible-family Tune has missing, extra, mistyped or out-of-range target fields
- **THEN** apply and overwrite reject it without device traffic or a partial saved update
