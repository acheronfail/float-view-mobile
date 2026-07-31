## 0. Prerequisite

- [ ] 0.1 Complete, sync, and archive `support-direct-and-can-vesc-connections`; confirm its verified-route, discovery, continuity, controller-selection, and fixture-topology contracts are the repository baseline before implementation begins.

## 1. Protocol catalog and canonical fixtures

- [x] 1.1 Add immutable source-provenance fixtures for source-faithful Refloat 1.0.0 command 1 and command 10 detail levels 1–4, Refloat 1.1.0 and 1.2.3 command 31/32 layouts and values, compact charging and accumulated values, active states, and firmware faults; do not add command 35 history.
- [x] 1.2 Encode the `design.md` telemetry catalog, stable names, units, nullability, enum mappings, and legacy-to-dynamic equivalence cases in shared conformance fixtures.
- [x] 1.3 Add shared tests for the 64-field, 64-byte-key, 4,096-byte aggregate-name, 32-extension, 16,384-byte JSON, 65,536-byte message/document, 50-diagnostic, and 2,000-point/event bounds.
- [x] 1.4 Add cross-platform fixtures for latest-reported compact caching, one `slowObservedAt`, decreasing device counters, generation clearing, and discrete transition deduplication.
- [x] 1.5 Expand immutable source provenance and the compatibility catalog to audited stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 releases, grouped by legacy, dynamic, and alert-capable dynamic wire profiles.
- [ ] 1.6 Add immutable upstream provenance for stable VESC 7.00 and the complete 6.02/6.05/6.06/7.00 `COMM_FW_VERSION` baseline, correcting the 6.05 hardware-CRC suffix and recording 7.01 as an excluded test build.

## 2. Conformance simulator and Refloat codecs

- [x] 2.1 Replace the reduced telemetry source with one bounded version-independent simulated board-state model covering the complete catalog, active firmware faults, and no utilization metadata.
- [x] 2.2 Implement deterministic coherent scenes, freeze/resume, individual pinned overrides, validation, and reset behavior without coupling semantic state to a packet family.
- [x] 2.3 Correct the fixture-only Refloat 1.0.0 command 1 high-precision encoder and complete command 10 detail levels 1–4, including field-weakening, accumulated, charging, state, and active-fault responses.
- [x] 2.4 Complete the Refloat 1.1.0 and 1.2.3 command 31/32 encoders with exact always-present/runtime catalogs, values, flags, charging and active-alert tails, and absent-field behavior.
- [x] 2.5 Extend request classification, route-specific simulation, counters, and response fragmentation for realtime, compact, and unknown-safe request paths; reject command 35, commands 41–43, and unsupported writes with bounded diagnostics.
- [x] 2.6 Add table-driven encoder and simulator tests across supported package families, direct/CAN routes, semantic scenes, bounds, invalid values, exact fixtures, and deferred commands.
- [x] 2.7 Project every audited stable version through its shared wire profile and preserve command-10 metres/second versus command-31 kilometres/hour speed semantics.
- [ ] 2.8 Expand the composed VESC identity encoder and workbench selector to stable 7.00, emit version-applicable CRC suffixes from one common profile, and add malformed, unknown, and nonzero-test-version scenarios without duplicating board profiles.

## 3. Coordinated contract, native pipeline, and persistence slice

- [x] 3.1 Extend `frontend/src/lib/backend/contract.ts`, contract fixtures, and the mock with the exact typed catalog, stable state enums that supersede public numeric board/footpad codes, one ISO-8601 `slowObservedAt`, bounded extensions, typed `SessionEvent[]`, `eventsTruncated`, concrete limits, nullability, and no utilization field.
- [x] 3.2 Expand Swift and Kotlin legacy/dynamic decoders to the same catalog, version tails, active fault, compact values, negotiated extensions, and canonical fixtures.
- [x] 3.3 Implement matching Apple and Android normalization that trusts each new protocol-valid device value, merges the latest compact cache into realtime samples, clears it on route generation, and never clamps decreasing counters.
- [x] 3.4 Add the same single-stream native polling rule on Apple and Android: command 31 at the existing cadence, one command 10 detail-level-4 request due each second in the next free slot, one in-flight request, no immediate retry, and no command 35 lane.
- [x] 3.5 Update the shared SQL schema and both stores together: retain summary/query columns, add bounded `detail_json` and `extension_json`, append deduplicated discrete changes through an independent monotonic `session_event` sequence in stable catalog order, and commit each normalized sample and its transitions atomically.
- [x] 3.6 Update Apple and Android session retrieval, downsampling, diagnostics, summaries, deletion, loopback serialization, and export with the same numeric series, transition events, bounds, units, and identifier privacy.
- [x] 3.7 Enforce one selected controller per active telemetry session on both hosts; a controller or route change ends the old session and clears all route-scoped telemetry before accepting the new controller.
- [x] 3.8 Add matching Swift and Kotlin tests for decoding, layouts, scheduling, direct/CAN routing, latest compact values, decreasing counters, transitions, controller changes, reconnect/stale responses, storage failure, recovery, serialization, export, bounds, and privacy.
- [x] 3.9 Treat tasks 3.1–3.8 as one breaking vertical slice: do not consider the contract change landed until the mock, both backends, SQL, and canonical conformance compile and agree.
- [x] 3.10 Replace exact three-version recognition with audited wire-profile entries on Apple and Android, require the complete bounded dynamic command-32 semantic baseline before ready, accept bounded extra IDs, and reject missing, renamed, prerelease, or unknown profiles consistently.
- [x] 3.11 Correct dynamic speed normalization without changing the required compact command-10 conversion, and add mirrored regression tests for both units and all supported releases.
- [ ] 3.12 Expand mirrored Swift/Kotlin VESC compatibility through stable 7.00, strictly decode terminated names, UUID, flags, hardware type, and zero test-version marker, tolerate only the bounded optional CRC suffix, and require route plus Refloat capability evidence before ready.

