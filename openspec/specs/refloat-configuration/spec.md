# Refloat Configuration Specification

## Purpose

Define bounded, exact-version discovery, editing, persistence, and verification of Refloat custom configuration.

## Requirements

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
- **THEN** their schema fingerprints and saved-configuration namespaces remain distinct

#### Scenario: Malicious expanded document

- **WHEN** a schema exceeds a compressed or expanded bound, declares a DTD/entity, exceeds structural limits, or contains invalid references
- **THEN** schema loading becomes unavailable, no configuration values or write are accepted, and bounded diagnostics disclose no raw document

#### Scenario: Unsupported package

- **WHEN** an unaudited package, including another prerelease or stable 1.3.2, supplies a syntactically valid custom schema
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

A Refloat configuration write SHALL require the active ready read/write connection generation, selected route, exact supported package version, matching schema signature, latest normal-mode current-value base revision, authoritatively normal package mode, exactly idle ride state, and no competing Refloat mutation. While flywheel mode is active the centralized mutation authority MUST reject command 95 and Tune apply before scheduler submission; the only permitted device mutation is an eligible semantic command 22 that disables flywheel. The backend SHALL validate and serialize the complete draft, submit command 95 through the connection-wide request scheduler, treat its acknowledgement as non-authoritative, then reread command 93 and compare every canonical value after documented firmware normalization. Success SHALL be returned only after a complete match. Timeout, rejection, disconnect, route or generation replacement, malformed readback, or mismatch MUST NOT return success.

#### Scenario: Verified direct write

- **WHEN** an eligible direct controller acknowledges a valid full configuration and command 93 reads back the expected canonical values
- **THEN** the request succeeds, publishes the newer complete current values, and reports no pending operation

#### Scenario: Verified CAN write

- **WHEN** an eligible selected CAN controller receives the same operation
- **THEN** commands 95 and 93 are wrapped exactly once for the selected CAN ID and only matching-route readback can complete the write

#### Scenario: Acknowledged but rejected write

- **WHEN** command 95 is acknowledged but Refloat rejects or ignores the configuration
- **THEN** the required readback mismatch returns a bounded failure and Float View replaces any stale current values with the newly observed device state

#### Scenario: Idle write with footpad contact

- **WHEN** a configuration write is requested with idle ride state, ready package state, normal mode, and all other write conditions satisfied
- **THEN** the backend permits the write regardless of pressed, unknown, or missing footpad readings and reports success only after verified readback

#### Scenario: Unsafe board state

- **WHEN** a write is requested while riding, package mode is flywheel or otherwise not normal, firmware is read-only, or another Refloat mutation is pending
- **THEN** the backend returns `invalid_state`, sends no command 95, and preserves current values

#### Scenario: Indeterminate interruption

- **WHEN** the device disconnects after command 95 but before verified readback
- **THEN** the request fails without claiming success and a fresh current read is required before any later write

### Requirement: Exact-version saved configurations

Native persistence SHALL store bounded revisioned named Tunes with opaque ID, optional description, package name, Tune-family version, full-schema fingerprint, wire signature, typed values and timestamps. List/apply SHALL match family, fingerprint and signature. Only audited beta2/beta3/stable 1.3.1 share a family. Other versions SHALL stay isolated without automatic migration. Create/update/delete SHALL retain revision-conflict rules.

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

### Requirement: Review Tune application before writing

Activating a compatible saved Tune SHALL open a confirmation before any application request. The confirmation SHALL show only changed tunable settings from a fresh current board read to the saved values, with readable labels, units and enum choices. Long differences SHALL scroll with keyboard and touch while the title and actions remain visible. Matching tunes SHALL show no differences and SHALL NOT cause a write.

#### Scenario: Confirm while idle

- **WHEN** the user activates a compatible Tune while the board is idle
- **THEN** the UI shows current and saved values and sends no application request until explicit confirmation

#### Scenario: Confirm while riding

- **WHEN** an eligible riding user opens the confirmation
- **THEN** the modal includes the momentary-cutout warning and retains authoritative live safety gating

#### Scenario: Cancel or matching tune

- **WHEN** the user cancels by button, backdrop, Escape or native Back, or inspects a matching Tune
- **THEN** no application occurs and dismissal restores focus to the activating control

### Requirement: Preserve reviewed Tune application state

Tune application SHALL retain the reviewed saved revision and refresh current board values before confirming. Changed tunable values SHALL refresh the comparison and require explicit review again. Read or apply failure SHALL retain the modal with actionable error feedback and no automatic retry. Connection or identity changes SHALL NOT authorize application using the old preview. Confirmation state SHALL remain transient across frontend recreation.

#### Scenario: Values change during review

- **WHEN** current tunable values change before confirmation
- **THEN** the updated comparison is displayed without an application request and the user must confirm again

#### Scenario: Failed operation

- **WHEN** the current read or application fails
- **THEN** the confirmation remains open with an error and allows dismissal or an explicit retry when eligible

#### Scenario: Connection changes

