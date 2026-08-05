## 1. Blocking prerequisite reconciliation

- [ ] 1.1 Verify every preceding OpenSpec change that affects controller discovery, the request scheduler, BMS work, Refloat telemetry, live controls, fixture behavior, contract shape, or storage has been implemented, synced, and archived; stop without implementation if any prerequisite remains active.
- [ ] 1.2 Re-read canonical specs and current frontend, Swift, Kotlin, SQL, conformance, and fixture ownership after those archives, then revise this proposal, design, delta specs, and tasks to remove stale names, resolve overlapping requirements, and preserve one scheduler and one Refloat mutation gate.
- [ ] 1.3 Resolve the final scheduler priority/context API and audit exact Refloat write normalization, especially legacy 1.0 patch identification and any fields beyond `meta.is_default`, against the then-current vendored sources.
- [ ] 1.4 Run `OPENSPEC_TELEMETRY=0 openspec validate add-refloat-configuration --strict` and commit the reconciled planning artifacts before starting implementation.

## 2. Immutable schemas and protocol vectors

- [ ] 2.1 Extract exact source-derived compressed ConfigParams schemas for Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 into bounded immutable backend fixtures with tag, source path, and provenance metadata.
- [ ] 2.2 Generate and review canonical normalized-schema expectations, SHA-256 full-schema fingerprints, VESC Tool-compatible CRC32C wire signatures, defaults, representative current values, serialized payloads, and accepted writes for every audited release.
- [ ] 2.3 Add adversarial fixtures for Qt envelope/decompression failure, expansion excess, DTD/entity input, excessive nesting/counts/strings, duplicate fields/order, invalid references/types/encodings/numbers, unaudited fingerprints, truncation, signature mismatch, and remainder bytes.
- [ ] 2.4 Add command 92–95 direct and CAN route vectors for schema chunks, current/default values, valid writes, acknowledgements, wrong index/offset/route, rejection, normalization, and matching/mismatching readback.
- [ ] 2.5 Prove fixtures distinguish Refloat 1.0.0 from 1.0.1 by full-schema fingerprint even when INFO and wire signature collide, and prove later patch identities match their reported version and audited fingerprint.

## 3. Mirrored native schema and value codecs

- [ ] 3.1 Add matching Swift and Kotlin bounded configuration models for schema identity, groups/subgroups, supported field metadata, typed values, pages, value revisions, operations, and stable codec errors.
- [ ] 3.2 Implement the Apple bounded Qt `qCompress` envelope/zlib decoder and streaming XML parser using platform facilities, explicit vocabulary/structure limits, DTD/entity rejection, canonical fingerprint input, and sanitized descriptions.
- [ ] 3.3 Implement the matching Android decoder/parser with identical accepted metadata, normalization, sanitization, bounds, fingerprint input, and error classifications.
- [ ] 3.4 Implement VESC Tool-compatible CRC32C wire-signature construction and exact supported field wire decoding/encoding in Apple, including complete consumption and finite/range/representability validation.
- [ ] 3.5 Implement the matching signature and value codec in Android with byte-for-byte equivalent results and failures.
- [ ] 3.6 Add table-driven Swift and Kotlin tests over every immutable valid/adversarial schema, fingerprint, signature, value, and write vector.
- [ ] 3.7 Delete any temporary or duplicated schema/version-specific codec path after the generic mirrored implementation passes the shared vectors.

## 4. VESC identity, commands, and scheduling

- [ ] 4.1 Extend both firmware identity decoders and private firmware records with bounded custom-config count and nullable hardware-configuration CRC while keeping them out of public identity and compatibility broadening.
- [ ] 4.2 Add VESC commands 92–95 and matching typed request/response decoders on Apple for exact index, total length, offset, chunk bytes, values payload, and write acknowledgement.
- [ ] 4.3 Add the corresponding Kotlin commands and decoders with equivalent bounds and errors.
- [ ] 4.4 Extend the source-faithful firmware identity fixtures/tests for version-applicable custom-config count and CRC tails without weakening existing strict remainder handling.
- [ ] 4.5 Add typed scheduler transactions for schema chunks, current/default reads, full write, and verification read with exact route, offset, operation, priority, deadline, and connection-generation context on both platforms.
- [ ] 4.6 Verify direct and CAN routing wraps every command exactly once, rejects wrong-route/offset/generation responses, bounds chunk retries/queues, and preserves riding telemetry priority over configuration work.

