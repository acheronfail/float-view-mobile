## 1. Contract and persistence foundations

- [x] 1.1 Extend `frontend/src/lib/backend/contract.ts` with identifying and controller-selection states, direct/CAN route, sorted bounded candidate and support fields, stable connection issues, same-device retry semantics, and the mandatory `device.selectController` request.
- [x] 1.2 Add nullable `preferredCanId` Board Profile handling and validation to the frontend model, mock backend, Apple persistence, Android persistence, and shared current SQL/schema versions without adding migrations.
- [x] 1.3 Add private automatic learned-controller CAN ID and firmware-UUID persistence on both native backends with public-preference precedence and old-route UUID invalidation when that preference changes, keeping UUIDs outside contract serialization, exports, errors, and default logs.
- [x] 1.4 Separate saved-device ensure/load from session creation so a valid local VESC transport has saved identity and Board Profile during discovery while arbitrary UART peers remain unsaved and sessions begin only at application `ready`.
- [x] 1.5 Make controller selection transactionally merge only `preferredCanId` into the latest Board Profile and add concurrent editor/selection revision tests.
- [x] 1.6 Update contract fixtures and focused frontend/native contract tests for new fields, exact selection error mapping, CAN range validation, connection issues, same-device retry, revision behavior, and 16-candidate bounds.

## 2. Protocol ownership and compatibility families

- [x] 2.1 Extract mirrored Swift and Kotlin VESC frame, firmware identity, and command-routing owners, preserving the full length-checked firmware UUID and supported optional tails.
- [x] 2.2 Implement one direct-or-CAN `ControllerRoute` encoder on each backend and move all controller-bound commands to it, deleting duplicated CAN wrapping and nullable-CAN guards.
- [x] 2.3 Extract Refloat legacy and dynamic codec families on both backends and implement source-faithful INFO, layout, and telemetry decoding for source packages 1.0.0, 1.1.0, and 1.2.3, reporting legacy wire identity as `1.0`.
- [x] 2.4 Add exact VESC 6.02/6.05/6.06 and Refloat legacy-1.0/1.1.0/1.2.3 compatibility catalogs with immutable protocol fixtures and cross-platform focused tests.
- [x] 2.5 Delete broad version predicates and superseded protocol branches after every caller uses the compatibility catalogs and focused codecs.
- [ ] 2.6 Expand the exact VESC catalog through stable 7.00, strictly decode the complete identity baseline and zero test-version marker, correct 6.05 hardware-CRC provenance, keep bounded CRC/name/UUID data out of compatibility keys, and add mirrored Swift/Kotlin acceptance and rejection tests.

## 3. BLE stream hardening

- [x] 3.1 Update Android BLE setup to accept the default ATT MTU, treat larger MTU negotiation as optional, segment ordered writes to the current safe payload, and bound pending bytes and chunks.
- [x] 3.2 Update Apple BLE setup with one bounded service/characteristic/notification deadline and ordered segmentation using the current write-length and supported response mode.
- [x] 3.3 Mirror VESC Tool discovery by observing BLE broadly, publishing only UART-advertising devices or exact saved Apple UUID/Android address matches, and requiring full UART validation after either path.
- [x] 3.4 Preserve and test the narrowly scoped Android missing-CCCD Express compatibility while failing other missing notification capabilities consistently.
- [x] 3.5 Add Apple and Android adapter tests for saved-identifier advertisement bypass versus unknown-device rejection, process-restart rediscovery, default and enlarged payloads, response modes, missing setup elements, setup timeouts, fragmentation, queue bounds, cancellation, and stale callbacks.

## 4. Native controller discovery and telemetry

- [x] 4.1 Implement the generation-based `ControllerDiscovery` state machine on Apple for local firmware classification, saved profile loading, explicit preference then private hint then bounded CAN discovery, capability-based gateways, sorted supported/unsupported candidates, selection validation, issues, retry, and reconnect identity validation.
- [x] 4.2 Implement the matching `ControllerDiscovery` state machine on Android with the same 64-ID/16-candidate bounds, ordering, errors, retry, and direct/configured/hinted/discovered/ambiguous behavior.
- [x] 4.3 Extract Apple telemetry layout and polling into a route-aware owner that supports legacy fixed and dynamic families, serializes requests, and starts only after negotiation completes.
- [x] 4.4 Extract Android telemetry layout and polling into the matching route-aware owner and remove the current non-null-CAN prerequisite.
- [x] 4.5 Wire `device.selectController` through both dispatchers so a current supported candidate merges the preference and resumes exactly once, malformed input returns `invalid_request`, and all well-formed stale/unknown/unsupported selections return `invalid_state`.
- [x] 4.6 Make reconnect on both backends renegotiate and validate route, UUID when meaningful, firmware, Refloat family, and layout before polling resumes; allow a private automatic hint to follow the same UUID to a new CAN ID and otherwise interrupt the old session with `controller_changed` rather than append a replacement controller.
- [x] 4.7 Implement stable failed-state issue serialization and same-device `device.connect` retry on both backends while preserving different-device exclusion and disconnect from identification/selection/failure.
- [x] 4.8 Add native application-service tests for pre-ready saved identity, direct ready/session creation, discovered CAN, private-hint fast path/fallback, configured CAN with absent ping, wrong configured ID, generic gateway forwarding, mixed CAN nodes, discovery overflow, zero/one/multiple candidates, persisted selection, retry, disconnect, and reconnect continuity.