- **WHEN** the board disconnects or its exact configuration identity changes during review
- **THEN** confirmation cannot apply the old preview to the replacement connection or identity

#### Scenario: Frontend recreation

- **WHEN** the frontend is recreated with an open confirmation
- **THEN** it restores no pending confirmation or application request

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

### Requirement: Visible persistent configuration write feedback

The editor SHALL show actionable write-block guidance in its fixed footer and open confirmation. Failed-write feedback SHALL remain visible in the footer after dismissal until a successful write or explicit read replaces it. Feedback SHALL preserve edits, scroll position, focus behavior, and guarded pointer/keyboard activation, without automatic mutation retries.

#### Scenario: Scrolled blocked editor

- **WHEN** writing is blocked while the user is scrolled through fields
- **THEN** the reason and disabled write action remain visible together without scrolling to the start

#### Scenario: Readiness changes inside confirmation

- **WHEN** a backend blocker appears while confirmation is open
- **THEN** the confirmation displays its actionable reason beside the disabled confirm action and remains dismissible

#### Scenario: Failed write dismissed

- **WHEN** a write cannot be confirmed and the user closes its confirmation
- **THEN** the footer retains the failure and recovery guidance without claiming that the write did not reach the board or clearing the draft

#### Scenario: Successful recovery

- **WHEN** a subsequent write succeeds or an explicit read succeeds
- **THEN** the previous failed-write feedback clears without being replayed as a mutation

### Requirement: Exact beta3 configuration support

The backend and simulator SHALL additionally support exact Refloat 1.3.1-beta3 with its audited full-schema fingerprint and wire signature using the existing bounded schema/value codec, configuration write/readback, cache and Tune policies. Beta2, beta3 and stable 1.3.1 SHALL share the audited `1.3.1` Tune family, with matching full-schema fingerprint and wire signature. Supported stored data SHALL remain readable.

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

Exact stable 1.3.1 SHALL use the audited beta3 schema fingerprint, wire signature and bounded configuration lifecycle. Its cache and configuration drafts SHALL retain exact observed versions. Tunes for audited beta2, beta3 and stable SHALL share canonical version `1.3.1`, while matching package, full schema fingerprint and wire signature.

#### Scenario: Stable schema and Tune

- **WHEN** stable 1.3.1 supplies the audited 172-field schema
- **THEN** current/default reads, safe writes, verified readback and compatible-family Tunes are available
- **AND** existing beta Tunes remain readable, listable, editable and applicable on all three audited versions without data migration

### Requirement: Bounded Tune family compatibility

Only exact `1.3.1-beta2`, `1.3.1-beta3` and `1.3.1` SHALL share the `1.3.1` Tune family. New Tunes and public saved-Tune identities SHALL use `1.3.1`. Existing beta-labelled rows SHALL participate in family-filtered pagination without changing their IDs, revisions, values or timestamps. Other versions and schemas SHALL remain incompatible.

#### Scenario: Existing beta Tune on stable

- **WHEN** a supported stored beta2 or beta3 Tune is listed, read, updated or applied on stable 1.3.1
- **THEN** its public Tune version is `1.3.1` and family matching succeeds only with the matching full schema and signature
- **AND** fresh current revision, confirmation, mutation safety and verified readback remain required

#### Scenario: Incompatible neighbor

- **WHEN** beta1, beta4, 1.3.0, 1.3.2 or a mismatched schema or signature is supplied
- **THEN** the Tune remains incompatible without a device write

### Requirement: Fresh saved-Tune capture

Creating or updating a Tune from current board values SHALL require a fresh authoritative normal-mode current revision and reject stale or flywheel-temporary values. Complete typed portable values SHALL remain bounded and schema-validated. Family equivalence SHALL NOT relax revision, value or mode validation.

#### Scenario: Stale source

- **WHEN** a caller saves current values from an old revision or flywheel mode
- **THEN** the request fails without modifying the saved Tune

### Requirement: Portable Tune application safety

Portable Tune apply SHALL retain fresh-base, read/write, exclusive-mutation and verified-readback rules in normal mode. Audited releases at or above 1.1.0 MAY apply with known idle/riding state and ready/running package; 1.0.0/1.0.1 SHALL require idle/ready. Footpad readings SHALL NOT restrict application or configuration writes. General edited writes SHALL retain idle/ready rules.

#### Scenario: Shared family keeps write gates

- **WHEN** a compatible-family Tune is applied
- **THEN** the shared mutation authority uses the target board's exact version and existing safety checks, with success only after complete mandatory readback

### Requirement: Source-correct numeric editor presentation

Numeric configuration fields SHALL use the active schema's VESC Tool editor units, decimal precision and increment steps. Percentage editors SHALL use whole percentages of the declared maximum absolute bound. Edited inputs SHALL normalize at editor precision before conversion to canonical firmware units. Untouched values, stored Tunes, draft values, schema fingerprints and wire signatures MUST retain their meaning. Existing overflow rejection and mutation eligibility SHALL remain enforced.

#### Scenario: Scaled percentage field