## 5. Native configuration lifecycle and safety

- [ ] 5.1 Implement the Apple connection-scoped `RefloatConfigurationService` for family discovery, exact audited schema resolution, bounded chunk assembly, progress/availability, current/default revisions, cancellation, and reconnect behavior.
- [ ] 5.2 Implement the matching Android service with equivalent state transitions, deadlines, diagnostics, and lifecycle behavior.
- [ ] 5.3 Integrate a shared native Refloat mutation gate on Apple so semantic live controls, full configuration writes, and Tune apply cannot overlap and all preconditions are rechecked immediately before scheduler submission.
- [ ] 5.4 Integrate the matching Android mutation gate and remove superseded per-owner exclusivity or direct write paths.
- [ ] 5.5 Enforce ready read/write access, exact version/fingerprint/signature, latest current base, normal package mode, idle ride state, no engaged footpad, fresh current-generation evidence, and complete validation before command 95 on both platforms.
- [ ] 5.6 Implement Apple command-95 acknowledgement followed by mandatory command-93 verification, expected firmware normalization, canonical comparison, mismatch/current-state publication, and indeterminate interruption behavior.
- [ ] 5.7 Implement the equivalent Android write/readback lifecycle and result/error mapping.
- [ ] 5.8 Invalidate current configuration values after confirmed semantic actions that affect serialized fields without deriving live-control state optimistically from configuration drafts.
- [ ] 5.9 Add mirrored lifecycle tests for read-only access, riding/footpad/mode rejection, competing operations, stale bases, disconnect phases, route replacement, acknowledgement without persistence, matching/mismatching readback, and recovery.

## 6. Contract, loopback, and mock

- [ ] 6.1 Add bounded TypeScript schema identity, field/page, typed value/revision, availability/pending operation, Tune record/page, and request/result shapes with fixed field/string/count/cursor/JSON limits.
- [ ] 6.2 Add mandatory `refloat.configuration.open/schema/read/write` and `refloat.tune.list/get/create/update/delete/apply` methods to the contract and frontend client without raw XML, command, route, or serialized-byte inputs.
- [ ] 6.3 Add the small `ActiveConnection.refloatConfiguration` summary to complete authoritative state and validators while keeping schema fields, values, and Tunes request-scoped.
- [ ] 6.4 Implement strict Apple loopback parsing, validation, serialization, one-terminal-result behavior, and stable error mapping for every new method.
- [ ] 6.5 Implement matching Android loopback handling with identical bounds and semantics.
- [ ] 6.6 Extend the mock backend with deterministic paged schemas, current/default values, safety/readback outcomes, Tunes persistence, reconnect/version changes, and authoritative summary revisions.
- [ ] 6.7 Add contract/mock tests for page cursors, encoded-size bounds, malformed typed maps, stale version/fingerprint/signature/base, operation contention, slow frontend snapshots, and every terminal error.

## 7. Exact-version native persistence and cache

- [ ] 7.1 Add canonical SQL for bounded application-wide `refloat_configuration` records with revisions, names/descriptions, exact package version, schema fingerprint, wire signature, values JSON, timestamps, checks, and useful exact-match indexes.
- [ ] 7.2 Bump the disposable storage version and update SQL/schema fixtures so incompatible test databases are recreated rather than migrated.
- [ ] 7.3 Implement Apple create/get/paged-list/update/delete operations with transactions, bounded JSON validation, optimistic revision conflicts, deterministic ordering/cursors, and exact compatibility filtering.
- [ ] 7.4 Implement matching Android persistence and query behavior.
- [ ] 7.5 Implement bounded Apple schema cache storage keyed by audited connection identity/custom index/nonzero hardware CRC with digest revalidation, corruption deletion, and fetch fallback.
- [ ] 7.6 Implement the equivalent Android cache behavior and ensure cache identity is never exposed publicly or used alone as write authority.
- [ ] 7.7 Add native storage tests for limits, pagination, conflicts, exact 1.0 patch isolation, later patch isolation, app restart, delete, cache hit/miss/change/corruption, and schema-version recreation.

