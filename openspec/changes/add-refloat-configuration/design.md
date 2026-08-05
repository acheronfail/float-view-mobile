## Context

VESC firmware exposes registered custom configurations through commands 92–95. Refloat registers one configuration at index `0`: command 92 returns its Qt-compressed `ConfigParams` XML in bounded chunks, commands 93 and 94 return complete current and default serialized values, and command 95 accepts a complete serialized replacement. The XML supplies parameter types, bounds, display metadata, grouping, serialization order, wire encodings, defaults, and the inputs for a CRC32C schema signature. The audited stable Refloat packages contain 144–167 serialized fields and approximately 304–353 KB of uncompressed XML, but only approximately 23–26 KB on the device.

Float View currently does not retain the firmware custom-config count or hardware-configuration CRC, implement commands 92–95, parse `ConfigParams`, expose configuration methods, or store named Refloat configurations. The earlier `support-vesc-bms-telemetry` change introduces the one-request-in-flight `VescRequestScheduler`; `add-refloat-live-controls` introduces exact-version Refloat mutation profiles and one pending semantic control action. This change is deliberately proposed before those changes are implemented, so its first task is a blocking reconciliation against their archived repository state.

The upstream VESC acknowledgement for command 95 does not carry Refloat's `set_cfg` success value. A received acknowledgement therefore proves only transport handling, not persistence. Refloat also rewrites `meta.is_default`, can reject writes in special modes, and otherwise permits configuration changes while running. Float View needs stricter product safety and mandatory current-value readback.

## Goals / Non-Goals

**Goals:**

- Generate a complete grouped Refloat configuration editor from the active device schema rather than a version-hardcoded frontend form.
- Support exact current/default reads and persistent full-configuration writes for every audited stable Refloat 1.0.0–1.2.3 package over direct and CAN routes.
- Keep schema parsing, signatures, encoding, validation, request scheduling, lifecycle, and persistence in matching native owners.
- Verify every write by rereading current values and comparing canonical wire values.
- Store bounded named full configurations by exact package version and schema signature and never migrate them implicitly.
- Preserve the existing semantic live controls and arbitrate all Refloat mutations through one connection-scoped policy.
- Make malformed, oversized, stale, read-only, unsafe, unsupported, and unverifiable operations fail closed.

**Non-Goals:**

- Executing or recreating Refloat's bespoke QML application.
- Reproducing its six-slot Tune blacklist, automatic board backup, or cross-version migration behavior.
- Supporting unknown, prerelease, or future packages merely because their XML can be parsed.
- Partial writes, optimistic success, automatic application on connect, or configuration changes while riding.
- VESC motor/app configuration, firmware updates, motor control, Handtest, remote movement, record/history commands, or raw protocol access from the frontend.

## Decisions

### 1. Treat the connected schema as data but keep compatibility closed

The backend downloads custom configuration index `0` after supported Refloat family discovery. It retains the non-public custom-config count and hardware-configuration CRC from controller identity, verifies that index `0` exists, assembles command-92 chunks under fixed compressed and expanded byte limits, decodes the Qt `qCompress` envelope with the platform standard zlib implementation, and parses only the allowlisted `ConfigParams` vocabulary.

The normalized schema retains package name and exact source version, a SHA-256 fingerprint of canonical validated full-schema metadata, hardware CRC when present, computed wire configuration signature, ordered transmittable fields, defaults, types, encodings, bounds, enum labels, sanitized plain-text descriptions, and General grouping. Non-transmittable identity fields and Internal fields may be used for validation but are not editable. Unknown XML elements are ignored only within explicit nesting and byte/count/depth bounds; DTDs, external entities, duplicate parameter names, duplicate serialization entries, missing referenced fields, unsupported encodings, invalid numbers, excessive strings/enums/groups, or a trailing expanded document fail the load.

Refloat 1.0.x INFO reports only wire version `1.0`, so exact 1.0.0 versus 1.0.1 identity is resolved only after the full validated schema fingerprint matches one immutable audited source fixture. Their wire signatures are the same because the serialized layout is unchanged, while their descriptions/group labels differ. Later INFO versions report the patch, but their schema fingerprint must still match the corresponding audited release. A mismatch leaves configuration unavailable without disrupting already-supported telemetry.

Runtime parsing avoids one hardcoded frontend form, but it does not broaden the audited package catalog. An unknown version may remain unsupported even if it returns syntactically valid XML.

Alternative considered: ship one schema per version in the application. This would duplicate upstream metadata, make patch constraint changes easy to miss, and defeat the device's self-description. Immutable source-derived schemas remain in conformance only as golden fixtures.

### 2. Mirror one typed native codec and never accept opaque frontend bytes

