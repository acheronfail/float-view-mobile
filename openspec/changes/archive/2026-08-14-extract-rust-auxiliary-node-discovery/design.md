## Context

Apple `AuxiliaryNodeDiscovery.swift` and Android `AuxiliaryNodeDiscovery.kt` are parallel pure state machines. They consume the local decoded firmware identity, an optional complete controller-discovery inventory, and an optional preferred BMS CAN ID; retain one bounded auxiliary generation; and return effects that each native `ApplicationService` executes through the existing request scheduler. They also own the route/status/candidate and last-observation retention rules published by the application owner.

The safe Rust controller-discovery candidate now exposes the same `Route`, `RoutedIdentity`, complete classified inventory, and canonical CAN limits needed by auxiliary discovery. This change extracts the auxiliary decision layer only. It does not select Rust in either host, so production BLE and BMS behavior continue through the existing native state machines.

## Goals / Non-Goals

**Goals:**

- Provide one unsafe-free, dependency-minimal Rust auxiliary-node discovery candidate with total typed transitions.
- Directly reuse controller-discovery routes, routed identities, inventory, and limits rather than defining another cross-component transport model.
- Cover exact stock VESC BMS 6.00 and VBMS32 custom-module 6.06 admission; direct BMS selection; preferred route behavior; complete-inventory reuse; incomplete-inventory CAN enumeration; sequential probes and timeouts; zero/one/many candidates; exact bounds; liveness availability; observation retention/recovery; reset; and stale-generation isolation.
- Keep observation content generic and owned so the future Rust application model can compose this lifecycle without coupling discovery to current Swift/Kotlin snapshot records or borrowed decoder payloads.
- Preserve exact canonical limits: CAN IDs `0...253`, at most 64 normalized IDs, and at most 16 BMS candidates.
- Make the candidate ready for direct composition by a later unified Rust runtime without adding an Apple FFI or Android JNI package.

**Non-Goals:**

- Selecting the candidate in Apple or Android, deleting native auxiliary discovery, or changing the frontend contract.
- Parsing Board Profile JSON, loading `preferredBmsCanId`, constructing BMS requests, decoding firmware or command-96 values, attaching wall-clock observation time, or serializing frontend snapshots.
- BLE, clocks, timers, scheduler execution, polling cadence, controller readiness, sessions, SQLite, loopback serving, files, theme, lifecycle, permissions, or UI behavior.
- Adding code generation, callbacks, opaque handles, foreign records, or another bridge catalogue entry.

## Decisions

### Compose the controller-discovery domain model directly

The new crate will depend only on `float-view-controller-discovery` in production and use its `Route`, `RoutedIdentity`, CAN domain, and complete inventory. It will not define an auxiliary copy of firmware identity or route attribution.

Alternative: depend only on firmware identity and duplicate route/inventory records. Rejected because the future unified runtime already obtains these exact facts from controller discovery, and conversion would preserve migration scaffolding inside safe Rust.

### Use a generic owned observation payload

`AuxiliaryNodeDiscovery<O>` will own `Option<O>` as its last valid observation. Discovery changes only presence and retention: a current valid observation attaches the selected route; unavailability retains it as stale; a later current observation recovers; begin/reset clears it. The crate never inspects, logs, decodes, or serializes `O`.

Alternative: depend on borrowed `vesc-bms-values::BmsValues`. Rejected because decoder values borrow the response payload and intentionally exclude observation time, while lifecycle state must outlive that response. Alternative: copy every BMS field into another owned record. Rejected because the future application-state crate—not discovery—should own that canonical record.

### Return ordered typed effects and generation-stamp all asynchronous work

Transitions return only `PingCan`, `ProbeFirmware`, and `RequestValues` effects carrying the current generation where asynchronous attribution is required. CAN results, firmware results, probe/scan timeout, observations, and unavailability accept the generation and route needed to ignore stale work. No timer, scheduler, request bytes, callback, or host reference crosses the boundary.

Alternative: mirror the native unqualified `markUnavailable` and observation callbacks. Rejected because explicit generation attribution closes a stale-completion race and is required by the future single Rust runtime.

### Fail closed at canonical bounds

The candidate normalizes the finite `0...253` CAN domain, rejects 65 unique IDs before probing, retains at most 16 sorted display-safe BMS candidates, and reports a stable unavailable issue rather than publishing a truncated successful result when a seventeenth BMS exists. Current native implementations classify that untested seventeenth-node edge as ambiguous; the canonical `vesc-bms-telemetry` requirement already says overflow is unavailable, so the Rust candidate follows the canonical contract while production behavior remains unchanged in this extraction.

Alternative: reproduce the native edge exactly. Rejected because it would encode known drift from the canonical bounded-resource requirement into the future owner.

### Treat native behavior plus canonical requirements as the extraction oracle

Shared vectors will cover every behavior asserted by both native unit suites and the canonical direct/preferred/reuse/enumeration routes. Rust-only adversarial tests add all CAN IDs, 64/65 and 16/17 boundaries, malformed routes, reordered/stale work, repeated transitions, generation rollover, observation retention, and diagnostic privacy. Audits prove native files remain selected and no host/build source references the candidate. `just verify` plus the Android/iOS/macOS ESP32 matrix guards production behavior.

## Risks / Trade-offs

- **The repository temporarily has three auxiliary-discovery implementations.** → Keep Rust unselected, audit production isolation, and require direct unified-runtime composition rather than per-platform auxiliary bridges.
- **The generic observation type makes the candidate less concrete in isolation.** → Constrain its role to opaque owned retention and test it with source-derived observation tokens; the later Rust application-state slice supplies the concrete snapshot.
- **A complete controller inventory may include the direct transport plus mixed CAN nodes.** → Accept only routed identities, classify each exact BMS tuple, preserve direct precedence, sort routes deterministically, and reject malformed direct/preferred input without panicking.
- **Native and canonical behavior differ for a seventeenth BMS.** → Encode the canonical fail-closed result explicitly, document the difference, and leave selected hosts untouched until the eventual runtime cutover.
- **Physical iOS lifecycle orchestration may outlive a passing XCTest.** → Bound runner execution, record product evidence separately from harness shutdown, and defer only slow location acquisition as agreed.

## Migration Plan

1. Add and verify the isolated Rust candidate while native auxiliary discovery remains selected.
2. In later increments, compose auxiliary discovery with Rust request scheduling, BMS polling/decoding, application state, persistence, and loopback runtime.
3. Cut over hosts only when the unified runtime executes the complete production path; delete both native auxiliary state machines and temporary protocol bridges in those cutovers.

Rollback is deletion of the unselected crate, shared vectors, focused recipes, and documentation/spec additions; no production package or persisted data changes.

## Open Questions

None for this extraction. The concrete owned Rust BMS snapshot belongs to the later unified application-state slice.