- **WHEN** brightness has canonical value 0.5, editor scale 100 and step 5
- **THEN** the editor and review display 50 percent, accept 50 as canonical 0.5 and increment by five displayed percentage points

#### Scenario: Editor decimal precision

- **WHEN** a user enters 18.19 for a one-decimal current field
- **THEN** the draft uses canonical 18.2 and the settled editor and confirmation display 18.2

#### Scenario: Percentage editor

- **WHEN** the duty tiltback field uses upstream percentage editing
- **THEN** its editor displays whole percentages and converts them to the corresponding canonical fraction

#### Scenario: Restored canonical draft

- **WHEN** an existing saved draft or Tune contains canonical fractions and numeric input text
- **THEN** the editor converts their presentation without reinterpreting stored fractions as percentages or changing untouched values

#### Scenario: Invalid or incomplete input

- **WHEN** numeric input is incomplete, non-finite or outside the declared bounds
- **THEN** the editor retains actionable validation and does not enable a write using a previously valid value

### Requirement: Actionable numeric encoding limits

Configuration schemas SHALL expose canonical numeric encoding bounds. The editor SHALL reject values beyond those bounds before writing, identify each affected field and show a usable limit in editor units and precision. Input controls SHALL respect encoding limits as well as schema bounds. Invalid restored drafts MUST remain visible and correctable without board traffic. Backend validation MUST remain authoritative.

#### Scenario: Unsigned integer overflow

- **WHEN** a field allows 100000 in its schema but its encoding stores at most 65535
- **THEN** the editor identifies that field, explains the controller limit of 65535 and blocks writing until corrected

#### Scenario: Decimal limit and editor precision

- **WHEN** a field stores at most 0.32767 and its editor uses three decimal places
- **THEN** overflow feedback offers 0.327 or lower as a usable correction and never recommends a value that rounds to an overflowing encoding

#### Scenario: Restored overflow

- **WHEN** an existing draft contains an overflowing value
- **THEN** opening the editor retains the value, displays field-specific guidance and disables writing without sending a mutation

### Requirement: Actionable configuration error presentation

Configuration operations SHALL present generic backend error responses as understandable recovery guidance for the attempted action, preserving stable error codes and original error envelopes for diagnostics. Useful specific messages SHALL remain intact. Write failures MUST retain edits, avoid automatic retries and preserve uncertainty when a write may have reached the controller.

#### Scenario: Generic state rejection

- **WHEN** an eligible-looking configuration write receives a generic invalid-state error
- **THEN** the user sees guidance to check connection and board readiness and reread current values, without backend-state terminology

#### Scenario: Specific rejection

- **WHEN** the backend supplies a specific actionable rejection
- **THEN** that explanation remains visible rather than being replaced by generic recovery copy

#### Scenario: Indeterminate write

- **WHEN** write confirmation times out or the controller becomes unavailable
- **THEN** feedback preserves the possibility that the write reached the controller and asks for a fresh current read before retrying

### Requirement: Per-field firmware defaults

Every configuration field SHALL offer a Default action immediately before its Help action. Default SHALL read the active controller's default values and replace only the selected draft value, clearing that field's invalid input while preserving other edits and the current-value write base. No default action SHALL write to the board. Loading, failure and connection replacement MUST preserve draft integrity and allow recovery.

#### Scenario: Default one field

- **WHEN** a user activates a field's Default action
- **THEN** its firmware default replaces only that draft field and normal explicit write confirmation remains required

#### Scenario: Touch and keyboard activation

- **WHEN** the default action is activated by primary pointer release or keyboard
- **THEN** it runs once; non-primary, cancelled and disabled activations do nothing

#### Scenario: Default read fails or connection changes

- **WHEN** the default response fails or belongs to a replaced connection
- **THEN** existing edits remain unchanged and actionable feedback permits an explicit retry

### Requirement: Stable minor configuration admission

Supported stable patches SHALL accept audited full-schema fingerprints and wire signatures from their stable minor family, replacing exact patch matching. Unknown schemas and cross-minor schemas SHALL fail closed for configuration. Configuration identity and caches SHALL retain the exact observed patch. Legacy INFO 1.0 SHALL retain fingerprint-selected source identity. Existing exact prerelease schema and Tune-family rules SHALL remain unchanged.

#### Scenario: Later patch reuses known schema

- **WHEN** stable 1.2.255 or 1.3.2 supplies an audited schema from its own stable minor
- **THEN** configuration reads and guarded writes are available with the observed patch version and unchanged full fingerprint/signature
- **AND** reconnect and cached-schema validation apply the same policy

#### Scenario: Schema validation remains local and strict

- **WHEN** a supported patch supplies an unknown fingerprint or signature, a different minor's schema or malformed XML
- **THEN** configuration remains unavailable without broadening admission from the version guarantee alone

#### Scenario: Tune identity remains exact

- **WHEN** a stable patch has the same schema as another patch but a different exact Tune version
- **THEN** it does not silently share saved Tunes across patches
- **AND** exact same-version Tunes and the existing beta2/beta3/stable-1.3.1 Tune family remain available