## 5. Shared frontend connection experience

- [x] 5.1 Update the frontend backend client, state store, and mock scenarios to consume the new route, candidate, and connection-state shapes without platform branching.
- [x] 5.2 Display direct or CAN-with-ID connection information in the live firmware/details area while keeping private identifiers hidden.
- [x] 5.3 Add the shared accessible controller-selection modal with candidate count, CAN-ID sorting, available hardware/firmware identity, visible disabled unsupported candidates, guarded primary-pointer and keyboard activation, pending protection, disconnect, and authoritative recovery.
- [x] 5.4 Add a clearable optional preferred CAN ID field and explanatory next-connect behavior to the shared Board Profile editor, with integer `0...253` validation and ordered revision-checked writes.
- [x] 5.5 Add stable connection-issue presentation and same-device retry to the failed live view while retaining configuration, latest-sample dimming, and disconnect.
- [x] 5.6 Add component and interaction tests for direct/CAN display, sorted supported/unsupported candidate prompts, duplicate activation prevention, stale request errors, WebView/state recovery, failed-state configuration/retry, disconnect, and preferred-ID validation/clearing.

## 6. Simulator and canonical conformance

- [x] 6.1 Extend conformance-owned simulator types to compose bounded multiple CAN nodes, distinct supported/unsupported per-controller firmware/Refloat identities and UUIDs, generic gateway identity, non-VESC nodes, duplicate/invalid/excessive ping data, omitted ping IDs, timeouts, and route-specific responses.
- [x] 6.2 Update fixture-dev state, validation, controls, and tests to retain the common single-controller workflow while sharing the source-faithful multi-controller simulator types and removing the direct-telemetry limitation.
- [x] 6.3 Add table-driven software conformance for all eighteen direct/Express, VESC-version, and Refloat-version combinations with legacy source 1.0.0 reported as wire `1.0` and correct legacy/dynamic requests and telemetry.
- [x] 6.4 Add canonical scenarios for automatic CAN discovery, private hint match/fallback, configured CAN without ping, wrong configured CAN, generic gateway forwarding, mixed nodes, zero candidates, ID/candidate overflow, supported/unsupported ambiguity, selection validation/atomic persistence, failed retry, reconnect continuity, and frontend recovery.
- [x] 6.5 Add conformance assertions for pre-ready saved identity without a session, detected route presentation, stable issue codes, exact request errors, candidate ordering/bounds/privacy, ready-session ordering, replacement-controller interruption, direct/CAN polling routes, bounded failure diagnostics, and no unsafe commands.
- [ ] 6.6 Run the full five-topology physical set on Android and macOS; run direct dynamic, Express discovery, and background continuity on iOS; retain identity, ride, and reconnect on all three platforms.
- [ ] 6.7 Expand the composed fixture and software matrix to VESC 6.02/6.05/6.06/7.00, including source-applicable CRC suffixes and fail-closed unknown, malformed, 7.01-test, and nonzero-test-version cases.

## 7. Source cleanup and documentation

- [x] 7.1 Move Apple connection code into `application`, `protocol/vesc`, and `protocol/refloat`, and Android code into its established `backend`, `protocol/vesc`, and `protocol/refloat` counterparts, while retaining one shared Apple backend and one Android application module.
- [x] 7.2 Delete superseded discovery, retry, polling, routing, and compatibility code from monolithic application-service/protocol files and remove any dependency made unused by the extraction.
- [x] 7.3 Add `backends/README.md` documenting mirrored Swift/Kotlin owners, dependency direction, privacy boundaries, and the compatibility-entry-versus-new-codec rule.
- [x] 7.4 Update fixture and developer documentation to describe supported direct/CAN setups, explicit CAN preference, ambiguity selection, and the full declared firmware/package catalog.

## 8. Verification

- [x] 8.1 Run focused frontend type, unit, and interaction checks for the contract, Board Profile editor, live route display, and selection modal.
- [x] 8.2 Run focused Apple and Android protocol, Bluetooth, discovery, polling, persistence, and application-service tests.
- [ ] 8.3 Run the software conformance compatibility/topology matrix and complete the distributed physical BLE checkpoint: full topology set on Android/macOS and direct, Express discovery, and background continuity on iOS.
- [x] 8.4 Run OpenSpec validation and review that every proposal capability, normative scenario, and implementation task remains coherent.
- [x] 8.5 Run `just verify` as the complete software gate.
- [ ] 8.6 Run focused firmware-identity and compatibility regressions, strict validation of both coordinated OpenSpec changes, and `just verify` after the VESC-profile revision.
