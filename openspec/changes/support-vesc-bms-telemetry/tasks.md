## 1. Dependency and contract foundation

- [ ] 1.1 Complete or sync `support-direct-and-can-vesc-connections`, then reconcile this change's delta specs against the resulting canonical controller-route, Board Profile, and conformance requirements before implementation.
- [ ] 1.2 Add the TypeScript `BmsStatus`, direct-or-CAN BMS connection, bounded candidate, complete `BmsSnapshot`, and mandatory `ActiveConnection.bms` shapes with fixed units and null semantics.
- [ ] 1.3 Add nullable `preferredBmsCanId` to the frontend Board Profile model, mock defaults and validation, Apple and Android domain models, current SQL/default JSON, and revision-checked persistence without a migration.
- [ ] 1.4 Add matching Swift and Kotlin BMS domain types and explicit loopback serialization, including bounded arrays, nullable optional tail fields, and UUID exclusion.
- [ ] 1.5 Update contract validators and mock authoritative-state scenarios for discovering, not-found, attached, unavailable-with-stale-snapshot, and ambiguous BMS states.

## 2. Connection-wide VESC request scheduling

- [ ] 2.1 Define mirrored typed request, expected-response, priority, route-context, deadline, cancellation-generation, and bounded-queue models for the Apple and Android VESC request schedulers.
- [ ] 2.2 Implement the Apple connection-wide scheduler as the sole post-UART VESC write owner with one in-flight request, deterministic priority, timeout completion, and stale-generation rejection.
- [ ] 2.3 Implement the matching Android scheduler with equivalent queue bounds, priorities, deadlines, response attribution, cancellation, and diagnostics.
- [ ] 2.4 Route controller firmware discovery, CAN ping, Refloat INFO/layout/telemetry, and current-limit requests through the scheduler on Apple and remove direct competing writes and the global pending-firmware CAN attribution.
- [ ] 2.5 Route the matching Android controller, Refloat, and limits work through its scheduler and remove superseded write and pending-route state.
- [ ] 2.6 Add scheduler tests on both platforms for controller-before-auxiliary priority, riding telemetry before BMS, sequential firmware route attribution, timeout advancement, queue overflow, reconnect cancellation, and late responses.
- [ ] 2.7 Re-run existing controller discovery, reconnect, polling cadence, and session-continuity tests to prove the scheduler refactor preserves behavior before enabling BMS requests.

## 3. VBMS protocol identity and values codecs

- [ ] 3.1 Add `COMM_BMS_GET_VALUES(96)` to the Swift and Kotlin VESC command enums and use the generic direct/CAN command router for exact `[96]` and `[34, id, 96]` payloads.
- [ ] 3.2 Refactor the Apple firmware decoder into a bounded common prefix plus strict motor, VBMS 6.00, and other-hardware tails while preserving current motor compatibility and private UUID behavior.
- [ ] 3.3 Implement the matching Kotlin hardware-specific firmware decoder and keep every accepted/rejected identity fixture equivalent to Swift.
- [ ] 3.4 Implement the Apple BMS values decoder for the exact legacy and extended variants, 50-cell/50-temperature bounds, boolean balancing, finite values, route-ID validation, 40-byte status, and no partial results.
- [ ] 3.5 Implement the matching Kotlin BMS values decoder with identical scales, bounds, tail handling, error classification, and output fields.
- [ ] 3.6 Add immutable source-derived VBMS firmware and legacy/extended value fixtures with provenance under the canonical backend fixtures.
- [ ] 3.7 Add table-driven Swift and Kotlin protocol tests for minimum, typical, and maximum valid values plus every count, truncation, flag, non-finite, CAN mismatch, status, and remainder failure.

## 4. Auxiliary discovery and live polling

- [ ] 4.1 Implement the Apple `AuxiliaryNodeDiscovery` generation owner for local VBMS selection, reusable complete inventories, low-priority auxiliary enumeration, 64-ID/16-candidate bounds, preferred direct probes, unique selection, no-BMS, ambiguity, and unavailable states.
- [ ] 4.2 Implement the matching Android auxiliary discovery owner with the same ordering, bounds, state transitions, preference behavior, and controller-lifecycle independence.
- [ ] 4.3 Extend controller discovery on both platforms to publish structurally valid classified inventory information without transferring controller selection, compatibility, failure, or continuity ownership.
- [ ] 4.4 Wire local direct-VBMS endpoints so the BMS route is direct while controller discovery continues through CAN as a gateway.
- [ ] 4.5 Apply `preferredBmsCanId` immediately to only the active auxiliary generation on both platforms, directly probe configured IDs omitted from ping, preserve controller/session state, and resume automatic discovery when cleared.
- [ ] 4.6 Implement mirrored BMS polling owners that request scheduler slots at approximately one hertz, publish attached only after a direct response, retain freshness, become unavailable after three consecutive failures, and recover on a valid response.
- [ ] 4.7 Integrate complete BMS state into Apple and Android authoritative snapshots and conflated events while bypassing session persistence, sample counts, aggregates, diagnostics, exports, and SQLite telemetry tables.
- [ ] 4.8 Add native application tests for direct controller plus CAN VBMS, Express plus controller/VBMS, direct VBMS plus CAN controller, no BMS, unique BMS, configured omission, wrong preference, multiple ambiguity, overflow, loss, recovery, reconnect, preference changes, device replacement, privacy, and no unsafe commands.

