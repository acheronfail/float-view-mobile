## ADDED Requirements

### Requirement: Rust Refloat configuration service candidate parity

The bounded Rust candidate SHALL independently reproduce the selected native Refloat configuration lifecycle and safety decisions above the existing Rust codec without becoming a supported backend or production runtime. It SHALL own the audited release catalogue, schema paging and assembly, cache admission decisions, generation and operation matching, flywheel restrictions, current/default revisions, write construction, acknowledgement-to-verification sequencing, normalized readback comparison, conflict classification, failure recovery, and reset behavior through deterministic platform-neutral facts, events, effects, and snapshots.

#### Scenario: Begin from cache miss or valid cache

- **WHEN** begin facts describe an eligible direct or CAN controller generation with no cache, a valid audited cached schema, an unaudited cached schema, or corrupt cached bytes
- **THEN** the candidate SHALL deterministically select bounded schema paging or the exact current-value request
- **AND** it SHALL emit the exact store/remove/no-cache decision without reading or writing host storage itself

#### Scenario: Assemble and admit an audited schema

- **WHEN** exact operation- and generation-matched schema chunks arrive in order within every declared extent
- **THEN** the candidate SHALL assemble no more than the codec bound, decode through the existing safe codec, resolve the exact reported release and full fingerprint/signature, emit a cache-store decision where eligible, and request current values unless flywheel mode forbids it

#### Scenario: Read current and default values

- **WHEN** matched current or default value bytes arrive against the selected schema
- **THEN** the candidate SHALL decode them through the safe codec, increment one monotonic revision, publish the exact source and complete typed values, and clear only the completed operation

#### Scenario: Verify one configuration write

- **WHEN** a writable normal-mode service receives an exact identity, base revision, and complete typed value set followed by a matching acknowledgement and authoritative readback
- **THEN** the candidate SHALL encode through the safe codec, sequence write then verification effects under one operation, normalize only the audited non-legacy metadata field, publish the verified current revision, and return success only for an exact normalized match

#### Scenario: Enforce state and flywheel safety

- **WHEN** callers request reads or writes while unavailable, loading, busy, stale, read-only, identity-mismatched, revision-mismatched, or in flywheel mode
- **THEN** the candidate SHALL return the exact closed native-equivalent failure without emitting host work or partial state
- **AND** entering flywheel SHALL invalidate current values and cancel only an automatic current read while leaving schema/default state bounded and intact
- **AND** exiting flywheel SHALL request one fresh current read only when the connection remains eligible and idle

#### Scenario: Reject stale, malformed, and adversarial sequences

- **WHEN** chunks, values, acknowledgements, verification responses, failures, aborts, generations, operations, routes, indexes, commands, lengths, offsets, or ordering violate the active transition
- **THEN** the candidate SHALL fail closed with a stable payload-free error, preserve bounded valid state where specified, and never publish partial schema, values, payload, cache bytes, or effects

#### Scenario: Reset and recover deterministically

- **WHEN** an active operation fails, aborts, reconnects, or resets across generation and operation-counter boundaries
- **THEN** the candidate SHALL match native availability/current/pending recovery semantics, reject all late work, clear connection-scoped data, and produce the same snapshot and effects on repeated equivalent sequences

#### Scenario: Preserve production isolation

- **WHEN** the candidate is added to the locked workspace and complete verification runs
- **THEN** Swift and Kotlin SHALL remain the selected production service owners and no Apple FFI, Android JNI, bridge catalogue entry, package dependency, generated binding, exported symbol, persistent format, or production call path SHALL change