Apple and Android add corresponding Refloat configuration protocol owners below application coordination. Each computes the exact VESC Tool CRC32C signature from serialization order, parameter type, wire type, and enum names; decodes commands 93/94 only when the returned signature and exact payload consumption match; and encodes command 95 from a complete typed value map. Numeric values must be finite, within schema bounds, and exactly representable through the declared wire encoding. Enums, booleans, bitfields, integers, and strings receive matching strict checks.

The frontend receives normalized fields and typed values and submits a complete typed draft plus package version, schema signature, and base value revision. It never supplies a serialized payload, command number, CAN ID, schema XML, or arbitrary parameter absent from the normalized schema.

Alternative considered: parse XML and serialize values once in TypeScript. That would reduce mirrored code but move protocol validation into a disposable WebView, permit opaque native writes, and violate the repository's ownership boundary.

### 3. Use a connection-scoped configuration owner above the shared scheduler

One `RefloatConfigurationService` per native backend owns schema availability, the current/default value revisions, one configuration operation, schema cache access, and saved-configuration application. It submits typed scheduler transactions using the selected controller route exactly once. Schema chunks are background controller work; explicit current/default reads and a user write are below due riding telemetry but above background lighting refresh, auxiliary discovery, and BMS polling. No catch-up chunk queue is accumulated.

The configuration owner and semantic live-control owner share one connection-generation mutation gate. A configuration write cannot overlap package/flywheel/lighting/fatal operations, and a semantic persistent action cannot begin during a configuration write or verification read. Disconnect, route replacement, or generation change cancels outstanding work and prevents late responses from updating replacement state.

Alternative considered: give configuration its own write/timeout coordinator. That would reintroduce competing requests on the uncorrelated VESC stream and duplicate scheduler lifecycle logic.

### 4. Keep large schemas out of authoritative state and page them by subgroup

`ActiveConnection` gains only a small `refloatConfiguration` summary: status, writable flag, exact package version, nullable schema signature, whether current/default values are loaded, and nullable pending operation. It contains neither schema fields nor saved configurations.

Mandatory control methods open/refresh the configuration, retrieve a bounded subgroup page with a cursor, read current or default values, and write a complete draft. Every schema page has bounded fields, enum values, description bytes, and encoded JSON size below the existing WebSocket limit. Values are returned as one bounded map because the audited serialized field count and value sizes fit comfortably; the contract still fixes count and byte limits.

Saved-configuration methods list bounded metadata pages and get/create/update/delete records separately. Apply is a write operation using the saved record only after exact active-version and schema-signature validation.

Alternative considered: publish schema in `AppState`. Expanded metadata and descriptions can exceed 65,536 bytes, would be repeated with telemetry snapshots, and would burden every WebView recreation and slow consumer.

### 5. Write from a complete base and verify canonical readback

Current and default reads each produce a complete immutable value set with a monotonically increasing connection-local revision. Editing occurs in the frontend, but write requires the latest current-value revision as its base even when the visible draft began from defaults or a saved configuration. The backend merges no partial patch: it validates a complete value for every serialized field against the active schema and rejects missing, extra, stale, or mismatched inputs.

Write eligibility requires the same active device, ready read/write access, exact supported package and schema, normal package mode, exactly idle ride state, no engaged footpad, no other Refloat mutation, and a fresh current base. The frontend presents a bounded diff and explicit persistent-write warning, but native policy remains authoritative.

After command 95 acknowledgement, the owner schedules command 93, decodes it under the same schema, and compares every canonical value after expected firmware normalization. `meta.is_default` is expected to become false. Only a complete match returns success and publishes the new revision. Timeout, acknowledgement without matching readback, rejected/silent firmware behavior, disconnect, or mismatch returns `unavailable` or `conflict` and exposes the newly read device values when safe; it never reports optimistic success.

Alternative considered: treat command 95 acknowledgement as success. VESC discards Refloat's setter result, so this would claim writes that may have failed.

### 6. Store full exact-version configurations, not inferred Refloat Tunes

The catalog gains a bounded `refloat_configuration` table containing opaque ID, revision, bounded name and optional description, exact package name/version, full-schema fingerprint, wire signature, bounded typed values JSON, and created/updated times. The product stores application-wide configurations so a rider can switch compatible boards, while list/apply filters strictly by the active exact version, fingerprint, and signature.

The menu label is `Tunes…`, but records are complete configurations. The XML does not contain Refloat's QML tune blacklist, so calling a dynamically inferred subset equivalent to upstream Tunes would be inaccurate. There is no automatic migration: a different exact version creates a separate namespace and old records remain visible only in management views explicitly designed for incompatible records.

