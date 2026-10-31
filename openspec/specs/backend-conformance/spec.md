# Backend Conformance Specification

## Purpose

Define canonical product, protocol, persistence and native-boundary coverage for the selected shared backend.

Completed extraction, intermediate native ownership and per-protocol bridge
acceptance are historical. Original requirements and evidence remain in
[archived changes](../../changes/archive/); ongoing checks follow
[AGENTS.md](../../../AGENTS.md#verification).

## Requirements

### Requirement: Verification follows change impact

Ongoing changes SHALL run relevant checks for changed behavior and affected consumers, following the root [verification policy](../../../AGENTS.md#verification). Focused tests, type/lint checks, builds and integration scenarios SHALL be selected by dependency and behavioral impact. `just verify` and `just test all` SHALL remain available as comprehensive checks, but SHALL NOT be mandatory for every edit, commit, spec sync or archive. Broader checks SHALL be selected when cross-cutting impact, uncertainty or failures justify them, or when explicitly requested.

This requirement supersedes recurring full-suite, all-platform packaging and physical-gate interpretations elsewhere in the canonical specs. Acceptance gates for completed Rust extractions, refactors, host cutovers and other completed changes describe their original acceptance only; they SHALL NOT be replayed for subsequent maintenance. Archived plans and recorded evidence remain historical. Behavioral requirements and test coverage SHALL remain intact.

#### Scenario: Frontend-only edit

- **WHEN** a change affects frontend presentation or interaction without changing the backend contract or native integration
- **THEN** relevant frontend tests, type/lint checks and affected browser/build checks suffice, without Rust, native packaging, emulator/simulator or physical suites

#### Scenario: Shared implementation or contract edit

- **WHEN** a change affects Rust behavior, shared types, public projection or a host boundary
- **THEN** verification includes affected crates and consumers plus relevant canonical and host integration coverage, rather than only tests adjacent to edited files

#### Scenario: Documentation-only edit

- **WHEN** a change only revises documentation, specifications or planning artifacts
- **THEN** document consistency, links, commands and applicable strict OpenSpec validation are checked without application builds or runtime suites

#### Scenario: Complete a scoped change

- **WHEN** relevant checks pass and no required coverage remains outstanding
- **THEN** the change can complete and its specs can be synced or archived without running `just verify` or producing a routine physical-test skip justification

### Requirement: One canonical suite

The repository SHALL contain one top-level `/conformance` TypeScript black-box suite that imports the hand-written frontend contract and is not copied into Swift or Kotlin implementations.

#### Scenario: Common execution

- **WHEN** conformance runs against mock, Android, iOS, or macOS
- **THEN** the same scenario and assertion implementations are used

### Requirement: Initial identification recovery parity

The canonical software suite SHALL verify that Apple and Android implement the same bounded recovery behavior for a missing initial direct firmware response, including generation cancellation and retry exhaustion.

#### Scenario: Dropped first response recovers on both backends

- **WHEN** the simulated BLE peer drops the first direct firmware response and returns a valid response to the recovery request
- **THEN** each native backend reaches `ready` after exactly one transport connection and two direct firmware writes

#### Scenario: Recovery remains bounded and generation-safe

- **WHEN** both direct firmware responses are absent or the connection generation changes before the recovery callback
- **THEN** conformance observes no third request, no stale state mutation, and no durable session before valid identification completes

### Requirement: Complete contract coverage

The canonical suite SHALL cover every mandatory WebSocket control method including standalone `session.list`, pending-delete cancel, and prune preview/start/cancel, session-data HTTP request kind, result shape, state event, stable error, authentication rule, ordering invariant, lifecycle behavior, persistence behavior, and resource bound visible to the frontend. A required scenario MUST create its prerequisite device, Board Profile, session, history revision, snapshot, cursor, prune job, or session-list page and MUST fail rather than silently skip its assertions when that state is absent. Historical assertions SHALL use `session.get` metadata plus `series`, `samples`, and `events`; `session.getDiagnostics`, embedded `SessionDetail`, and session-group assertions SHALL NOT remain.

#### Scenario: Contract operation added

- **WHEN** a new frontend-visible control method or session-data request kind is introduced
- **THEN** the change is incomplete until canonical success and relevant authentication, failure, cancellation, and bound scenarios cover it

#### Scenario: Backend drift

- **WHEN** a backend returns a structurally or behaviorally incompatible WebSocket or HTTP result
- **THEN** canonical conformance fails with the first actionable mismatch

#### Scenario: Missing scenario resource

- **WHEN** a success scenario requires a device, Board Profile, session, snapshot, cursor, or session-list page that setup did not create
- **THEN** canonical conformance fails with an actionable setup error instead of omitting the operation

#### Scenario: Standalone destructive product operations

- **WHEN** canonical conformance creates disposable standalone sessions
- **THEN** it exercises single/batch pruning and query/deletion coordination without creating or unlinking a session group

### Requirement: Minimal target adapters

Each target adapter SHALL only build or locate its application, prepare isolated test state, launch it, expose or refresh bootstrap readiness, apply uniform foreground/background controls where relevant, and stop it. Contract requests and assertions MUST remain in the shared suite.

#### Scenario: Platform launch

- **WHEN** conformance selects a target
- **THEN** platform-specific code returns a usable endpoint without duplicating scenario logic

#### Scenario: Mobile frontend recreation

- **WHEN** a shared scenario backgrounds and foregrounds a mobile target
- **THEN** its adapter performs only the OS lifecycle control and endpoint refresh while shared TypeScript performs every assertion

### Requirement: Software and hardware execution

The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Each change SHALL select software checks by affected behavior and consumers. Physical suites SHALL be required only for affected OS adapters, permissions, lifecycle/background behavior, radio timing, or firmware/wire interoperability that software coverage cannot establish. Physical coverage SHALL be scoped to relevant platforms, routes and scenarios; unrelated hardware matrices SHALL NOT be required. Results and missing required coverage SHALL be reported honestly; software/simulator evidence SHALL NOT be presented as physical evidence. Hardware scenarios SHALL exercise the production Bluetooth, protocol, application-service, and storage paths selected by each platform host. Component tests MUST NOT present an unselected candidate as a production path.

#### Scenario: Optional comprehensive software check

- **WHEN** `just verify` runs without physical hardware
- **THEN** the frontend, mock, native code, deterministic software conformance, and every retained Rust component's locked formatting, warnings-denied linting, unit tests, and parity tests are verified

#### Scenario: Rust candidate parity

- **WHEN** the VESC framing crate is present without an Apple or Android host bridge
- **THEN** the software gate compares its valid, malformed, fragmented, noisy, multiple-frame, CRC, and size-bound behavior with immutable source-derived evidence without adding a backend target adapter

#### Scenario: Physical BLE gate

- **WHEN** a hardware conformance command runs with a configured ESP32 fixture
- **THEN** the same relevant contract scenarios drive the backend through its real OS Bluetooth stack and selected production implementations

#### Scenario: Production path

- **WHEN** physical conformance launches a Debug application with a test-only endpoint seam
- **THEN** Bluetooth, VESC protocol, application-service, session, and SQLite behavior use the implementations selected for that host's production build rather than a test-only candidate

#### Scenario: Shared product or frontend change

- **WHEN** a change affects shared product policy, persistence, or frontend behavior without changing an OS or hardware interaction that needs physical evidence
- **THEN** passing the affected software checks satisfies verification without physical testing or a separate hardware-skip report

#### Scenario: Hardware-sensitive change

- **WHEN** a change affects behavior whose correctness depends on the real OS/device/firmware interaction
- **THEN** the change identifies and runs the relevant physical matrix, and reports unavailable hardware or unperformed checks explicitly

### Requirement: Deterministic state synchronization

The shared conformance client SHALL wait for bounded predicates over monotonic authoritative state revisions and SHALL be able to bootstrap a replacement WebSocket after mobile frontend recreation without restarting the native backend or fixture runner.

#### Scenario: Asynchronous connection transition

- **WHEN** a connection request is accepted before the backend reaches `ready`
- **THEN** conformance waits for a newer state satisfying the expected transition rather than using an unbounded sleep

#### Scenario: iOS foreground recovery

- **WHEN** iOS stops its loopback frontend server while backgrounded and starts it again in the foreground
- **THEN** conformance obtains current bootstrap credentials, opens a replacement WebSocket, and verifies the preserved native state

### Requirement: Conformance-owned VESC simulation

`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed BLE-transport and one-or-more CAN-controller identity/topology/Refloat configurations, telemetry-layout fields and phases, fragmentation, cadence assertions, disconnect injection, CAN discovery omissions and failures, and bounded failure diagnostics. Exact response encoding SHALL be checked against immutable protocol fixtures. The ESP32 firmware MUST NOT duplicate those behaviors. Software-host VESC simulation SHALL also be owned by conformance and reused through thin test adapters; native hosts MUST NOT retain their own protocol response models. Software selection SHALL require an explicit conformance build and authenticated run configuration, and MUST remain unavailable in ordinary and physical builds.

#### Scenario: Real BLE request and deterministic response

- **WHEN** the application sends a VESC request over real BLE and the firmware reports `WRITE <HEX>`
- **THEN** canonical conformance classifies the request, selects the configured deterministic response, and supplies exact `NOTIFY <HEX>` bytes to the firmware

#### Scenario: Hardware variant

- **WHEN** a conformance scenario selects a different typed board configuration
- **THEN** the same fixture firmware presents the selected identity and topology through conformance-supplied VESC responses

#### Scenario: Express controller discovery

- **WHEN** a backend queries an Express transport followed by its CAN-connected VESC controllers
- **THEN** conformance supplies distinct transport and controller firmware, Refloat identity, telemetry layout, and telemetry values in the expected routed request order

#### Scenario: Multiple controller candidates

- **WHEN** an Express configuration contains multiple VESC motor controllers and optional non-VESC CAN nodes
- **THEN** conformance returns the configured ping and per-ID firmware responses and can verify that only display-safe VESC candidates reach authoritative state

#### Scenario: Software host parity

- **WHEN** a software host runs canonical conformance
- **THEN** it uses the shared independent simulation models while the selected production service and protocol implementation handle application requests

#### Scenario: Explicit software build

- **WHEN** an emulator or simulator launches an ordinary build without conformance authorization
- **THEN** its device type alone cannot select a simulated peripheral

### Requirement: Concurrent fixture orchestration

One bounded fixture runner SHALL own serial setup, the `RESET`/`READY` handshake, diagnostic LEDs, advertising, GATT-write processing, simulator responses, notification fragmentation, disconnect injection, reconnect advertising, cadence verification, and cleanup while shared contract requests execute concurrently.

#### Scenario: Connect requires fixture responses

- **WHEN** `device.connect` waits for native firmware and Refloat discovery
- **THEN** the fixture runner continues processing `WRITE` events and returning `NOTIFY` bytes until the backend reaches its expected state

#### Scenario: Reconnect injection

- **WHEN** a scenario commands `DISCONNECT` after telemetry has begun
- **THEN** the runner restarts advertising, services the reconnect identity sequence, and preserves bounded diagnostics across the transition

#### Scenario: Runner failure

- **WHEN** serial, BLE, framing, command safety, cadence, or scenario behavior fails
- **THEN** the runner records the first useful violation, sets the failure LED, and performs bounded target, fixture, serial, and LED cleanup

### Requirement: Read-only physical simulation

Canonical physical conformance SHALL permit required VESC identity, discovery, telemetry, only the exact route-correct Refloat product-control requests defined by `refloat-live-controls`, and exact eligible VESC-core shutdown `[156, 0, 0]`. It SHALL reject unknown, forced-shutdown, restart, broadcast-shutdown, motor-control, firmware, arbitrary persistent-configuration, tuning, recording, history, Handtest, remote, and RC-move commands. Product controls SHALL mutate only disposable simulated Refloat runtime/configuration state; shutdown SHALL simulate target disappearance without powering off the ESP32; neither path may issue motor movement on the ESP32 or host.

#### Scenario: Allowlisted Refloat control

- **WHEN** a writable physical scenario receives a version-correct package, flywheel, lighting, or clear-fatal request through the selected route
- **THEN** conformance applies the bounded simulated state transition and supplies the exact source-defined response or later confirmation

#### Scenario: Mutating VESC command

- **WHEN** a backend sends a command capable of motor motion, firmware mutation, arbitrary configuration, or another mutation outside the product-control allowlist
- **THEN** conformance fails the scenario, supplies no response, and does not mutate simulated state

#### Scenario: Allowlisted controller shutdown

- **WHEN** an eligible direct or CAN-routed physical scenario receives exact `[156, 0, 0]`
- **THEN** conformance records one accepted shutdown intent and simulates selected-controller disappearance without sending a motor command or powering off the ESP32

### Requirement: Conformance owns hardware dependencies

Any host serial dependency required to control the physical fixture SHALL belong to `/conformance` and MUST NOT be installed by `/frontend` or implemented inside the ESP-IDF project.

#### Scenario: Frontend setup

- **WHEN** a developer installs or runs the frontend without hardware conformance
- **THEN** no hardware serial dependency is required by the frontend application

### Requirement: Supported-backend gate

Android, iOS, and macOS SHALL NOT be considered supported unless their current build passes the canonical suite.

#### Scenario: Release verification

- **WHEN** a supported platform build is prepared
- **THEN** its required canonical conformance result is successful

### Requirement: Canonical physical scenario matrix

Physical conformance SHALL run identity, ride, and unexpected-disconnect/reconnect scenarios on Android, iOS, and macOS. Android, iOS, and macOS SHALL each run the complete representative topology set: a direct dynamic controller, direct legacy controller, automatically discovered Express controller, explicitly configured Express controller whose ping does not identify it, and Express connection requiring user selection. Android and iOS SHALL also run mobile background continuity. macOS backgrounding SHALL NOT be required because mobile OS suspension is the behavior under test.

#### Scenario: Identity

- **WHEN** any physical target scans and connects to the advertised fixture
- **THEN** it reaches `ready`, exposes the configured direct or CAN route and supported controller and Refloat identity, sends only allowed startup requests, and completes an explicit disconnect

#### Scenario: Ride

- **WHEN** any physical target receives idle, riding, and post-ride telemetry phases with configured fragmentation
- **THEN** conformance verifies route-correct request cadence, automatic durable-session state, metadata/data review, export, and deletion through the shared contract

#### Scenario: Unexpected disconnect and reconnect

- **WHEN** any physical target loses the fixture connection after telemetry begins and the fixture advertises again
- **THEN** the backend reconnects, revalidates its controller route, resumes identity and telemetry, preserves the same active session, and later completes it through explicit disconnect

#### Scenario: Reconnect finds another controller

- **WHEN** the fixture returns a different meaningful controller UUID during an active-session reconnect
- **THEN** the backend interrupts the old session, reports `controller_changed`, and does not append replacement-controller telemetry

#### Scenario: Explicit CAN ID without ping

- **WHEN** the Board Profile configures the simulated controller ID and Express CAN ping omits that ID
- **THEN** the target probes the configured route directly and reaches `ready` without selecting a different controller

#### Scenario: Multiple-controller selection

- **WHEN** Express discovery reports two VESC motor controllers
- **THEN** conformance observes both candidates, selects one through the shared contract, verifies the chosen route and persisted preference, and verifies its reuse after reconnect

#### Scenario: Mobile background continuity

- **WHEN** Android or iOS is backgrounded while a physical ride session is active
- **THEN** native polling and session work continue where the OS permits, and a foregrounded replacement frontend recovers the same connection and session with additional samples

#### Scenario: macOS physical coverage

- **WHEN** macOS physical conformance runs
- **THEN** it runs identity, ride, reconnect, and the complete representative topology set through the shared Apple backend and CoreBluetooth without a synthetic mobile-background scenario

#### Scenario: iOS physical coverage

- **WHEN** iOS physical conformance runs
- **THEN** it runs identity, ride, reconnect, direct dynamic, Express discovery, and background continuity without repeating all macOS shared-backend topology cases

### Requirement: Bounded diagnostics

Conformance SHALL bound execution time, captured messages, hardware transcripts, retries, and output. Passing runs SHALL not retain historical evidence artifacts; failures SHALL report the first useful violation and bounded diagnostic context.

#### Scenario: Physical progress

- **WHEN** a physical conformance suite runs
- **THEN** it reports the active scenario, meaningful classified fixture and lifecycle milestones, bounded telemetry progress, scenario timing, and cleanup without dumping every raw packet

#### Scenario: Scenario failure

- **WHEN** a backend or fixture violates an expectation
- **THEN** the runner stops within its deadline and reports the first violation with bounded context, and a secondary cleanup failure does not replace or indefinitely delay it

### Requirement: Conformance uses the native port registry

Canonical target adapters SHALL use macOS port `11010`, iOS port `11011`, and Android port `11012`. Mobile target adapters MUST forward the same port number on the host and device and MUST remove only their own forwarding rule during cleanup.

#### Scenario: macOS conformance endpoint

- **WHEN** conformance starts the macOS target
- **THEN** readiness and authenticated WebSocket access use `127.0.0.1:11010`

#### Scenario: iOS conformance forwarding

- **WHEN** conformance starts an iOS target
- **THEN** `iproxy` exposes host port `11011` to device loopback port `11011`

#### Scenario: Android conformance forwarding

- **WHEN** conformance starts an Android target
- **THEN** ADB exposes host port `11012` to device loopback port `11012`

#### Scenario: Forwarding cleanup

- **WHEN** a mobile conformance target stops or fails
- **THEN** its adapter removes the platform-specific same-number forwarding rule without disturbing another platform endpoint

### Requirement: Exhaustive connection compatibility coverage

Software conformance SHALL cover the Cartesian product of direct and Express topology, audited VESC 6.02/6.05/6.06/7.00, and representative Refloat 1.0.0/1.1.0/1.2.3. It SHALL additionally cover the complete VESC identity baseline, source-applicable CRC suffix, audited nonzero firmware test-version guarded writes, rejected unsupported-version writes, malformed and unknown identities, and configured, absent, invalid, duplicate, zero, one, and multiple CAN candidates without requiring a separate simulator implementation per combination.

#### Scenario: Supported version matrix

- **WHEN** the software compatibility matrix runs
- **THEN** all twenty-four topology/controller/package combinations establish the expected route and protocol family, report legacy source package 1.0.0 as wire version `1.0`, and produce route-correct telemetry

#### Scenario: VESC identity profile fixtures

- **WHEN** conformance supplies complete VESC 6.02 identity without a CRC and complete 6.05, 6.06, and 7.00 identities with their bounded source-defined CRC suffix
- **THEN** both native backends derive the same stable version, hardware type, names, and private UUID and select the same `vescCoreReadOnly` profile

#### Scenario: Audited test-version access

- **WHEN** conformance supplies a known 6.02/6.05/6.06/7.00 identity with a nonzero test-version marker
- **THEN** both backends reach ready after route and Refloat validation, permit read requests, and permit writes through the same capability and safety checks as release firmware

#### Scenario: Unsupported VESC release evidence

- **WHEN** conformance supplies VESC 7.01 test identity, an unknown major/minor pair, truncated baseline, unterminated name, or unexpected remainder
- **THEN** both backends fail closed consistently, do not request Refloat telemetry, and do not create a durable session

#### Scenario: Wrong configured ID

- **WHEN** a configured CAN ID is absent, times out, or reports non-VESC hardware
- **THEN** conformance verifies bounded failure and no fallback to another candidate

#### Scenario: Ambiguous selection recovery

- **WHEN** the frontend connection is recreated while multiple candidates await selection
- **THEN** conformance verifies the candidates recover through `state.get` and a current selection resumes the connection exactly once

#### Scenario: Candidate ordering and support

- **WHEN** firmware probes discover supported and unsupported VESC motor controllers out of CAN-ID order
- **THEN** conformance verifies all motor controllers appear sorted by CAN ID, unsupported entries cannot be selected, and no supported entry is silently chosen

#### Scenario: Discovery bounds

- **WHEN** simulation exceeds 64 normalized CAN IDs or 16 motor controllers
- **THEN** conformance verifies the matching stable issue, no misleading truncated candidate state, and no unbounded probing

#### Scenario: Selection profile merge

- **WHEN** another Board Profile field changes after candidates are published and a selection is accepted
- **THEN** conformance verifies only `preferredCanId` is merged into the latest profile and its revision advances once

#### Scenario: Stable selection errors

- **WHEN** conformance sends malformed, wrong-device, unsupported-candidate, stale, or duplicate controller selections
- **THEN** both backends return `invalid_request` only for malformed input and `invalid_state` for every well-formed invalid-state selection

#### Scenario: Failed same-device retry

- **WHEN** a failed connection's preferred CAN ID is corrected and `device.connect` names the same device
- **THEN** conformance verifies a new generation clears the old issue and uses the latest Board Profile

#### Scenario: Same controller moves CAN ID

- **WHEN** a private automatic hint fails during active-session reconnect and discovery finds the same meaningful UUID at another CAN ID
- **THEN** conformance verifies the hint and route update, the same session resumes, and no ambiguity prompt is shown

### Requirement: BLE setup and stream coverage

Software backend tests SHALL cover VESC Tool-style saved-identifier advertisement allowlisting, default and enlarged BLE write payloads, supported write response modes, fragmented frames and notifications, missing setup elements, setup timeouts, bounded queues, and stale lifecycle callbacks.

#### Scenario: Default payload fragmentation

- **WHEN** an outbound VESC frame exceeds the default BLE payload
- **THEN** tests verify ordered safe chunks reassemble to exactly one original frame

#### Scenario: Setup timeout

- **WHEN** service discovery or notification enablement never completes
- **THEN** tests verify deterministic failure within the setup deadline and bounded retained diagnostics

#### Scenario: Stale notification

- **WHEN** an old connection generation supplies bytes after reconnect begins
- **THEN** tests verify those bytes cannot complete identification or mutate the new connection state

#### Scenario: Saved advertisement exception

- **WHEN** unfiltered BLE observation receives one exact saved identifier and one unknown identifier without UART service advertisements
- **THEN** tests verify only the saved device is published and both still require actual UART service validation before connection setup succeeds

#### Scenario: Saved identity before session

- **WHEN** a first-time peer supplies valid local VESC firmware and later requires controller selection
- **THEN** tests verify saved identity and profile exist during selection while no durable session exists until application `ready`

### Requirement: Detailed telemetry compatibility matrix

Software conformance SHALL exercise complete detailed telemetry for every audited stable Refloat release in 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 through direct and CAN routes and across audited VESC 6.02, 6.05, 6.06, and 7.00. It SHALL verify the complete VESC identity baseline, audited nonzero test-version guarded write access, source-applicable bounded hardware-CRC suffix, exact command 1 fixture compatibility, command 10 detail levels and metres/second conversion, command 31/32 bytes and native kilometres/hour values, route wrapping, required-field negotiation, version-applicable active state and tails, normalization, nullability, unknown bounded extensions, and bounded source-correct command-35 alert reads while excluding data-recorder and unsupported write requests from either backend.

#### Scenario: Audited route and package combinations

- **WHEN** the detailed telemetry matrix runs
- **THEN** each audited stable VESC/controller-package pair through direct and CAN routing produces its exact protocol profiles and equivalent normalized semantic values where the source carries them

#### Scenario: Stable VESC identity families

- **WHEN** the matrix supplies 6.02 without a CRC suffix and 6.05, 6.06, or 7.00 with the source-defined bounded suffix
- **THEN** Apple and Android select the same `vescCoreReadOnly` profile, ignore CRC value for compatibility, and proceed to route and Refloat capability validation

#### Scenario: Audited test-version write access

- **WHEN** conformance supplies a known 6.02/6.05/6.06/7.00 identity with a nonzero test-version marker
- **THEN** Apple and Android complete Refloat negotiation, allow read requests and guarded writes, and expose read/write access beside the VESC and Refloat versions

#### Scenario: Controller compatibility rejection

- **WHEN** conformance supplies VESC 7.01 with a nonzero test-version marker, another unknown release, or a malformed identity baseline
- **THEN** Apple and Android reject the identity with `unsupported_controller`, reject malformed identity within the bounded identification deadline, and never begin Refloat negotiation or durable session creation

#### Scenario: Compatibility rejection

- **WHEN** conformance supplies an unknown version, an unaudited prerelease version, or a dynamic layout missing a required semantic ID
- **THEN** Apple and Android reject it before ready with the same unsupported-Refloat outcome

#### Scenario: Slow compact polling

- **WHEN** a dynamic package is configured to expose slow compact values
- **THEN** conformance verifies one route-correct detail-level-4 command 10 becomes due each second, never overlaps the one in-flight request, does not retry immediately after timeout, and merges into later realtime samples with its own `slowObservedAt`

### Requirement: Detailed persistence and contract coverage

Canonical conformance SHALL verify that both backends commit, recover, publish, serialize, range-query, page, diagnose through exact retained samples, prune, and export every typed detailed field, observed discrete transition, and representative bounded extension with stable names, units, nullability, continuity, revision, and privacy. It SHALL cover dedicated stable columns, write-time field presence, authoritative sample/event and recovery-summary markers, recorded/stored counts, source evidence, catalog reconciliation, storage failure, frontend recreation, reconnect, selected-controller change, active snapshot growth, session interruption, decreasing device counters, and unknown future enum values.

#### Scenario: End-to-end detailed sample

- **WHEN** the simulator supplies a sample containing representative fields from every semantic group
- **THEN** live state, published metadata, series, exact samples, transition pages, and export agree on committed sequences and values on Apple and Android

#### Scenario: One controller per session

- **WHEN** a product selects a different discovered controller after recording detailed telemetry
- **THEN** conformance observes the old session end, route-scoped telemetry clear, and the new controller contribute only to a new session and snapshot

#### Scenario: Privacy and composed bounds

- **WHEN** detailed telemetry, scenario aliases, learned identities, oversized fields, extension limits, series field/bucket products, sample pages, event pages, and decoded HTTP bodies are exercised
- **THEN** conformance observes the 65,536-byte WebSocket, 64-field layout, 64-byte key, 4,096-byte aggregate-name, 32-extension, 16,384-byte request/extension JSON, 16-request-field, 1,024-bucket, 65,536-observation, 2,000-sample-page, 500-event-page, and 4,194,304-decoded-byte bounds with no private controller UUID or token in state, history, logs, or errors

#### Scenario: Interrupted publication

- **WHEN** a test seam stops the process after the session marker commits but before catalog publication
- **THEN** restart reconciles the catalog and returns matching sample/event boundaries, counts, elapsed coverage, fields, riding/distance/maxima summaries, and query results before exposing the recovered session

#### Scenario: In-process publication failure

- **WHEN** a test seam fails the catalog update after the session transaction commits without stopping the process
- **THEN** no newer state is published, another append or metadata read first attempts reconciliation, and an unreconciled session reports a stable storage failure

### Requirement: Workbench software verification

The software gate SHALL test scenario-document validation and round trips, deterministic private identity derivation, topology editing, semantic scenes, overrides, version projections, manual disconnect and identity transitions, fragmentation, coordinator serialization, diagnostics bounds, and accessible control activation without requiring an ESP32 or native device.

#### Scenario: Scenario document round trip

- **WHEN** a valid complex scenario is exported and imported
- **THEN** software verification observes equivalent public configuration and wire behavior but no exported private UUID

#### Scenario: Scene projection matrix

- **WHEN** representative semantic scenes are encoded through each supported Refloat family
- **THEN** exact fixtures and decoders agree on present fields, absent fields, state flags, and response bounds

### Requirement: Representative physical detailed telemetry checkpoint

The physical BLE checkpoint SHALL exercise detailed telemetry on Android and macOS with at least one direct and one CAN-routed dynamic scene, one legacy scene, response fragmentation, a manual reconnect event, and representative warning and field-weakening values. iOS SHALL exercise one dynamic detailed scene and background recovery through the shared Apple backend without duplicating the full macOS matrix.

#### Scenario: Physical workbench scene

- **WHEN** a developer selects a detailed riding or warning scene in `just fixture dev` and operates the native product
- **THEN** the platform reaches ready, displays and persists the expected normalized fields, survives the required lifecycle event, and retrieves them after disconnect

### Requirement: Standalone session catalog conformance

The canonical suite SHALL exercise `session.list` against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify bounded first pages, deterministic `(startedAt, sessionId)` ordering, duplicate-free keyset continuation beyond 50 sessions, catalog-revision cursor conflicts, bounded complete state snapshots without recent-session or group collections, and unchanged catalog revision across telemetry-only state events. It SHALL also verify that automatic-linking settings and session-group methods, payloads, tables, and fixtures are absent.

#### Scenario: Page continuation beyond fifty sessions

- **WHEN** a disposable catalog contains more than one bounded session-list page
- **THEN** every target returns identical standalone summaries in deterministic order and reaches all rows through continuation without offset pagination or a total-history cap of 50

#### Scenario: Stale catalog cursor

- **WHEN** a catalog-visible session mutation occurs after a cursor is issued
- **THEN** every target rejects continuation with the same bounded conflict and the next first-page request succeeds

#### Scenario: Telemetry-only state change

- **WHEN** a persisted telemetry sample publishes a newer complete state snapshot without changing catalog-visible metadata
- **THEN** the snapshot remains bounded, contains no session collection or group payload, and leaves `sessionCatalogRevision` unchanged

#### Scenario: Grouping surfaces are removed

- **WHEN** conformance inspects settings, control methods, state, storage recreation, and mock fixtures
- **THEN** no automatic-linking field, session-group method, linked-ride payload, group table, or group fixture is exposed

### Requirement: Canonical session-data conformance

The canonical black-box suite SHALL exercise the same session-data HTTP requests against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify bootstrap discovery and development URL resolution, token and exact-Origin authentication before body/session processing, no-store responses, gzip decoding, explicit columnar shapes and parallel-array lengths, field/range minimization, published boundaries, stable error mappings, cursors, request/decoded-response bounds, cancellation cleanup, and the continued 65,536-byte WebSocket limit without duplicating assertions in target adapters.

#### Scenario: Cross-platform data response

- **WHEN** canonical conformance sends one valid series, samples, or events request to any supported target
- **THEN** the decoded response satisfies the same hand-written contract and immutable expected result

#### Scenario: Unauthenticated HTTP query

- **WHEN** a query has an invalid token or Origin and names an existing or missing session
- **THEN** every target returns indistinguishable rejection before body parsing/session lookup and exposes no private identifier or token

#### Scenario: Bounds and cleanup

- **WHEN** conformance exercises oversized, malformed, cancelled, concurrent, invalid-range, invalid-field, future-boundary, and cross-request-cursor cases
- **THEN** every target remains within resource/shape limits, stays cancellable without a hard correctness deadline, releases cancelled work, and remains responsive to a later valid request and durable append

#### Scenario: Decoded stream overflow

- **WHEN** the canonical client receives a gzip body that expands beyond the decoded bound
- **THEN** it aborts before JSON parsing or complete retention and reports a bounded invalid-backend failure

### Requirement: Immutable historical aggregation and pagination vectors

The mock, shared Rust history owner and canonical conformance SHALL consume one immutable platform-neutral vector set for exact reduction, pagination and rejection behavior.

#### Scenario: Apply immutable history vectors

- **WHEN** history aggregation or pagination vectors run
- **THEN** conformance SHALL own immutable platform-neutral vectors containing ordered raw samples, stable and extension fields, null and time-gap continuity cases, ride phases, session-captured persistence intervals, pruning resolution, published revisions/boundaries, bounded requests, exact expected M4 segments, and keyset pages. The frontend mock, shared Rust reducer/reader and canonical suite SHALL consume those vectors. Vectors SHALL cover checked bucket boundaries, extrema order and deduplication, duplicate elapsed values, sequence ties, nulls at and across bucket edges, alternating gaps, configured and derived time gaps, snapshot/revision exclusion, late fields, dense sample windows, event pages, and invalid cursor replay.

#### Scenario: Extrema order

- **WHEN** minimum and maximum observations occur in either order inside a bucket
- **THEN** every implementation emits them once in original `(elapsedMs, sequence)` order with the same expected segments

#### Scenario: Bounded alternating gaps

- **WHEN** valid and null observations alternate within one bucket and across the next bucket boundary
- **THEN** every implementation remains within four observations per bucket and emits no segment that bridges a null generation

#### Scenario: Configured and automatic time gaps

- **WHEN** the same rows are reduced with an explicit seconds threshold and with automatic phase/resolution policy
- **THEN** every implementation emits the exact expected continuity generations for each captured policy

#### Scenario: Snapshot exclusion

- **WHEN** fixture rows, fields, or events exist beyond the request's published boundaries
- **THEN** every implementation excludes them from metadata, series, samples, and event results

#### Scenario: Page continuation

- **WHEN** a fixture window exceeds one sample or event page
- **THEN** every implementation returns identical chronological, duplicate-free pages and rejects a cursor rebound to another request

### Requirement: Canonical SQLite storage and reader verification

Native verification SHALL create the shared current schema verbatim and SHALL inspect representative query plans and lifecycle behavior for both hosts. It SHALL verify dedicated stable-column persistence, bounded extension decoding, field-presence discovery after late samples, marker/catalog publication order, in-process and restart reconciliation, history/prune metadata, incremental auto-vacuum, read-only WAL snapshots, writer non-obstruction, cancellability, two-read concurrency, cancellation-release and ordinary-completion target measurements, and reader coordination with deletion/export/pruning/recovery. Series and sample-page query plans MUST use an elapsed index without avoidable full-range sorting or temporary ordering.

#### Scenario: Requested stable columns only

- **WHEN** a long complete-catalog session is queried for two stable fields
- **THEN** the checked plan and result use only the ordering/snapshot columns and those dedicated numeric columns without decoding stable `detail_json`

#### Scenario: Active writer and cancelled reader

- **WHEN** a long historical reader is cancelled while an active session append becomes due
- **THEN** the append completes within its existing persistence bound and all reader transaction, connection, lease, reducer, and compression resources are released

#### Scenario: Delete with an open lease

- **WHEN** deletion races a completed-session query
- **THEN** no session files are removed beneath an open connection and no new reader starts after pending deletion

#### Scenario: Slow cancellation

- **WHEN** a loaded reference device exceeds the cancellation-release or ordinary-completion target
- **THEN** verification records stage/resource diagnostics while requiring the operation to remain cancellable and preventing stale publication or unsafe file mutation

### Requirement: Cross-input chart and tooltip conformance

Frontend verification SHALL exercise the shared chart viewport and synchronous per-chart tooltip model with mouse, touch, pen, keyboard, and assistive-technology inputs. It SHALL cover hover, synchronized chart inspection, background pan threshold, two-touch pinch where only one contact is primary, vertical page scrolling, pointer cancellation/lost capture, visible controls, edge positioning, and guarded activation without double action.

#### Scenario: Desktop hover and mobile touch equivalence

- **WHEN** mouse hover and touch tap target the same elapsed position in equivalent chart state
- **THEN** both paths use the same loaded series position and present equivalent elapsed, label, formatted value, and unit semantics

#### Scenario: Gesture arbitration

- **WHEN** tests distinguish plot tap, background horizontal drag, crosshair-handle drag, two-touch pinch, non-primary one-pointer input, and browser-owned vertical scroll
- **THEN** each input produces only its specified selection, pan, inspection, zoom, no-op, or cancellation behavior

### Requirement: Representative long-session performance gate

Verification SHALL include a deterministic 150,000-sample complete-catalog session, a larger pathological resource-bound session, and physical checkpoints on an iPhone 12 and Samsung Galaxy S9. Each checkpoint SHALL record exact OS/WebView versions, cache state, native query/reduction/gzip time, cancellation-release time, response bytes, frontend decode/path/render time, SVG path/observation counts, and interaction long tasks. With an empty session-feature cache, metadata to first useful visible-group SVG SHALL target one second in each of at least three recorded runs. An uncached settled zoom or pan SHALL target 250 milliseconds. Current bounded paths SHALL provide gesture feedback before that settled fetch, active persistence cadence SHALL remain within its existing bound, and every measured operation SHALL remain cancellable. A target miss SHALL produce retained stage diagnostics rather than a correctness timeout.

#### Scenario: Mobile overview budget

- **WHEN** each reference device opens the deterministic long session with an empty feature cache
- **THEN** it reports whether a useful first visible SVG met the one-second target plus bounded response, cache, path, observation, and stage measurements

#### Scenario: Mobile zoom budget

- **WHEN** the target settles an uncached zoom or pan over that session
- **THEN** verification reports whether the newest requested range met the 250-millisecond target and requires that superseded responses never subsequently replace it

#### Scenario: Immediate gesture feedback

- **WHEN** the user continuously pans or pinches before a new range response is available
- **THEN** the existing bounded SVG transforms on animation frames and diagnostics expose any interaction-blocking long task

#### Scenario: Native scan target miss

- **WHEN** the dedicated-column native scan misses its allocated initial-overview budget before HTTP or UI implementation proceeds
- **THEN** the miss and stage profile are retained for review, repeated material misses trigger a persisted-aggregate design decision, and the individual run is not failed solely by elapsed time

### Requirement: Canonical pruning conformance

The mock, shared Rust pruner and canonical black-box targets SHALL consume immutable pruning vectors covering exact retained sequences, protected evidence, counts, revisions, cancellation and recovery.

#### Scenario: Apply immutable pruning vectors

- **WHEN** canonical prune vectors run
- **THEN** conformance SHALL apply immutable pruning vectors to the mock, shared Rust and decoded black-box targets. Vectors SHALL define elapsed-zero bucket alignment, first/last rows, first non-null field sources, per-field minimum/maximum roles, greedy maximum-coverage selection, stable field and extension ordering, nulls, multi-role rows, phase-boundary pairs, event sources, earliest-tied summary sources, the eight ordinary-row cap, aligned coarser re-pruning, recorded/stored counts, and expected history revisions. Canonical scenarios SHALL cover preview, one ride, all rides with a captured upper session key, rides completed after batch start, ineligible skips, cancellation before commit, cancellation between rides, crash recovery, stale snapshots/cursors, exports, bounded selector/temp-file resources, WAL checkpoint, and interrupted incremental compaction.

#### Scenario: Deterministic capped selection

- **WHEN** a bucket offers more than eight ordinary source candidates across many numeric fields
- **THEN** every implementation retains the same at-most-eight rows after protected evidence and reports the same uncovered roles

#### Scenario: Atomic cancellation and batch progress

- **WHEN** cancellation or process interruption occurs at each transaction and between-ride seam
- **THEN** every ride is either wholly old or wholly revised, completed batch work is not repeated, and bounded progress reaches the same recoverable state

#### Scenario: Logical prune before compaction

- **WHEN** pruning commits and incremental vacuum is interrupted
- **THEN** exact retained history, revision, events, summaries, and export disclosure remain correct while reclaimable pages stay resumable

#### Scenario: Bounded pruning resources

- **WHEN** preview and pruning process a pathological raw ride and are cancelled at selection, checkpoint, and vacuum seams
- **THEN** measured native memory remains independent of raw sequence count, temporary keep storage is removed, and actual file reclamation remains distinguishable from its preview estimate

#### Scenario: Stale consumer after prune

- **WHEN** a frontend uses a pre-prune snapshot, cursor, cache response, or export read after the history revision advances
- **THEN** every target rejects or discards it and requires refreshed metadata without mixing removed and retained rows

### Requirement: Location contract coverage

The canonical suite SHALL cover the location setting, permission/status actions, stable errors, state boundary metadata, authenticated `locations` pages, cursor/limit bounds, stale snapshots, and board-only fallback for the mock and every supported native backend.

#### Scenario: Permission matrix

- **WHEN** conformance exercises not-determined, granted, denied, restricted/unavailable, and settings-required fixtures
- **THEN** every target returns the same bounded status/action semantics and board logging remains usable

#### Scenario: Location query

- **WHEN** conformance requests a valid page, an over-limit page, and a stale-snapshot page
- **THEN** it verifies ordered bounded results, explicit gaps, continuation behavior, and the expected stable error

### Requirement: Location persistence and projection coverage

The canonical suite SHALL verify monotonic location sequencing, elapsed-time alignment, accuracy preservation, durable snapshot boundaries, restart recovery, provisional-session deletion, pruning, explicit gaps, deterministic nearest-row CSV tie-breaking, blank unmatched CSV fields, and independent archive-track retention.

#### Scenario: Sparse location track

- **WHEN** a fixture emits telemetry at a higher cadence than location observations with a deliberate gap
- **THEN** conformance verifies no fabricated coordinates, correct gap metadata, and correct chart/map/CSV time projection

#### Scenario: Recovery boundary

- **WHEN** the backend is interrupted during a location append and restarted
- **THEN** conformance verifies that only the last complete location boundary is readable and board telemetry remains consistent

### Requirement: Background and frontend recreation coverage

Physical iOS and Android conformance SHALL include an authorized location-enabled `riding` state, a mobile background transition, fixture telemetry during the transition, foreground/rebootstrap, and verification that native location rows and boundaries continued without a WebView-owned recorder.

#### Scenario: Android background checkpoint

- **WHEN** Android backgrounds an active location-enabled session and then returns to the foreground
- **THEN** the shared suite reads additional location rows and the same session boundary after bootstrap refresh

#### Scenario: iOS background checkpoint

- **WHEN** iOS backgrounds an active location-enabled session and then returns to the foreground
- **THEN** the shared suite reads preserved native state and additional location rows when Core Location delivered them, or the documented bounded unavailable/gap result

### Requirement: Privacy and battery diagnostics

Conformance SHALL assert that coordinates do not appear in complete state events or default diagnostics and SHALL record bounded provider/update counts for battery-policy verification without requiring a user-configurable interval.

#### Scenario: Coordinate redaction

- **WHEN** conformance captures state events and failure diagnostics after location writes
- **THEN** no latitude, longitude, or full location payload appears outside an explicit session-data/export assertion

#### Scenario: Update count

- **WHEN** a session enters and exits the location-eligible state
- **THEN** provider/update accounting shows capture started and stopped within bounded lifecycle timing and no idle-only updates were persisted

### Requirement: Saved and guest tracking conformance

The canonical suite SHALL exercise explicit saved and guest connection choices against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify known saved-board direct connection, unknown-board save-and-connect, unknown-board guest connection, cancellation before connection, reconnect mode retention, bounded live state, and the absence of guest durable side effects.

#### Scenario: Unknown board guest isolation

- **WHEN** conformance connects an unknown fixture board as guest, publishes live telemetry, location opportunities, and controller selection, then disconnects
- **THEN** no saved-device, Board Profile, controller hint, session, sample, location, catalog revision, exportable ride record, or Sessions entry is created

#### Scenario: Unknown board save and connect

- **WHEN** conformance connects the same fixture board with save-and-connect
- **THEN** the saved board is available, the ready state creates the normal provisional session, accepted samples and location follow saved-mode persistence rules, and the board appears in the saved-board index

#### Scenario: Guest reconnect retention

- **WHEN** a guest fixture connection fails or reconnects and the frontend is recreated
- **THEN** the active connection retains guest mode, live state recovers, and no durable state is promoted

#### Scenario: Unknown-board cancellation

- **WHEN** the frontend cancels the unknown-board choice before `device.connect`
- **THEN** no native connection generation starts and the fixture remains discoverable

### Requirement: Board-scoped session catalog conformance

The canonical suite SHALL verify the required saved-device filter on `session.list`, deterministic board-scoped keyset continuation, bounded per-board summary projection, stale filter/cursor conflicts, missing-filter rejection, local-date grouping inputs, and session-detail return metadata. It SHALL verify that global `All sessions` UI data and an unfiltered backend query are not required and that guest activity never appears in any board-scoped result.

#### Scenario: Board filter and continuation

- **WHEN** conformance creates sessions for at least two saved boards and requests successive pages for one board
- **THEN** every target returns only that board's sessions in deterministic order, continues without duplicates, and keeps the cursor bound to the board filter and catalog revision

#### Scenario: Board summary projection

- **WHEN** conformance queries a board with retained sessions, deleted sessions, and no sessions
- **THEN** each target returns equivalent bounded count, riding-time, distance, and most-recent-session values with defined unavailable handling and no maximum-speed or active/in-progress count

#### Scenario: Guest absent from history

- **WHEN** a guest fixture produces the same telemetry as a saved fixture
- **THEN** board summaries, board-scoped session pages, and catalog revisions are unchanged by guest activity

#### Scenario: Stale board cursor

- **WHEN** a board-scoped cursor is reused after catalog mutation or with another board identifier
- **THEN** every target returns the same bounded conflict and the next first-page request succeeds

### Requirement: Archive identity and board lifecycle conformance

The canonical suite SHALL verify stable opaque board identity, archive import association, configurable undiscovered boards, Home-only connection initiation, and irreversible board forgetting across the mock and native targets. It SHALL verify that portable archives omit raw VESC firmware identifiers, Apple UUIDs, Android addresses, and private controller-route UUIDs.

#### Scenario: Archive merges into existing board

- **WHEN** a session archive is imported after a saved board with the same `deviceHistoryId` already exists
- **THEN** every target attaches the imported sessions to that board without creating a duplicate

#### Scenario: Archive creates configurable board

- **WHEN** a session archive is imported for an unknown `deviceHistoryId`
- **THEN** every target creates a board entry with no local platform association, exposes it in Sessions, and makes it configurable despite zero local discovery

#### Scenario: Invalid archive identity

- **WHEN** conformance imports an archive with a missing or malformed `deviceHistoryId`
- **THEN** every target rejects the archive before creating a board or inserting a session

#### Scenario: Home owns connection initiation

- **WHEN** an imported board is absent from a Home scan
- **THEN** its history and configuration work without starting Bluetooth, and only an explicit Home save-and-connect operation can bind a local association

#### Scenario: Forget board cleanup

- **WHEN** conformance confirms Forget board for an inactive board
- **THEN** every target removes that board's profile, private routing metadata, and all owned session files/catalog rows while preserving other boards

#### Scenario: Active board cannot be forgotten

- **WHEN** conformance requests Forget board while the board is actively connected
- **THEN** every target returns `invalid_state` and preserves the board and all sessions

### Requirement: Canonical VBMS simulation ownership

`conformance/` SHALL own source-faithful local and CAN-routed stock VESC BMS 6.00 and VESC Express VBMS32 6.06 identity, values encoding, topology composition, request classification, timeouts, malformed responses, and deterministic scenario behavior. The programmable ESP32 firmware MUST remain a bounded byte and event bridge and MUST NOT interpret BMS state or choose responses.

#### Scenario: Source-faithful VBMS fixtures

- **WHEN** software or physical conformance answers VBMS firmware and values requests
- **THEN** it uses immutable payload fixtures derived from pinned `vesc_bms_fw` and `vesc_express` encoders and validates shared Rust and simulator decoding against the same bytes

#### Scenario: Routed BMS request

- **WHEN** a scenario places a VBMS at CAN ID `42`
- **THEN** conformance responds only to a command-96 request forwarded exactly once to ID `42` and records other routes as bounded unexpected requests

#### Scenario: Unsafe BMS command

- **WHEN** a backend sends a BMS mutation, bootloader, configuration, or firmware-write command
- **THEN** canonical conformance fails the scenario and the fixture performs no mutation

### Requirement: Complete VBMS topology and lifecycle matrix

Canonical conformance SHALL exercise standalone direct VBMS, direct-controller-plus-VBMS, and gateway-plus-controller-and-VBMS compositions with zero, one, configured, omitted-from-ping, and multiple auxiliary BMS nodes. It SHALL verify that a direct exact VBMS becomes BMS-only without controller work, while auxiliary BMS behavior never controls board-controller readiness or session continuity.

#### Scenario: Standalone direct VBMS

- **WHEN** the BLE transport itself presents an exact supported VBMS identity
- **THEN** both backends reach ready with BMS device kind, poll values directly, publish equivalent decoded values, and create no controller route, Refloat state, telemetry sample, location, or session

#### Scenario: Unique auxiliary BMS

- **WHEN** each supported controller topology contains one live CAN-routed VBMS
- **THEN** both backends establish the same controller route, become ready, select the expected independent BMS route, and publish equivalent decoded values

#### Scenario: Multiple BMS nodes without preference

- **WHEN** two valid auxiliary BMS identities are present and `preferredBmsCanId` is null
- **THEN** both backends publish the same sorted ambiguity candidates, send no BMS values request, and continue controller telemetry

#### Scenario: Multiple BMS nodes with preference

- **WHEN** two valid auxiliary BMS identities are present and `preferredBmsCanId` names one of them
- **THEN** both backends directly probe and poll only that exact CAN route without requiring it in ping results

#### Scenario: BMS loss during ride

- **WHEN** an auxiliary BMS stops responding during a persisted riding session
- **THEN** conformance observes attached-to-unavailable BMS state after the bounded threshold while controller samples and the same session continue

#### Scenario: BMS recovery and reconnect

- **WHEN** a standalone or auxiliary BMS later responds or its BLE transport reconnects
- **THEN** conformance observes generation-safe revalidation and a fresh attached snapshot without creating or replacing a session for standalone BMS mode

### Requirement: Standalone BMS negative-work verification

The canonical suite SHALL verify Apple and Android BMS-only connections against the same source-faithful direct VBMS fixture and SHALL reject any CAN discovery, forwarding, controller, Refloat, mutation, or durable-session work from that connection.

#### Scenario: Direct BMS request sequence

- **WHEN** software conformance connects to the standalone VBMS scenario
- **THEN** its bounded request transcript contains direct firmware identification and direct read-only BMS values requests but no CAN ping, forwarded packet, controller telemetry, Refloat, motor configuration, shutdown, or BMS mutation command

#### Scenario: BMS-only generation cancellation

- **WHEN** standalone BMS polling completes after explicit disconnect or a replacement generation begins
- **THEN** conformance observes no stale BMS snapshot, session, or navigation-driving device kind from the cancelled generation

### Requirement: Bounded BMS failure and contract assertions

Conformance SHALL cover maximum and excessive cell and temperature counts, invalid balancing flags, truncation at every variable section, non-finite values, CAN-ID mismatch, legacy and extended tails, status termination and bounds, response timeouts, candidate overflow, state-event bounds, privacy, and live-only persistence.

#### Scenario: Malformed values table

- **WHEN** table-driven cases supply each malformed count, value, flag, route, or remainder
- **THEN** Apple and Android reject the same cases without publishing partial values or blocking controller work

#### Scenario: Maximum complete values

- **WHEN** a valid response contains 50 cells, 50 temperatures, and the maximum bounded extended status
- **THEN** both backends publish equivalent complete state within the contract message bound and the frontend renders it without an unbounded collection

#### Scenario: Live-only verification

- **WHEN** multiple BMS snapshots occur during a ride followed by review, diagnostics, export, and deletion
- **THEN** conformance verifies state revisions and latest live replacement while session sample shapes, counts, aggregates, stored rows, diagnostics, and export contents contain no BMS additions

#### Scenario: Privacy verification

- **WHEN** a VBMS identity contains a meaningful UUID
- **THEN** canonical state, errors, bounded diagnostics, ordinary logs, and exports contain no UUID bytes or textual encoding

### Requirement: Canonical Refloat control coverage

Canonical conformance SHALL cover `refloat.control.set`, `refloat.fatalError.clear`, and every `RefloatControls` field against mock, Apple, and Android. Software scenarios SHALL exercise representative Refloat 1.0.0, 1.1.0, and 1.2.3 profiles across direct and CAN routes; ready and reconnecting lifecycle; saved and guest tracking; normal, flywheel, and read-only modes; idle and riding state; capability absence; idempotence; competing requests; timeouts; malformed responses; stale generations; frontend recreation; and exact state revisions. The flywheel matrix SHALL prove that every device mutation except exact flywheel disable emits no packet and that flywheel disable remains reachable while active flywheel run state is normalized as riding. Physical coverage SHALL include one direct and one CAN-routed writable control sequence per native backend without requiring motor movement.

#### Scenario: Versioned software matrix

- **WHEN** the Refloat control matrix runs across representative audited packages and routes
- **THEN** both native backends emit the exact package, flywheel, legacy-or-stable lighting, and version-applicable fatal command bytes and publish equivalent confirmed state

#### Scenario: Safety rejection matrix

- **WHEN** conformance requests controls while read-only, reconnecting, riding, in an incompatible package mode, without capability, during an active fault, or while another action is pending
- **THEN** Apple and Android return the same stable error, emit no prohibited command, and preserve the last confirmed controls, except that exact flywheel disable remains permitted from active flywheel mode

#### Scenario: Flywheel mutation matrix

- **WHEN** conformance requests package, lighting, fatal-clear, or another mutating operation while flywheel mode is active and its run state is normalized as riding
- **THEN** mock, Apple, Android, and the fixture reject every other mutation without transmission, while flywheel disable emits only the exact command 22 bytes

#### Scenario: Frontend recovery

- **WHEN** conformance recreates the frontend during a pending action or after an external simulated lighting change
- **THEN** replacement state recovers the native pending or newly confirmed values without duplicate command transmission

#### Scenario: Control command bounds

- **WHEN** control responses are truncated, oversized, routed from the wrong CAN ID, delayed past timeout, or delivered after a generation change
- **THEN** the matching action fails or the response is discarded with bounded diagnostics and no stale state publication

### Requirement: Canonical controller shutdown coverage

Canonical conformance SHALL cover `controller.shutdown` and complete controller-action state against mock, Apple, and Android. It SHALL exercise VESC 6.02 exclusion; stable writable 6.05, 6.06, and 7.00 acceptance; test-build rejection; idle and non-idle state; normal and flywheel mode; direct and CAN routes; exact payload bounds; a competing Refloat mutation; controller or transport loss; continued connectivity after an unsupported-hardware no-op; stale generations; and exact state revisions. Physical validation SHALL separately verify actual shutdown and manual power recovery on supported controller hardware.

#### Scenario: Versioned shutdown matrix

- **WHEN** the shutdown matrix runs across audited VESC versions and direct or CAN routes
- **THEN** only eligible 6.05, 6.06, and 7.00 scenarios emit exactly one `[156, 0, 0]` payload through the selected route

#### Scenario: Shutdown safety matrix

- **WHEN** conformance requests shutdown while non-idle, read-only, reconnecting, in flywheel mode, stale, or competing with another mutation
- **THEN** mock, Apple, Android, and the fixture reject it equivalently without transmission

#### Scenario: Shutdown lifecycle matrix

- **WHEN** an accepted shutdown is followed either by target loss or continued connectivity
- **THEN** target loss suppresses automatic reconnect while continued connectivity clears pending intent without claiming power-off

### Requirement: Canonical Refloat configuration coverage

Canonical conformance SHALL cover every configuration/Tune method, state field, shared codec, safety decision, scheduler interaction, persistence rule and frontend recovery against mock and both native hosts.

#### Scenario: Exercise complete configuration matrix

- **WHEN** configuration and Tune regression cases run
- **THEN** canonical conformance SHALL cover every Refloat configuration and Tune contract method, state-summary field, shared codec, safety decision, scheduler interaction, persistence rule, and frontend recovery behavior against mock, Apple, and Android. Immutable source-derived fixtures SHALL include exact compressed schemas, normalized metadata, full-schema fingerprints, wire signatures, current/default payloads, and valid writes for every audited stable Refloat 1.0.0–1.2.3 release. Software scenarios SHALL cover legacy 1.0 patch resolution and signature collision, direct and CAN routes, cache hit/miss/corruption, paging, bounds, malformed XML and values, stale revisions/generations, semantic-control contention, write acknowledgement/readback, reconnect, read-only firmware, exact-version isolation, package/configuration convergence, runtime-versus-persistent lighting, and flywheel-mode rejection of every mutation except eligible flywheel disable. They SHALL also prove that command 93 is withheld during flywheel and a fresh normal-mode read is required after exit.

#### Scenario: Exact release matrix

- **WHEN** configuration vectors run for each audited stable Refloat release
- **THEN** both hosts expose the shared codec’s exact normalized schemas, signatures, typed values, encoded writes, and strict errors from the same upstream-derived fixtures

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

#### Scenario: Flywheel isolation matrix

- **WHEN** flywheel mode begins with loaded current values and mock, Apple, Android, and fixture receive current-read and mutation requests
- **THEN** every target invalidates current values, withholds command 93, emits no mutation except eligible command 22 flywheel disable, and requires a fresh normal-mode current read after exit

#### Scenario: Overlapping-state convergence

- **WHEN** package state changes through command 12 or a verified full write changes package or persistent lighting fields
- **THEN** every target invalidates or refreshes the affected configuration state, refreshes semantic state from authoritative telemetry/response, and never equates persistent lighting defaults with runtime overrides

### Requirement: Physical configuration checkpoint

Physical conformance SHALL perform a bounded non-motor Refloat configuration checkpoint for representative legacy and dynamic packages through direct and CAN routes on each native backend. It SHALL capture current values, read defaults without writing, change one fixture-safe non-actuating value through the complete write/readback flow, and restore and verify the captured starting configuration before completion. Failure to restore SHALL fail the checkpoint with actionable bounded output.

#### Scenario: Safe physical round trip

- **WHEN** a writable fixture runs the configuration checkpoint
- **THEN** the backend reads schema/current/default, verifies one safe write, restores the original complete values, and causes no motor movement

#### Scenario: Interrupted restore

- **WHEN** connectivity is lost before original values are verified
- **THEN** the checkpoint fails explicitly and does not claim cleanup or success

### Requirement: Cross-platform flywheel confirmation regression

Canonical verification SHALL exercise equivalent Apple and Android application-service lifecycles for flywheel enable, repeated flywheel telemetry during pending disable, normal-mode disable confirmation, prohibited competing mutation cancellation, and absent/stale failure callbacks. Affected shared software and direct/CAN physical control scenarios SHALL pass when required by change impact without confirmation timeout, duplicate completion, mutation safety relaxation, or motor movement.

#### Scenario: Native service parity

- **WHEN** focused native tests inject the same command transmission and authoritative telemetry sequence
- **THEN** Apple and Android publish equivalent pending and confirmed states and return the same successful enable/disable results

#### Scenario: Affected physical control regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on affected hosts
- **THEN** each exercised host completes enable and disable confirmation without fixture rejection, timeout, or motor movement

### Requirement: Lighting query and mutation distinction

Canonical conformance SHALL prove that the shared Refloat simulator distinguishes zero-mask lighting status queries from nonzero-mask lighting mutations across representative legacy and stable releases, direct and CAN routes, and normal and flywheel modes.

#### Scenario: Retain query and mutation safety evidence

- **WHEN** the simulator processes lighting queries and mutations
- **THEN** evidence SHALL include exact query responses, unchanged disposable state, retained wrong-route and malformed rejection, test-version guarded write behavior, and retained flywheel rejection for every actual mutation. Affected physical checks SHALL exercise the relevant direct/CAN writable control routes without timing waivers.

#### Scenario: Deterministic query matrix

- **WHEN** software conformance sends source-correct zero-mask lighting requests through representative supported versions and routes
- **THEN** each request receives the current bounded lighting response without state mutation or a safety diagnostic

#### Scenario: Flywheel race regression

- **WHEN** a scheduled lighting refresh occurs between flywheel enable and disable in the canonical control sequence
- **THEN** the query succeeds, flywheel disable remains reachable, and every nonzero-mask competing mutation remains rejected

#### Scenario: Affected physical regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on affected hosts
- **THEN** each exercised host completes its route-correct sequence without fixture rejection or motor movement

### Requirement: Refloat 1.3.0 selectable telemetry evidence

The repository SHALL prove exact Refloat 1.3.0 compatibility, complete bounded command-33 behavior, configuration identity, retained control behavior, older-release isolation, and shared production-host composition using immutable source-derived fixtures, focused safe-component tests, canonical conformance, root audits, and affected software/physical checks.

#### Scenario: Cover complete selectable behavior

- **WHEN** selectable telemetry regression vectors run
- **THEN** evidence MUST cover every documented non-reserved command-33 mask field in float32 form, echoed request identity, exact variable response extents, direct/CAN routes, fast/slow merge, generation replacement, persistence/export, limited alert semantics, configuration read/write/readback, and malformed/adversarial inputs without requesting unstable command 35.

#### Scenario: Exercise complete command-33 field coverage

- **WHEN** focused codec and service vectors enumerate individual and combined fast, slow, and capability-gated GNSS masks for exact Refloat 1.3.0
- **THEN** every documented non-reserved field is decoded with its source type and unit, existing typed fields and canonical bounded extensions are populated exactly, and unavailable internal-only fields remain null

#### Scenario: Reject malformed selectable responses

- **WHEN** tests supply unsupported flags or mask bits, mismatched echoed flags/masks, truncation at every boundary, trailing bytes, non-finite values, invalid states, impossible coordinates, stale routes, or replacement generations
- **THEN** no partial or stale sample is published, persisted, exported, or used as mutation safety evidence and only bounded payload-free diagnostics are produced

#### Scenario: Preserve current alert boundary

- **WHEN** exact Refloat 1.3.0 reports alert reason and fatal latch through command 33 and an active firmware fault through the retained bounded command-10 probe
- **THEN** live state preserves the existing alert reason, active-fault, latch, and fatal-clear behavior without command 35, alert history, alert names, or persisted fatal-latch history

#### Scenario: Prove configuration and Tune isolation

- **WHEN** generated 1.3.0 schema/value fixtures exercise direct and CAN schema paging, current/default reads, an eligible write, mandatory readback, cache identity, save, list, and apply
- **THEN** only the exact audited 1.3.0 fingerprint and signature are accepted and no saved configuration is migrated or offered across the 1.2.3/1.3.0 boundary

#### Scenario: Preserve older releases

- **WHEN** the canonical software matrix replays every audited Refloat 1.0.0–1.2.3 fixture after 1.3.0 support is selected
- **THEN** legacy command 10 and negotiated commands 31/32 retain their exact requests, decoding, controls, configuration, session, and failure behavior

#### Scenario: Exercise selected production composition

- **WHEN** affected software or physical host checks exercise Refloat 1.3.0
- **THEN** they use the selected shared runtime without a superseded host decoder and retain the required route, interoperability and cleanup assertions

### Requirement: Shared server regression coverage

Canonical tests SHALL cover shared authentication, malformed/oversized input, ordering, slow/multiple clients, cancellation, lifecycle recovery and archive cleanup. Component tests SHALL cover hostile handles/envelopes, generation races and bounded shutdown. Native packaging checks apply when that boundary changes.

#### Scenario: Exercise each selected host

- **WHEN** canonical software scenarios run through the selected shared server
- **THEN** they exercise that host's selected shared server and fail on missing prerequisites or contract drift

#### Scenario: Verify platform packaging

- **WHEN** clean Apple host/device/simulator and Android host/four-ABI Debug/Release builds run
- **THEN** the required shared server symbols and dependencies are present within the existing binding artifacts without obsolete server libraries

#### Scenario: Verify lifecycle and contention

- **WHEN** clients reconnect, consume slowly, cancel reads, or race listener stop with host completion during recording
- **THEN** the suite verifies bounded work, monotonic state, exactly-once terminal results where connected, and uninterrupted permitted persistence

### Requirement: Shared server physical interoperability coverage

Affected OS/BLE behavior SHALL be checked through the selected production service on relevant platforms and routes, preserving recording, reconnect and frontend/lifecycle recovery. Missing required hardware SHALL be recorded without replacing it with software evidence; independent software checks still proceed.

#### Scenario: Run the physical checkpoint

- **WHEN** each physical host connects through the fixture and its frontend is recreated during recording
- **THEN** the canonical runner observes continued durable progress and current authenticated state through the shared server

#### Scenario: Hardware is unavailable

- **WHEN** a required device or fixture is unavailable
- **THEN** software verification completes independently and the missing physical result is recorded without claiming completion

### Requirement: Shared service regression coverage

Direct-host and affected native conformance SHALL use the production service constructor. Assertions and protocol simulation remain in conformance; target adapters contain only launch and OS integration. Storage/ABI/lifecycle fault tests and ownership audits SHALL preserve shared policy without native fallback implementations.

#### Scenario: Direct host exercises product behavior

- **WHEN** canonical product, archive, recording and reconnect scenarios run through the direct Rust test host
- **THEN** they use the same selected service as native production without test-host product policy, SQL or archive implementations

#### Scenario: Fault and saturation coverage

- **WHEN** storage faults occur between session commit and catalog publication, or cancellation races with saturated queues and worker shutdown
- **THEN** the suite verifies durable recovery, bounded resources, monotonic publication and no duplicate accepted mutation

### Requirement: Startup presentation regression coverage

Affected browser/native checks SHALL cover themed startup, real failure visibility, foreground reconstruction, renderer recovery and repeated recreation. Physical presentation remains distinct from software evidence and follows the impact-based verification policy.

#### Scenario: Early theme behavior

- **WHEN** focused startup checks execute the real frontend bootstrap before application modules load
- **THEN** they verify system light/dark, explicit hints, missing/invalid/inaccessible storage, and subsequent authoritative reconciliation using rendered canvas behavior

#### Scenario: Platform visual matrix

- **WHEN** an affected startup handoff is assessed
- **THEN** recordings cover representative repeated cold launches in light/dark, including saved preferences opposite to system appearance, on the relevant platform/OS/WebView combination
- **AND** evidence distinguishes actual visible handoff from native build or software results

#### Scenario: Lifecycle and failure coverage

- **WHEN** affected startup regression checks run
- **THEN** they exercise foreground/background resume, theme changes, Activity recreation and reproducible failure/recovery, preserving service/session continuity and bounded presentation resources

#### Scenario: Visual result is assessed

- **WHEN** a reviewer inspects a launch recording from before launch through first usable content
- **THEN** app-controlled background regions contain no unintended intermediate default color or transient loading/error screen
- **AND** intentional theme reconciliation and OS-owned animation/prompt regions are identified separately
- **AND** before/after timing observations show no added fixed startup dwell

#### Scenario: Evidence is incomplete

- **WHEN** a runtime or visual recording required by the approved matrix, or the required gate result, is missing or failing
- **THEN** the corresponding acceptance item stays incomplete with its limitation recorded
- **AND** a build result, final screenshot or unrelated successful test does not substitute for that evidence

#### Scenario: Device scope is bounded explicitly

- **WHEN** a change's impact or explicit user direction bounds device coverage
- **THEN** evidence identifies exercised and excluded environments without changing supported OS ranges or claiming excluded checks passed

#### Scenario: Presentation checks do not require a BLE fixture

- **WHEN** only startup presentation changes and existing software lifecycle/protocol behavior remains unchanged
- **THEN** startup acceptance does not require a new physical BLE checkpoint
- **AND** required phone startup observations are distinguished from the affected software checks

### Requirement: Dynamic battery estimation conformance

Canonical conformance SHALL cover temporal battery estimation and the preset/optional-capacity contract across the shared server, Apple and Android host integrations, and frontend mock. Deterministic telemetry sequences SHALL exercise known charge consumption, load/recovery steps, regeneration, charging, settled correction, missing current/voltage, profile changes, duplicate samples, persistence rejection, gaps, reconnect, atomic preset selection, generic-profile mappings, and capacity overrides. Oracles SHALL use independently calculated charge and observable temporal bounds rather than reproduce the estimator implementation. Recording/export assertions SHALL establish unchanged raw telemetry semantics. Scenario orchestration SHALL remain in conformance rather than device firmware.

#### Scenario: Dynamic load sequence

- **WHEN** a fixture applies acceleration, sustained load, release, regeneration, and stable rest at known timestamps
- **THEN** assertions verify charge accounting, absence of instantaneous percentage jumps, rest qualification, and gradual correction

#### Scenario: Failure and lifecycle sequence

- **WHEN** telemetry is duplicated, reordered, rejected, interrupted, or resumed, or the WebView is recreated
- **THEN** charge is counted only once for accepted observed intervals and provisional/method state matches lifecycle rules

#### Scenario: Host and mock parity

- **WHEN** equivalent configuration and accepted telemetry are presented through supported host integrations and the mock
- **THEN** capacity validation, method/provisional fields, percentage bounds, and lifecycle outcomes agree within documented numeric and timing tolerances

#### Scenario: Evidence limits

- **WHEN** validation uses synthetic fixtures without measured ride traces or a reference charge measurement
- **THEN** results describe verified stability and accounting rather than claim measured absolute SOC accuracy

#### Scenario: Preset contract and host parity

- **WHEN** known-curve and generic-curve presets are selected through the supported hosts and mock
- **THEN** catalog fields, complete selection defaults, identity, nominal capacity, invalid mixed requests, and conflict/failure behavior agree
- **AND** persistence and archive round trips preserve manual overrides without reapplying defaults

#### Scenario: Preset charge and configuration lifecycle

- **WHEN** a preset supplies capacity and the fixture observes known signed battery current
- **THEN** charge accounting uses full-pack Ah even with a generic curve
- **AND** count edits clear stale capacity unless replaced, effective configuration changes reseed once, and preset-name-only edits or UI mode switches do not reset temporal state

### Requirement: Refloat 1.3.1-beta2 interoperability evidence

Canonical conformance SHALL exercise exact beta2 admission, the existing selectable telemetry and control profiles, audited configuration identity, same-version Tunes, typed IMU-timeout state, and durable recovered history on direct and CAN routes. It SHALL preserve older-release behavior and supported baseline data. A targeted physical checkpoint SHALL confirm the core capabilities on actual beta2 firmware.

#### Scenario: Source-derived beta2 fixture

- **WHEN** the conformance fixture reports exact beta2 INFO, command-33 responses, command-35 mixed alerts, and its 172-field schema
- **THEN** both hosts reach equivalent ready state, publish live telemetry, verify configuration read/write and same-version Tune apply, and retain timeout activation and clearing without firmware-fault pollution

#### Scenario: Adjacent identities and malformed alerts

- **WHEN** a fixture reports beta1, beta3, stable 1.3.1, malformed suffixes, or invalid beta2 command-35 records
- **THEN** admission or the malformed response fails at its boundary without write traffic, partial history, or stale-generation state

#### Scenario: Saved and guest histories

- **WHEN** identical source-derived timeout transitions occur in saved and guest tracking
- **THEN** saved sessions page and export committed events, guest history remains bounded and ephemeral, and existing supported baseline sessions and archives still load

#### Scenario: Targeted physical checkpoint

- **WHEN** an exact beta2 board is available after software fixture development
- **THEN** a relevant native host validates discovery, advancing live and saved telemetry, schema/current/default reads, a safe complete configuration write/readback/restore, eligible same-version Tune application, and reconnect with original settings restored

#### Scenario: Physical coverage limit

- **WHEN** real IMU failure or an unaffected host or route cannot be exercised safely
- **THEN** the checkpoint reports the limit separately from simulated timeout coverage and does not claim unrun physical validation

### Requirement: Shared component and coarse ABI regression coverage

Focused tests SHALL exercise the retained protocol, coordination and SQLite guarantees in `native-core-migration` using immutable source-derived and adversarial vectors. Host-boundary tests SHALL exercise only the selected coarse C/JNI bindings; component tests must not recreate retired protocol bridges.

#### Scenario: Exercise wire and state-machine vectors

- **WHEN** framing, identity, configuration, BMS, controls, telemetry, compatibility, discovery, scheduling or mutation behavior changes
- **THEN** affected vectors SHALL check exact bytes/units, valid and invalid bounds, truncation/remainders, routing, generations, timeouts, cancellation, deterministic effects and private-data redaction

#### Scenario: Exercise persistence and history vectors

- **WHEN** writer, reader, reducer or pruner behavior changes
- **THEN** canonical database vectors SHALL check atomic markers, reopen/recovery, ordered paging, null/gap and extrema selection, stale-plan rejection, protected evidence, location neighborhoods, interruption, rollback and resource cleanup

#### Scenario: Exercise retained native boundaries

- **WHEN** the C/JNI envelope or host binding changes
- **THEN** tests SHALL check valid/hostile handles, tags, lengths, null/alignment/overlap/capacity, panic containment, complete result publication, lifetime races, callback-free ownership and exact retained symbols/architectures

#### Scenario: Preserve archived migration evidence

- **WHEN** a component's current regression cases are selected
- **THEN** original extraction/cutover reports remain historical evidence, while checks run against today's production shared owner under the current impact policy