## 4. Interactive scenario workbench

- [x] 4.1 Define and validate the versioned scenario document with the confirmed 65,536-byte document, 16-candidate, 64-CAN-ID, 64-byte-alias, finite-number, allowlist, and atomic-rejection rules.
- [x] 4.2 Implement scenario import/export and deterministic server-private controller identities derived from bounded aliases, with round-trip, privacy, version, size, and atomic-rejection tests.
- [x] 4.3 Extend fixture-dev coordinator state and actions for editable discovery topology, templates as editable starting points, one shared semantic scene, field pins, freeze/resume, and resolved source-family availability without selecting a controller for the product.
- [x] 4.4 Add serialized manual disconnect/re-advertise, reconnect-unchanged, move-controller, replace-controller, ping-visibility, and fragmentation actions that cannot mutate topology ambiguously in flight.
- [x] 4.5 Rework the control surface into topology, telemetry scene, expert override, connection-event, and request/diagnostic areas while retaining the common direct/single-Express workflow.
- [x] 4.6 Add primary/non-primary/cancelled pointer, keyboard, assistive-technology, loading, invalid-document, active-state, browser-reload, and HMR ownership tests for all new controls.
- [x] 4.7 Update fixture documentation with the manual workbench versus conformance-runner boundary, source-command coverage, one-active-controller application rule, and scenario sharing/privacy model.
- [ ] 4.8 Update workbench controls, scenario documents, validation, round trips, and documentation for the audited four-version VESC selector while retaining powered-off atomic configuration rules.

## 5. Shared frontend telemetry experience

- [x] 5.1 Update the backend client and authoritative state store to publish and recover complete detailed live and historical telemetry without platform branching.
- [x] 5.2 Add accessible grouped live presentation for motion, raw electrical load, temperatures, rider input, control contributions, accumulated values, charging, and source availability.
- [x] 5.3 Add explicit live presentation for package state/mode, footpad, wheelslip, darkride, stop condition, setpoint adjustment/pushback, alert reason, and active firmware fault using text and non-color-only distinctions.
- [x] 5.4 Add historical numeric charts and typed transition annotations using the same stable fields, units, nullability, 2,000-point bound, and 2,000-event bound as the contract.
- [x] 5.5 Display actual, requested, filtered, directional, booster, and field-weakening currents distinctly in amperes; do not compute or display utilization.
- [x] 5.6 Add frontend component and interaction tests for full, partial, legacy, unknown-state, fault, pushback, wheelslip, charging, field-weakening, slow-value age, session recovery, pointer cancellation, keyboard activation, and high-frequency updates.

## 6. Canonical conformance and cleanup

- [x] 6.1 Add the six-combination direct/CAN and Refloat-package matrix with exact command 1 fixture compatibility, command 10 and command 31/32 requests, routing, layouts, tails, normalization, nullability, and deferred/unsupported-command assertions.
- [x] 6.2 Add end-to-end Apple and Android conformance for atomic persistence, latest compact timing, decreasing counters, frontend recreation, typed transition events, diagnostics, exports, unknown bounded extensions, failures, reconnect, and controller-change session separation.
- [x] 6.3 Add conformance privacy and every concrete resource-bound assertion for scenario aliases, private UUIDs, field layouts, extensions, state snapshots, storage JSON, diagnostics, session responses, logs, and exports.
- [x] 6.4 Delete the reduced eight-field simulator layout, incorrect command 1 compact-shaped response, partial legacy encoding/decoding branches, discarded known-field paths, and duplicated telemetry serialization after all callers use the normalized pipeline.
- [x] 6.5 Update backend, conformance, fixture, and developer documentation with the supported catalog, commands, units, version differences, simple periodic compact polling, workbench operation, deferred commands, and raw-current-only presentation.
- [x] 6.6 Expand canonical conformance across every audited stable release and both routes, including required-layout acceptance/rejection and exact speed-unit regression coverage.
- [ ] 6.7 Compose every audited Refloat release with VESC 6.02/6.05/6.06/7.00 through both routes and add cross-platform rejection coverage for VESC 7.01 test, unknown, malformed, truncated, unterminated, and unexpected-tail identities.

## 7. Verification

- [x] 7.1 Run focused conformance and fixture-dev type, unit, ownership, encoder, scenario-document, coordinator, and Svelte interaction checks.
- [x] 7.2 Run focused Apple protocol, polling, application-service, SQL, session, loopback, export, and lifecycle tests.
- [x] 7.3 Run focused Android protocol, polling, application-service, SQL, session, loopback, export, and lifecycle tests.
- [x] 7.4 Run focused frontend contract, mock, state, live telemetry, warning, historical detail, and interaction checks.
- [ ] 7.5 Run one consolidated representative physical checkpoint on macOS, Android, and iOS covering direct/CAN, legacy/dynamic, fragmentation, reconnect, warning, field-weakening, controller selection, and background cases assigned by the spec.
- [x] 7.6 Run strict OpenSpec validation, review every normative scenario against implementation coverage, and run `just verify` as the complete software gate.
- [x] 7.7 Run focused compatibility and speed regressions, strict OpenSpec validation, and `just verify` after the audited-profile revision.
- [ ] 7.8 Run focused VESC identity, fixture-workbench, discovery, and compatibility regressions, strict validation of both coordinated changes, and `just verify` after the VESC-profile revision.