The test-stage storage version is bumped and incompatible stores are recreated from canonical SQL rather than migrated.

Alternative considered: copy Refloat's QML Tune subset and migrations. That would create hand-maintained version policy outside the runtime schema and contradict the requested strict version isolation.

### 7. Cache only validated schema bytes under native bounds

To avoid downloading roughly sixty 400-byte chunks on every connection, native storage may cache the validated compressed schema and normalized metadata by VESC firmware identity, exact Refloat version or legacy family, custom-config index, and nonzero hardware-configuration CRC. A cache hit must still recompute the full-schema fingerprint and wire signature and match the audited catalog; any key or digest change invalidates it. A missing/zero CRC fetches from the device. Cache corruption deletes that entry and falls back to a bounded fetch; it never broadens compatibility or supplies write authority by itself.

### 8. Give the editor a dedicated full-height connection-owned surface

The `/live` menu adds `Configure Refloat` when configuration discovery has begun and `Tunes…` when a supported exact version is known. Configure Refloat opens a full-height responsive surface owned by `features/connection`, with generated subgroup navigation, search, field help, current/default source selection, dirty count, validation, write diff, and focus/unsaved-change handling. It uses native button semantics and the established primary-pointer plus guarded keyboard/assistive activation path.

The surface reconstructs by reopening the native connection-scoped configuration after WebView recreation. It never retains a local schema or operation as authoritative. A reconnect makes editing read-only until schema and current values are re-established; a device/version/signature replacement discards the draft after warning rather than applying it to the new target.

### 9. Expand the simulator with exact disposable configuration state

Conformance owns immutable compressed schemas, signatures, default/current byte fixtures, and semantic expected values for every audited stable Refloat release. The programmable fixture composes the selected exact release with one disposable configuration model, answers commands 92–95 over direct or CAN routing, validates full writes, persists them only for the powered fixture scenario, and resets them with its selected package configuration. It never commands motor hardware.

Malformed, unknown, test-version, riding, special-mode, stale, and prohibited commands remain rejected. Canonical software tests cover all exact releases; the physical checkpoint uses representative legacy and dynamic releases and restores the starting configuration after verified writes.

## Risks / Trade-offs

- **Device XML is a moderately large untrusted input.** → Enforce compressed/expanded byte, depth, element, field, group, enum, and string bounds; prohibit entities/DTDs; sanitize descriptions; and consume the document exactly.
- **Swift and Kotlin codecs can drift.** → Drive both from identical immutable XML/value vectors and compare normalized schemas, signatures, bytes, errors, and readback results in canonical conformance.
- **Schema download can take many BLE round trips.** → Use scheduler-friendly bounded chunks, no catch-up queue, progress state, and validated CRC-keyed caching.
- **A write can succeed but verification can be interrupted.** → Return an indeterminate failure, retain no optimistic value, and force a fresh current read before another write.
- **Full configurations may be called Tunes but include hardware and safety fields.** → Label them clearly as complete Refloat configurations in explanatory copy and defer a curated Tune subset until separately specified.
- **Earlier active changes modify the same specs and ownership.** → Make post-archive reconciliation a blocking first task and revalidate every delta against canonical specs before implementation.
- **The VESC firmware may normalize values beyond `meta.is_default`.** → Golden fixtures define known normalization; any unrecognized mismatch fails visibly rather than silently weakening comparison.

## Migration Plan

1. Implement, verify, sync, and archive all preceding changes, including `support-vesc-bms-telemetry` and `add-refloat-live-controls`.
2. Before applying this change, rerun OpenSpec context/status, revise proposal/design/specs/tasks against canonical repository state, resolve overlapping requirements, and commit the revised planning phase.
3. Add immutable upstream-derived schemas and codec vectors, then implement the mirrored read-only schema/current/default path through the existing scheduler.
4. Extend the contract/mock and generated read-only editor before enabling writes.
5. Add the native mutation gate, complete write/readback flow, simulator behavior, and safety/conformance matrix.
6. Add exact-version persistence and Tunes management, bump disposable storage schema, and finish UI integration.
7. Run focused frontend/native/conformance tests and `just verify`; perform the explicit physical BLE checkpoint with restoration.

Rollback removes the configuration menu/actions, protocol handlers, schema cache, and catalog table while preserving telemetry and semantic live controls. Test-stage storage is recreated under the previous schema version. Device configuration is not automatically rolled back; the physical validation workflow restores its captured starting values before completion.

## Open Questions

- Confirm final scheduler priority names and shared mutation-owner API after the prerequisite changes are archived.
- Audit whether any supported release normalizes additional fields on write and capture each known case in immutable readback fixtures.
