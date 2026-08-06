## ADDED Requirements

### Requirement: Canonical Refloat configuration coverage

Canonical conformance SHALL cover every Refloat configuration and Tune contract method, state-summary field, native codec, safety decision, scheduler interaction, persistence rule, and frontend recovery behavior against mock, Apple, and Android. Immutable source-derived fixtures SHALL include exact compressed schemas, normalized metadata, full-schema fingerprints, wire signatures, current/default payloads, and valid writes for every audited stable Refloat 1.0.0–1.2.3 release. Software scenarios SHALL cover legacy 1.0 patch resolution and signature collision, direct and CAN routes, cache hit/miss/corruption, paging, bounds, malformed XML and values, stale revisions/generations, semantic-control contention, write acknowledgement/readback, reconnect, read-only firmware, and exact-version isolation.

#### Scenario: Exact release matrix

- **WHEN** configuration vectors run for each audited stable Refloat release
- **THEN** Swift and Kotlin produce equivalent normalized schemas, signatures, typed values, encoded writes, and strict errors from the same upstream-derived fixtures

#### Scenario: Malformed input matrix

- **WHEN** schemas or values contain truncation, expansion excess, entities, duplicate fields, invalid serialization references, unsupported encodings, signature mismatch, invalid values, or remainder bytes
- **THEN** all targets fail closed without partial state, writes, unbounded work, or raw-data diagnostics

#### Scenario: Write verification matrix

- **WHEN** command 95 is acknowledged and readback matches, differs, times out, is malformed, or arrives from a stale route/generation
- **THEN** all targets return the same terminal result and only the complete matching current state can succeed

#### Scenario: Exact-version persistence

- **WHEN** configurations are created, updated, listed, applied, deleted, and observed after a package-version change
- **THEN** revisions, bounds, durable results, conflicts, and compatibility filtering are equivalent across targets

#### Scenario: Frontend interaction matrix

- **WHEN** generated fields, source actions, confirmations, Tunes, unsaved state, pointer activation, keyboard activation, and reconnect recovery are exercised
- **THEN** the frontend sends exactly the intended bounded requests and never double activates or applies a stale draft

### Requirement: Physical configuration checkpoint

Physical conformance SHALL perform a bounded non-motor Refloat configuration checkpoint for representative legacy and dynamic packages through direct and CAN routes on each native backend. It SHALL capture current values, read defaults without writing, change one fixture-safe non-actuating value through the complete write/readback flow, and restore and verify the captured starting configuration before completion. Failure to restore SHALL fail the checkpoint with actionable bounded output.

#### Scenario: Safe physical round trip

- **WHEN** a writable fixture runs the configuration checkpoint
- **THEN** the backend reads schema/current/default, verifies one safe write, restores the original complete values, and causes no motor movement

#### Scenario: Interrupted restore

- **WHEN** connectivity is lost before original values are verified
- **THEN** the checkpoint fails explicitly and does not claim cleanup or success