## 8. Generated frontend editor and Tunes

- [ ] 8.1 Add a connection-owned Refloat configuration controller that opens native state, loads deterministic schema pages and current/default values, tracks source/dirty/validation state, and invalidates stale drafts across reconnect or identity change.
- [ ] 8.2 Build reusable generated field rendering for supported double, integer, enum, boolean, string, and bitfield metadata with native form semantics, schema bounds, steps, precision, suffixes, choices, and sanitized help.
- [ ] 8.3 Build the full-height responsive Configure Refloat surface with subgroup navigation, search, loading/error/progress states, dirty count, read-current/read-default actions, unsaved-change handling, and focus restoration.
- [ ] 8.4 Add bounded current-versus-draft diff presentation and explicit persistent-write confirmation that issues exactly one complete version/fingerprint/signature/base-revision request.
- [ ] 8.5 Add `Configure Refloat` to the `/live` menu with capability/read-only/reconnect states and the established primary-pointer plus guarded keyboard/assistive activation behavior.
- [ ] 8.6 Build the Tunes surface for exact-compatible application-wide records, including create from complete draft/current, bounded rename/description, paged list, diff, delete confirmation, revision conflict recovery, and verified apply.
- [ ] 8.7 Add `Tunes…` to the `/live` menu and explanatory copy that records are complete configurations, not upstream QML curated/migrating Tunes.
- [ ] 8.8 Add frontend tests for every generated field type, bounds/formatting/search/grouping, default-versus-current source, dirty and stale state, diff/confirm, read-only/safety gating, Tunes/version filtering/conflicts, reconnect/WebView recovery, non-primary activation, cancellation, keyboard use, and no double activation.

## 9. Fixture and canonical conformance

- [ ] 9.1 Extend the conformance semantic Refloat model with exact versioned schema/default/current configuration state, expected normalization, bounded counters, and reset behavior without duplicating nine board profiles.
- [ ] 9.2 Implement source-correct commands 92–95 in the software and ESP32-backed fixture paths for direct and CAN routes, exact chunks, complete signed values, accepted disposable writes, and later readback.
- [ ] 9.3 Preserve strict rejection for test firmware, unknown index, unsafe state, invalid signature/value, arbitrary app-data tuning, firmware/motor configuration, motor/remote/Handtest, recording/history, malformed, and unknown commands.
- [ ] 9.4 Extend the interactive fixture UI/state with observable configuration identity, operation counters, safe reset, and diagnostics while keeping scenario interpretation in conformance and all motor actuation impossible.
- [ ] 9.5 Add canonical mock/Apple/Android scenarios for every contract method and summary transition across all exact stable releases, direct/CAN, saved/guest, current/default, paging, caching, semantic-control contention, reconnect, and frontend recreation.
- [ ] 9.6 Add canonical adversarial schema/value/bound/route/generation/scheduler/write/readback matrices and assert equivalent errors, bytes, revisions, and absence of partial state across native targets.
- [ ] 9.7 Add exact-version Tune persistence/conformance scenarios including the Refloat 1.0 INFO ambiguity and identical wire-signature collision.
- [ ] 9.8 Add the explicit physical direct/CAN configuration checkpoint that captures current values, reads defaults, changes one fixture-safe non-actuating field, restores the full original configuration, and fails if restoration is not verified.

## 10. Documentation and verification

- [ ] 10.1 Update frontend/native/conformance ownership documentation with configuration schema/value flow, trust boundaries, scheduler and mutation ownership, cache/persistence locations, exact-version policy, and write/readback failure semantics.
- [ ] 10.2 Update vendor provenance documentation and fixture-generation instructions so audited schemas/fingerprints/signatures can be refreshed deliberately for a newly supported release.
- [ ] 10.3 Run focused frontend unit/type checks, Swift tests, Android unit/instrumented storage tests, conformance tests, fixture builds/tests, and OpenSpec strict validation; fix all failures.
- [ ] 10.4 Run `just verify` as the complete software gate and record the explicit physical BLE checkpoint separately where required.
- [ ] 10.5 Review for raw schema/value/private-identity logging, unbounded collections/messages/retries/cache/files, duplicate scheduler/mutation paths, platform-specific frontend branches, and superseded blanket rejection code; remove any violations before completion.