## 5. Shared frontend BMS experience

- [ ] 5.1 Extend the frontend state store and mock backend to consume complete BMS snapshots and status transitions without platform branches or local history accumulation.
- [ ] 5.2 Add `preferredBmsCanId` to the shared Board Profile editor with clearable integer `0...253` validation, independence from `preferredCanId`, next-state explanation, and revision-conflict recovery.
- [ ] 5.3 Add the conditional live-menu BMS action only after identity detection, retaining it for attached, unavailable, and ambiguous states and omitting it for no detected identity and `not_found`.
- [ ] 5.4 Build the responsive BMS sheet/popover with route, status, freshness, summary health, cells and balancing, temperatures, environment, counters, optional tail fields, unavailable placeholders, and unit-aware presentation.
- [ ] 5.5 Add ambiguity guidance and sorted candidates that direct the user to `preferredBmsCanId`, plus explicit stale presentation that retains the original observed time while unavailable.
- [ ] 5.6 Implement primary-pointer activation, cancellation, guarded keyboard/assistive click behavior, native button/dialog semantics, focus containment/restoration, touch action, dismissal, and duplicate-activation protection.
- [ ] 5.7 Add frontend component and interaction tests for menu visibility, attached/ambiguous/unavailable sheets, maximum arrays, freshness changes, unit display, preference validation, WebView recovery, primary/non-primary pointers, keyboard use, focus, and double activation.

## 6. Simulator and canonical conformance

- [ ] 6.1 Extend conformance-owned typed topology models with local and CAN VBMS identities, zero/one/multiple nodes, independent controller/BMS routes, legacy/extended values, omitted ping IDs, and bounded failure modes.
- [ ] 6.2 Implement source-faithful VBMS firmware and values encoders from the immutable fixtures and classify command 96 as an allowlisted read-only request only on the matching selected route.
- [ ] 6.3 Add canonical software scenarios for direct-controller-plus-VBMS, Express-plus-controller-and-VBMS, direct-VBMS-plus-controller, no BMS, unique automatic selection, configured selection without ping, wrong preference, ambiguity, candidate/ID overflow, loss, recovery, reconnect, and disconnect.
- [ ] 6.4 Add malformed-response tables for excessive counts, truncation at every variable section, invalid balancing, non-finite decoding inputs, reported-ID mismatch, optional-tail variants, status termination/bounds, and unexpected remainder.
- [ ] 6.5 Assert exact request serialization and priority, non-blocking readiness/session continuity, authoritative state/event bounds, stale freshness, privacy, no unsafe commands, and absence of BMS fields from persistence, diagnostics, review, and exports.
- [ ] 6.6 Extend the interactive fixture coordinator and types to compose zero/one/multiple VBMS nodes and direct-VBMS topology without adding behavior to ESP32 firmware.
- [ ] 6.7 Add deterministic plausible BMS generation, legacy/extended selection, timeout/malformed modes, counters, bounded diagnostics, and atomic powered-off validation to the fixture development server.
- [ ] 6.8 Add accessible fixture controls and tests for VBMS topology, IDs, values, failure modes, active configuration lock, atomic save, request counters, pointer/keyboard activation, and cleanup.
- [ ] 6.9 Run the explicit physical checkpoint for available direct-controller-plus-CAN-VBMS, Express-plus-CAN-VBMS, and direct-VBMS-gateway topologies on Android and Apple targets, recording any unavailable hardware combination rather than weakening software coverage.

## 7. Cleanup and documentation

- [ ] 7.1 Document the connection-wide scheduler, auxiliary discovery, BMS protocol owners, dependency direction, live-only boundary, privacy rules, and procedure for auditing another VBMS wire version in `backends/README.md`.
- [ ] 7.2 Update fixture and developer documentation with supported BMS topologies, `preferredBmsCanId`, ambiguity behavior, the conditional BMS sheet, and the explicit physical checkpoint.
- [ ] 7.3 Delete superseded controller-only parsing assumptions, direct write paths, duplicate route types, unused timers, and compatibility code made obsolete by the scheduler and hardware-typed inventory.
- [ ] 7.4 Review contract, native, frontend, fixture, and conformance code for accidental BMS persistence, cached-controller liveness inference, UUID disclosure, unsafe commands, or unbounded collections.

## 8. Verification

- [ ] 8.1 Run focused frontend type, component, interaction, contract, and mock tests for BMS state, Board Profile, menu, sheet, freshness, bounds, and accessibility.
- [ ] 8.2 Run focused Apple and Android scheduler, firmware, BMS codec, auxiliary discovery, polling, application-service, lifecycle, privacy, and persistence-exclusion tests.
- [ ] 8.3 Run the software conformance VBMS topology, malformed-values, routing, priority, lifecycle, safety, and live-only matrices and the interactive-fixture test suite.
- [ ] 8.4 Run strict OpenSpec validation for this change after reconciling its dependency and review every proposal capability and normative scenario against the task list.
- [ ] 8.5 Run `just verify` as the complete software gate.
