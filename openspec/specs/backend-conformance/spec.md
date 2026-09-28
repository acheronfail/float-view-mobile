# Backend Conformance Specification

## Purpose

Define the single canonical software and physical acceptance suite required of every supported backend.

## Requirements

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

The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Hardware scenarios SHALL exercise the production Bluetooth, protocol, application-service, and storage paths selected by each platform host. Component parity checks MAY exercise a candidate Rust implementation in the fast software gate but MUST NOT present that component as a supported backend or production path before host integration.

#### Scenario: Fast software gate

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

Software conformance SHALL cover the Cartesian product of direct and Express topology, audited VESC 6.02/6.05/6.06/7.00, and representative Refloat 1.0.0/1.1.0/1.2.3. It SHALL additionally cover the complete VESC identity baseline, source-applicable CRC suffix, audited nonzero firmware test-version read-only mode, rejected writes, malformed and unknown identities, and configured, absent, invalid, duplicate, zero, one, and multiple CAN candidates without requiring a separate simulator implementation per combination.

#### Scenario: Supported version matrix

- **WHEN** the software compatibility matrix runs
- **THEN** all twenty-four topology/controller/package combinations establish the expected route and protocol family, report legacy source package 1.0.0 as wire version `1.0`, and produce route-correct telemetry

#### Scenario: VESC identity profile fixtures

- **WHEN** conformance supplies complete VESC 6.02 identity without a CRC and complete 6.05, 6.06, and 7.00 identities with their bounded source-defined CRC suffix
- **THEN** both native backends derive the same stable version, hardware type, names, and private UUID and select the same `vescCoreReadOnly` profile

#### Scenario: Audited test-version access

- **WHEN** conformance supplies a known 6.02/6.05/6.06/7.00 identity with a nonzero test-version marker
- **THEN** both backends reach ready after route and Refloat validation, permit read requests, and reject writes consistently

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

Software conformance SHALL exercise complete detailed telemetry for every audited stable Refloat release in 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 through direct and CAN routes and across audited VESC 6.02, 6.05, 6.06, and 7.00. It SHALL verify the complete VESC identity baseline, audited nonzero test-version read-only mode, source-applicable bounded hardware-CRC suffix, exact command 1 fixture compatibility, command 10 detail levels and metres/second conversion, command 31/32 bytes and native kilometres/hour values, route wrapping, required-field negotiation, version-applicable active state and tails, normalization, nullability, unknown bounded extensions, and absence of command 35, data-recorder, or unsupported write requests from either backend.

#### Scenario: Audited route and package combinations

- **WHEN** the detailed telemetry matrix runs
- **THEN** each audited stable VESC/controller-package pair through direct and CAN routing produces its exact protocol profiles and equivalent normalized semantic values where the source carries them

#### Scenario: Stable VESC identity families

- **WHEN** the matrix supplies 6.02 without a CRC suffix and 6.05, 6.06, or 7.00 with the source-defined bounded suffix
- **THEN** Apple and Android select the same `vescCoreReadOnly` profile, ignore CRC value for compatibility, and proceed to route and Refloat capability validation

#### Scenario: Audited test-version read-only mode

- **WHEN** conformance supplies a known 6.02/6.05/6.06/7.00 identity with a nonzero test-version marker
- **THEN** Apple and Android complete read-only Refloat negotiation, allow read requests, reject writes, and expose read-only access beside the VESC and Refloat versions

#### Scenario: Controller compatibility rejection

- **WHEN** conformance supplies VESC 7.01 with a nonzero test-version marker, another unknown release, or a malformed identity baseline
- **THEN** Apple and Android reject the identity with `unsupported_controller`, reject malformed identity within the bounded identification deadline, and never begin Refloat negotiation or durable session creation

#### Scenario: Compatibility rejection

- **WHEN** conformance supplies an unknown version, prerelease version, or dynamic layout missing a required semantic ID
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

Conformance SHALL own immutable platform-neutral vectors containing ordered raw samples, stable and extension fields, null and time-gap continuity cases, ride phases, session-captured persistence intervals, pruning resolution, published revisions/boundaries, bounded requests, exact expected M4 segments, and keyset pages. The frontend mock, Swift reducer/reader, Kotlin reducer/reader, and canonical suite SHALL consume those vectors. Vectors SHALL cover checked bucket boundaries, extrema order and deduplication, duplicate elapsed values, sequence ties, nulls at and across bucket edges, alternating gaps, configured and derived time gaps, snapshot/revision exclusion, late fields, dense sample windows, event pages, and invalid cursor replay.

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

Conformance SHALL apply immutable pruning vectors to the mock, Swift, Kotlin, and decoded black-box targets. Vectors SHALL define elapsed-zero bucket alignment, first/last rows, first non-null field sources, per-field minimum/maximum roles, greedy maximum-coverage selection, stable field and extension ordering, nulls, multi-role rows, phase-boundary pairs, event sources, earliest-tied summary sources, the eight ordinary-row cap, aligned coarser re-pruning, recorded/stored counts, and expected history revisions. Canonical scenarios SHALL cover preview, one ride, all rides with a captured upper session key, rides completed after batch start, ineligible skips, cancellation before commit, cancellation between rides, crash recovery, stale snapshots/cursors, exports, bounded selector/temp-file resources, WAL checkpoint, and interrupted incremental compaction.

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
- **THEN** it uses immutable payload fixtures derived from pinned `vesc_bms_fw` and `vesc_express` encoders and validates Swift, Kotlin, and simulator decoding against the same bytes

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

Canonical conformance SHALL cover every Refloat configuration and Tune contract method, state-summary field, native codec, safety decision, scheduler interaction, persistence rule, and frontend recovery behavior against mock, Apple, and Android. Immutable source-derived fixtures SHALL include exact compressed schemas, normalized metadata, full-schema fingerprints, wire signatures, current/default payloads, and valid writes for every audited stable Refloat 1.0.0–1.2.3 release. Software scenarios SHALL cover legacy 1.0 patch resolution and signature collision, direct and CAN routes, cache hit/miss/corruption, paging, bounds, malformed XML and values, stale revisions/generations, semantic-control contention, write acknowledgement/readback, reconnect, read-only firmware, exact-version isolation, package/configuration convergence, runtime-versus-persistent lighting, and flywheel-mode rejection of every mutation except eligible flywheel disable. They SHALL also prove that command 93 is withheld during flywheel and a fresh normal-mode read is required after exit.

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

### Requirement: Apple Rust frame codec cutover evidence

Apple framing ownership SHALL NOT be considered complete until focused Rust/C/Swift checks, clean Apple packaging checks, `just verify`, canonical macOS execution, iOS simulator build/launch, and the representative physical iOS BLE checkpoint all exercise Rust CRC, encode, exact decode, and stream boundary scanning without a Swift framing fallback. Evidence SHALL cover immutable short/long vectors, the 4,096-byte payload limit, the 8,192-byte aggregate stream limit, malformed length/header/footer/CRC/trailing bytes, caller-region failures, one-byte and fixture fragmentation, split long headers, leading noise, multiple frames, incomplete suffix retention, reported consumption lengths, malformed-frame recovery, and aggregate overflow recovery.

The physical checkpoint SHALL use production CoreBluetooth and the shared Apple protocol path with fixture-supplied fragmented notifications and SHALL include the canonical iOS ride/background-recovery behavior. After that checkpoint succeeds, repository verification SHALL require Swift CRC, frame encode/exact decode, and stream header/length/noise parsing algorithms to be absent and SHALL rerun the complete software gate. This cutover SHALL add no Android target adapter or alter Android's required results.

#### Scenario: Exercise the compiled ABI

- **WHEN** focused verification runs the C harness and Swift codec/stream tests
- **THEN** the hand-written header, ABI version, exact four-symbol allowlist, fixed statuses, pointer and primitive-output bounds, panic containment, scanner consumption lengths, immutable frame bytes, and established Swift error mappings agree with the safe Rust component

#### Scenario: Verify Apple software production selection

- **WHEN** `just verify` and clean macOS and iOS simulator checks run after stream adapter cutover
- **THEN** the shared Apple codec and stream tests pass through linked Rust scanning and exact decoding and no Swift production framing algorithm or fallback is present

#### Scenario: Verify physical iOS stream framing

- **WHEN** focused physical iOS conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production CoreBluetooth notification bytes are bounded in Swift, interpreted and exactly decoded by Rust, and the same session plus additional samples survive foreground recovery

#### Scenario: Delete superseded Apple stream code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes Swift stream header/length/noise parsing, retains only the thin caller-owned byte-storage adapter, and passes the complete software gate again

#### Scenario: Preserve Android conformance

- **WHEN** the Apple stream-boundary cutover is verified
- **THEN** Android continues to pass its existing canonical and physical requirements through the Kotlin framing implementation without JNI, NDK, or Rust packaging

### Requirement: Android Rust frame codec cutover evidence

Complete Android frame-codec ownership SHALL NOT be considered transferred until focused Rust/JNI/Kotlin checks, clean host-JVM and Android Debug/Release packaging checks, `just verify`, canonical Android execution, and a representative physical Android BLE checkpoint all exercise Rust CRC, encode, exact decode, and stream boundary scanning without a Kotlin framing fallback. Evidence SHALL cover immutable short/long vectors, empty and 4,096-byte payloads, malformed length/header/footer/CRC/trailing bytes, null/alias/shape/capacity/status failures, one-byte and fixture fragmentation, split short and long headers, leading noise, multiple frames, retained incomplete suffixes, malformed-frame recovery, oversized declarations, exact consumption lengths, and aggregate stream failure through Kotlin-owned bounded storage.

The physical checkpoint SHALL use production Android Bluetooth and the normal Android protocol path with fixture-supplied fragmented notifications and SHALL include the canonical Android ride/background-recovery behavior. Location authorization, background capability, and monotonic persistence boundaries SHALL remain hard assertions, but absence of a location sample during the fixed automated dwell SHALL be reported as deferred manual end-to-end coverage rather than failing this framing checkpoint. After the BLE/session checkpoint succeeds, repository verification SHALL require Kotlin CRC, frame encoding, exact decoding, and stream boundary algorithms to be absent and SHALL rerun the complete software gate. This cutover SHALL not alter the frontend contract, Apple target adapters, or established Android stream results.

#### Scenario: Exercise the compiled JNI boundary

- **WHEN** focused verification runs bridge tests and the linked Android host-JVM codec and stream tests
- **THEN** ABI version, exact four-symbol allowlist, fixed statuses, primitive-array bounds, panic containment, immutable frame bytes, exact scanner consumption results, and established Kotlin error mappings agree with the safe Rust component

#### Scenario: Verify Android software production selection

- **WHEN** `just verify`, clean Gradle unit tests, Debug/Release assembly, and canonical Android conformance run after adapter cutover
- **THEN** Android codec and stream tests pass through linked Rust encode, exact decode, and stream scanning and no Kotlin production framing algorithm or fallback is present

#### Scenario: Verify physical Android framing

- **WHEN** focused physical Android conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production Android Bluetooth requests are Rust-encoded, inbound boundaries are Rust-selected, complete frames are Rust-decoded, and the same session plus additional samples survive foreground recovery

#### Scenario: Delete superseded Android stream code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes Kotlin header, length, noise, incomplete-suffix, frame-extent, recovery-decision, and aggregate-overflow parsing while retaining only the thin codec and bounded-storage adapter, then passes the complete software gate again

#### Scenario: Preserve Apple conformance

- **WHEN** the Android stream-boundary cutover is verified
- **THEN** iOS and macOS continue to pass their existing software, packaging, canonical, and physical requirements through the unchanged Apple Rust framing path

### Requirement: Rust firmware identity candidate evidence

The Rust firmware identity candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived parity, adversarial structural tests, the complete `just verify` software gate, and focused physical identity regression on Android, iOS, and macOS. Component evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, nonzero test markers, bounded names, private UUIDs, configuration counts, absent and present CRC tails, wrong commands, truncation at every field class, terminator bounds, invalid stock-BMS tails, incomplete extended tails, invalid CRC-tail lengths, unexpected trailing bytes, and redacted failures.

Physical evidence in this candidate-only change SHALL use the production Bluetooth and native firmware identity paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, fragmented framing, direct/CAN identity, or cleanup. Later host-cutover changes SHALL rerun focused physical identity through the selected Rust decoder before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, session, or identity failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode the committed motor, gateway, stock-BMS, and custom-module identity evidence
- **THEN** every accepted field and rejected structural class agrees with the current native behavior without adding a Rust-only compatibility rule

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while bridge symbol allowlists and production call sites remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** focused physical identity conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native parser completes real-BLE identity, route, and cleanup behavior without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical execution, and superseded-code deletion independently

### Requirement: Apple Rust firmware identity cutover evidence

Apple firmware identity ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical identity runs on iOS and macOS exercise the Rust decoder without a Swift fallback. Evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, version/name/private UUID/test/type/configuration/CRC fields, replacement UTF-8, wrong commands, structural truncation and tail failures, ABI mismatch, invalid pointer/capacity/overlap/result regions, output bounds, panic containment, and private-data-safe diagnostics.

Android SHALL pass the same focused physical identity regression through its unchanged Kotlin decoder and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, identity, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise source-derived and malformed identities through the compiled Apple ABI
- **THEN** fixed version/status/record behavior, field mapping, established Swift errors, bounds, redaction, and panic containment agree with the safe decoder

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus `just verify` run after cutover
- **THEN** shared Apple identity tests execute Rust through the linked two-symbol bridge and no Swift structural parser, fallback, generated binding, or extra symbol is present

#### Scenario: Verify Apple physical identity selection

- **WHEN** focused physical identity conformance runs on the connected iOS device and macOS host against fragmented ESP32 fixture responses
- **THEN** both shared Apple hosts reach ready, expose the expected direct/CAN identity and route, and clean up through the production Rust identity decoder

#### Scenario: Preserve Android regression

- **WHEN** the same focused physical identity scenario runs on the connected Android device
- **THEN** Android passes through its unchanged Kotlin identity decoder without linking or calling the Apple bridge

#### Scenario: Delete superseded Swift parser

- **WHEN** focused, packaging, canonical, and physical checks pass
- **THEN** the Swift common-prefix, terminated-name, hardware-tail, and CRC-tail implementation is absent and the complete software gate remains green

### Requirement: Android Rust firmware identity cutover evidence

Android firmware identity ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical identity runs on Android, iOS, and macOS exercise Android's Rust decoder without a Kotlin fallback and preserve Apple's established Rust path. Evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, version/name/private UUID/test/type/configuration/CRC fields, replacement UTF-8, wrong commands, structural truncation and tail failures, ABI mismatch, null/alias/array-shape/output-bound failures, pending exceptions, panic containment, invalid result records, and private-data-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, identity, route, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise source-derived and malformed identities through the compiled JNI ABI
- **THEN** fixed version/status/record behavior, field mapping, established Kotlin errors, bounds, redaction, exception handling, and panic containment agree with the safe decoder

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android identity tests execute Rust through the linked two-symbol bridge and no Kotlin structural parser, fallback, generated binding, committed binary, or extra identity symbol is present

#### Scenario: Verify Android physical identity selection

- **WHEN** focused physical identity conformance runs on the connected Android device against fragmented ESP32 fixture responses
- **THEN** Android reaches ready, exposes the expected direct or CAN identity and route, and cleans up through the production Rust identity decoder

#### Scenario: Preserve Apple physical identity regression

- **WHEN** the same focused physical identity scenario runs on the connected iOS device and macOS host
- **THEN** both Apple hosts pass through their unchanged Rust identity decoder and Apple bridge without linking or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** the Kotlin common-prefix, terminated-name, hardware-tail, and CRC-tail implementation is absent and the complete software gate remains green

### Requirement: Cross-platform flywheel confirmation regression

Canonical verification SHALL exercise equivalent Apple and Android application-service lifecycles for flywheel enable, repeated flywheel telemetry during pending disable, normal-mode disable confirmation, prohibited competing mutation cancellation, and absent/stale failure callbacks. The complete software gate and focused direct/CAN physical Refloat control sequences on Android, iOS, and macOS SHALL pass without confirmation timeout, duplicate completion, mutation safety relaxation, or motor movement.

#### Scenario: Native service parity

- **WHEN** focused native tests inject the same command transmission and authoritative telemetry sequence
- **THEN** Apple and Android publish equivalent pending and confirmed states and return the same successful enable/disable results

#### Scenario: Complete software regression

- **WHEN** `just verify` runs
- **THEN** native, conformance, frontend, Rust, packaging, and ownership gates pass without a new warning or fallback path

#### Scenario: All-host physical control regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on Android, iOS, and macOS
- **THEN** each host completes enable and disable confirmation without fixture rejection, timeout, or motor movement

### Requirement: Lighting query and mutation distinction

Canonical conformance SHALL prove that the shared Refloat simulator distinguishes zero-mask lighting status queries from nonzero-mask lighting mutations across representative legacy and stable releases, direct and CAN routes, and normal and flywheel modes. Evidence SHALL include exact query responses, unchanged disposable state, retained wrong-route and malformed rejection, retained test-version read-only behavior, and retained flywheel rejection for every actual mutation. The focused physical gate SHALL exercise direct and CAN writable control sequences on Android, iOS, and macOS without timing waivers.

#### Scenario: Deterministic query matrix

- **WHEN** software conformance sends source-correct zero-mask lighting requests through representative supported versions and routes
- **THEN** each request receives the current bounded lighting response without state mutation or a safety diagnostic

#### Scenario: Flywheel race regression

- **WHEN** a scheduled lighting refresh occurs between flywheel enable and disable in the canonical control sequence
- **THEN** the query succeeds, flywheel disable remains reachable, and every nonzero-mask competing mutation remains rejected

#### Scenario: All-host physical regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on Android, iOS, and macOS
- **THEN** every host completes its route-correct sequence without fixture rejection or motor movement

### Requirement: Rust custom-configuration candidate evidence

The Rust VESC custom-configuration candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, every-release fixture parity, adversarial structural tests, the complete `just verify` software gate, and focused physical regression on Android, iOS, and macOS. Component evidence SHALL cover all Refloat 1.0.0–1.2.3 schema/current/default/write/readback fixture bytes, every command 92–95 request and response variant, index and signed-integer boundaries, chunk bounds and arithmetic, value minimum/bounds, exact acknowledgements, recognized and unrecognized wrong commands, redacted failures, and unchanged native bridge symbols and production call sites.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, Refloat, and native custom-configuration paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness and control, or cleanup. Later host-cutover changes SHALL rerun focused configuration operations through the selected Rust codec before native codec deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, session, routing, control, or cleanup failures.

#### Scenario: Verify every-release component parity

- **WHEN** focused Rust tests encode and decode envelopes around every committed Refloat schema, current, default, accepted-write, and readback fixture
- **THEN** every request byte, response variant, extent, index, and opaque byte range agrees with selected native behavior without adding schema or value semantics

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every fixed-field truncation, integer boundary, chunk inconsistency, values boundary, acknowledgement tail, and wrong-command class
- **THEN** the candidate fails closed with stable bounded errors and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbol allowlists and production calls remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** focused identity and direct/CAN Refloat physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes BLE identity, direct/CAN readiness and safe control, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical configuration execution, and superseded-code deletion independently

### Requirement: Apple Rust custom-configuration cutover evidence

Apple custom-configuration envelope ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical direct/CAN configuration runs on iOS and macOS exercise the Rust codec without a Swift fallback. Evidence SHALL cover all Refloat 1.0.0–1.2.3 schema/current/default/write/readback fixtures; commands 92–95; index and signed-integer boundaries; chunk arithmetic and bounds; values minimum and bounds; exact acknowledgements; recognized and unrecognized wrong commands; ABI mismatch; invalid pointer, capacity, overlap, operation, record, offset, and length regions; output bounds; panic containment; and payload-safe diagnostics.

Android SHALL pass the same focused physical configuration regression through its unchanged Kotlin codec and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, configuration, routing, control, no-motor-movement, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise every-release and malformed configuration envelopes through the compiled Apple ABI
- **THEN** fixed version, operation, status, result-record, field, opaque-range, established Swift-error, bound, redaction, and panic behavior agrees with the safe component

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus `just verify` run after cutover
- **THEN** shared Apple configuration tests execute Rust through the linked three-symbol bridge and no Swift envelope parser, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Apple configuration selection

- **WHEN** focused direct and CAN physical control conformance runs on the connected iOS device and macOS host against the ESP32 fixture
- **THEN** both shared Apple hosts open schema, read current and default values, write the audited disposable configuration mutation, verify readback, complete established no-motor-movement controls, and clean up through the production Rust envelope codec

#### Scenario: Preserve Android configuration regression

- **WHEN** the same direct and CAN physical control scenario runs on the connected Android device
- **THEN** Android completes the same configuration and control sequence through its unchanged Kotlin codec without linking or calling the Apple bridge

#### Scenario: Delete superseded Swift codec

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Swift command construction, signed-integer encoding, chunk, values-envelope, and acknowledgement parsing are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, session, configuration, route, control, no-motor-movement, and cleanup assertion remains mandatory

### Requirement: Apple Rust controller-data cutover evidence

Apple command-4 realtime and command-14 current-limit structural ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical controller runs on iOS and macOS exercise the Rust decoder without a Swift fallback. Evidence SHALL cover committed normal and faulted realtime payloads; exact consumed prefixes and permitted tails; every selected scalar, scale, signed value, ERPM, raw fault, and classification; all four current limits; every supported serialization signature and series-cell offset; absent, zero, and positive optional cell counts; auto-float sign, zero, range, and invalid forms; wrong commands; every consumed truncation; zero and non-finite limits; ABI mismatch; invalid operation, pointer, capacity, overlap, record kind, flag, extent, reserved, numeric, fault, and optional-cell values; panic containment; and payload-safe diagnostics.

Android SHALL pass the same focused controller regression through its unchanged Kotlin decoder and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, controller-data, route, framing, telemetry, state, session, control, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise fixture-derived, boundary, and malformed controller data through the compiled Apple ABI
- **THEN** fixed version, operation, status, tagged-record, field, established Swift-error, bound, redaction, and panic behavior agrees with the safe component

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus canonical checks and `just verify` run after cutover
- **THEN** shared Apple controller-data tests execute Rust through the linked two-symbol bridge and no Swift byte reader, fixed-point, auto-float, signature/offset, fault-classification, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Apple controller-data selection

- **WHEN** focused identity and direct/CAN control physical conformance runs on the connected iOS device and macOS host, with mobile background coverage on iOS, against the ESP32 fixture
- **THEN** both shared Apple hosts complete real-BLE current-limit and realtime decoding, routing, telemetry, state, session/control behavior, and cleanup through the production Rust decoder

#### Scenario: Preserve Android controller-data regression

- **WHEN** the same applicable focused controller scenarios run on the connected Android device
- **THEN** Android completes them through its unchanged Kotlin decoder without linking, packaging, or calling the Apple bridge

#### Scenario: Delete superseded Swift parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Swift command-4/command-14 byte reading, scaling, auto-float, signature/offset lookup, optional-cell, and fault-classification algorithms are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, controller-data, route, framing, telemetry, state, session, control, and cleanup assertion remains mandatory

### Requirement: Rust VESC controller-data candidate evidence

The Rust VESC controller-data candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, upstream-derived fixture parity, adversarial structural and numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover normal and faulted realtime payloads; exact consumed prefixes and permitted tails; every selected signed fixed-point field; signed ERPM; all raw fault classifications; every supported motor-configuration signature and series-cell offset; unknown, absent, zero, and positive cell counts; VESC auto-float zero, sign, range, and invalid forms; every consumed-prefix truncation; wrong commands; zero and non-finite current limits; bounded storage; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native controller-data decoder, scheduling, telemetry, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, current-limit publication, telemetry, sessions, controls, background continuity, or cleanup. Later host-cutover changes SHALL rerun focused controller reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, controller-data, routing, state, session, control, or cleanup failures.

#### Scenario: Verify upstream-derived component parity

- **WHEN** focused Rust tests decode the committed realtime fixtures and source-derived current-limit vectors
- **THEN** every selected scalar, scale, signed value, fault, limit, signature, offset, and optional cell count agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every consumed truncation, wrong command, numeric boundary, invalid auto-float, zero limit, unknown signature, and cell-offset boundary
- **THEN** the candidate either returns the exact bounded value or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical controller regression

- **WHEN** focused identity, direct/CAN control, and mobile background physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE controller reads, routing, telemetry, state, session/control behavior, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust controller-data execution, and superseded-code deletion independently

### Requirement: Android Rust Refloat control codec cutover evidence

Android Refloat control ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical direct/CAN control runs on Android exercise the Rust codec without a Kotlin fallback while iOS and macOS preserve their established Rust path. Evidence SHALL cover every committed package, flywheel, lighting, query, and fatal-clear request; both lighting profiles; all current/target/enabled states; all defined response states; exact capacities; unavailable fatal clear; every invalid operation/profile/target/boolean/current-state/unused-parameter and response extent/prefix/package/command/profile/flag class; ABI mismatch; null, alias, array-shape, capacity, result-length, and result-value failures; pending exceptions; panic containment; and payload-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, routing, framing, control, confirmation, state, background/session, no-motor-movement, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise source-derived, exhaustive-state, and malformed controls through the compiled JNI ABI
- **THEN** fixed version, operation, status, payload, primitive-result, established Kotlin-error, bound, redaction, exception, and panic behavior agrees with the safe component

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android control tests execute Rust through the linked three-symbol bridge and no Kotlin control-wire algorithm, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Android control selection

- **WHEN** focused direct and CAN physical Refloat control conformance runs on the connected Android device against the ESP32 fixture
- **THEN** Android completes exact safe package, flywheel, lighting, query, fatal-clear, routing, framing, confirmation, state, and cleanup behavior through the production Rust codec without motor movement

#### Scenario: Preserve Apple control regression

- **WHEN** the same focused direct and CAN physical Refloat control scenarios run on the connected iOS device and macOS host
- **THEN** both Apple hosts complete them through their unchanged safe Rust codec and Apple bridge without linking, packaging, or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin codec

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Kotlin package, flywheel, lighting, query, fatal-clear, and lighting-response wire algorithms are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, routing, framing, control, confirmation, state, background/session, no-motor-movement, and cleanup assertion remains mandatory

### Requirement: Apple Rust Refloat control codec cutover evidence

Apple Refloat control ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical direct/CAN control runs on iOS and macOS exercise the Rust codec without a Swift fallback. Evidence SHALL cover every committed package, flywheel, lighting, query, and fatal-clear request; both lighting profiles; all current/target/enabled states; all defined response states; exact capacities; unavailable fatal clear; every invalid operation/profile/target/boolean/current-state/unused-parameter and response extent/prefix/package/command/profile/flag class; ABI mismatch; null, overlap, alignment, capacity, result-length, and result-value failures; panic containment; and payload-safe diagnostics.

Android SHALL pass the same focused physical direct/CAN control regression through its unchanged Kotlin codec and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, routing, control, confirmation, state, background/session, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise source-derived, exhaustive-state, and malformed controls through the compiled Apple ABI
- **THEN** fixed version, operation, status, payload, primitive-result, established Swift-error, bound, redaction, and panic behavior agrees with the safe component

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus canonical checks and `just verify` run after cutover
- **THEN** shared Apple control tests execute Rust through the linked three-symbol bridge and no Swift control-wire algorithm, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Apple control selection

- **WHEN** focused direct and CAN physical Refloat control conformance runs on the connected iOS device and macOS host against the ESP32 fixture
- **THEN** both shared Apple hosts complete exact safe package, flywheel, lighting, query, fatal-clear, routing, confirmation, state, and cleanup behavior through the production Rust codec without motor movement

#### Scenario: Preserve Android control regression

- **WHEN** the same focused direct and CAN physical Refloat control scenarios run on the connected Android device
- **THEN** Android completes them through its unchanged Kotlin codec without linking or calling the Apple bridge

#### Scenario: Delete superseded Swift codec

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Swift package, flywheel, lighting, query, fatal-clear, and lighting-response wire algorithms are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, routing, control, confirmation, state, background/session, no-motor-movement, and cleanup assertion remains mandatory

### Requirement: Rust Refloat control codec candidate evidence

The Rust Refloat control codec candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived fixture parity, exhaustive lighting-state and adversarial response tests, the complete `just verify` software gate, and focused physical Refloat control regression on Android, iOS, and macOS. Component evidence SHALL cover every committed package, flywheel, legacy/stable lighting, and fatal-clear request; both lighting profiles; zero-mask queries; all current, target, and enabled lighting combinations; exact payload capacities; all defined lighting response states; unavailable fatal clear; empty, short, long, wrong-prefix, wrong-package, wrong-command, wrong-profile, and undefined-flag failures; bounded storage; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat codec, mutation policy, confirmation, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress live package, flywheel, lighting, fatal-clear, or cleanup behavior. Later host-cutover changes SHALL rerun focused controls through the selected Rust component before native codec deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, control, routing, state, or cleanup failures.

#### Scenario: Verify source-derived and exhaustive component parity

- **WHEN** focused Rust tests encode the committed controls, exhaust both lighting profiles and state combinations, and decode all defined lighting response states
- **THEN** every command, parameter, mask, flag, payload extent, and returned boolean agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise unavailable fatal clear and every response extent, prefix, package, command, profile, and flag failure class
- **THEN** the candidate fails closed with stable bounded errors, fixed resource use, no partial result, and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical Refloat control regression

- **WHEN** focused Refloat live-control physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE control encoding, routing, framing, response decoding, confirmation, state, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust control execution, and superseded-code deletion independently

### Requirement: Android Rust BMS values cutover evidence

Android command-96 structural ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical BMS runs on Android exercise the Rust decoder without a Kotlin fallback while iOS and macOS preserve their established Rust path. Evidence SHALL cover committed legacy and extended payloads; every scalar, unit, collection, balancing flag, reported CAN ID, optional tail, signed fixed-point and auto-float rule; zero and maximum counts; maximum status; every prefix truncation and malformed command, count, flag, route, numeric, UTF-8, tail, and remainder class; ABI mismatch; null, alias, array-shape, capacity, expected-CAN, result-version, flag, count, offset, extent, boolean, numeric, and optional-tail failures; pending exceptions; output bounds; panic containment; and payload-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, BMS, route, polling, state, session, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise source-derived, exact-boundary, and malformed BMS values through the compiled JNI ABI
- **THEN** fixed version, status, result-record, field, collection, established Kotlin-error, bound, redaction, exception, and panic behavior agrees with the safe component

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android BMS tests execute Rust through the linked two-symbol bridge and no Kotlin BMS structural parser, fallback, generated binding, committed binary, or extra BMS symbol is present

#### Scenario: Verify physical Android BMS selection

- **WHEN** standalone, direct-controller, and Express BMS physical conformance runs on the connected Android device against the ESP32 fixture
- **THEN** Android completes real-BLE identity, direct/CAN command-96 decoding, polling, authoritative state, and cleanup through the production Rust decoder

#### Scenario: Preserve Apple physical BMS regression

- **WHEN** the same three focused BMS scenarios run on the connected iOS device and macOS host
- **THEN** both Apple hosts complete them through their unchanged safe Rust decoder and Apple bridge without linking, packaging, or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Kotlin command-96 fields, collection counts and loops, balancing validation, reported-CAN validation, BMS auto-float sequence, status-tail parsing, and remainder checks are absent while the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that location sample observation is deferred and every permission, capability, monotonic-boundary, BLE, BMS, route, polling, state, session, and cleanup assertion remains mandatory

### Requirement: Apple Rust BMS values cutover evidence

Apple command-96 structural ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical BMS runs on iOS and macOS exercise the Rust decoder without a Swift fallback. Evidence SHALL cover committed legacy and extended payloads; every scalar, unit, collection, balancing flag, reported CAN ID, optional tail, signed fixed-point and auto-float rule; zero and maximum counts; maximum status; every prefix truncation and malformed command, count, flag, route, numeric, UTF-8, tail, and remainder class; ABI mismatch; invalid pointer, capacity, overlap, result-version, flag, count, offset, extent, boolean, numeric, and optional-tail records; output bounds; panic containment; and payload-safe diagnostics.

Android SHALL pass the same standalone, direct-controller, and Express BMS physical regression through its unchanged Kotlin decoder and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, BMS, route, polling, state, session, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise source-derived, exact-boundary, and malformed BMS values through the compiled Apple ABI
- **THEN** fixed version, status, result-record, field, collection, established Swift-error, bound, redaction, and panic behavior agrees with the safe component

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus `just verify` run after cutover
- **THEN** shared Apple BMS tests execute Rust through the linked two-symbol bridge and no Swift BMS structural parser, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Apple BMS selection

- **WHEN** standalone, direct-controller, and Express BMS physical conformance runs on the connected iOS device and macOS host against the ESP32 fixture
- **THEN** both shared Apple hosts complete real-BLE identity, direct/CAN command-96 decoding, polling, authoritative state, and cleanup through the production Rust decoder

#### Scenario: Preserve Android BMS regression

- **WHEN** the same three focused BMS scenarios run on the connected Android device
- **THEN** Android completes them through its unchanged Kotlin decoder without linking, packaging, or calling the Apple bridge

#### Scenario: Delete superseded Swift parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Swift command-96 fields, collection counts and loops, balancing validation, reported-CAN validation, BMS auto-float sequence, status-tail parsing, and remainder checks are absent while the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that location sample observation is deferred and every permission, capability, monotonic-boundary, BLE, BMS, route, polling, state, session, and cleanup assertion remains mandatory

### Requirement: Rust VESC BMS values candidate evidence

The Rust VESC BMS values candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived fixture parity, adversarial structural tests, the complete `just verify` software gate, and focused physical BMS regression on Android, iOS, and macOS. Component evidence SHALL cover committed legacy and extended command-96 payloads; every decoded field and unit; signed fixed-point and auto-float behavior; zero and 50 cell/temperature counts; balancing flags; reported-CAN validation; absent and 40-byte status tails; every prefix truncation; excessive counts; wrong commands; malformed numeric, UTF-8, tail, and remainder classes; bounded storage; and payload-safe failures.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, native BMS decoder, polling, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress standalone direct BMS, direct-controller/CAN-BMS, Express/controller/CAN-BMS, or cleanup behavior. Later host-cutover changes SHALL rerun focused command-96 decoding through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, BMS, routing, polling, state, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode the committed legacy and extended BMS values payloads plus exact minimum and maximum bounded forms
- **THEN** every fixed value, collection item, flag, route field, optional tail, scale, and unit agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every truncation, count, flag, route, numeric, status, tail, remainder, and wrong-command failure class
- **THEN** the candidate fails closed with stable bounded errors, fixed resource use, no partial result, and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical BMS regression

- **WHEN** standalone, direct-controller, and Express BMS physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE identity, direct/CAN values decoding, polling, authoritative state, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical command-96 execution, and superseded-code deletion independently

### Requirement: Android Rust custom-configuration cutover evidence

Android custom-configuration envelope ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical direct/CAN configuration runs on Android exercise the Rust codec without a Kotlin fallback while iOS and macOS preserve their established Rust path. Evidence SHALL cover all Refloat 1.0.0–1.2.3 schema/current/default/write/readback fixtures; commands 92–95; index and signed-integer boundaries; chunk arithmetic and bounds; values minimum and bounds; exact acknowledgements; recognized and unrecognized wrong commands; ABI mismatch; null, alias, array-shape, capacity, operation, result, offset, and length failures; pending exceptions; output bounds; panic containment; and payload-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, configuration, routing, control, no-motor-movement, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise every-release and malformed configuration envelopes through the compiled JNI ABI
- **THEN** fixed version, operation, status, result-record, field, opaque-range, established Kotlin-error, bound, redaction, exception, and panic behavior agrees with the safe component

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android configuration tests execute Rust through the linked three-symbol bridge and no Kotlin envelope parser, fallback, generated binding, committed binary, or extra custom-configuration symbol is present

#### Scenario: Verify physical Android configuration selection

- **WHEN** focused direct and CAN physical control conformance runs on the connected Android device against the ESP32 fixture
- **THEN** Android opens schema, reads current and default values, writes the audited disposable configuration mutation, verifies readback, completes established no-motor-movement controls, and cleans up through the production Rust envelope codec

#### Scenario: Preserve Apple physical configuration regression

- **WHEN** the same focused direct and CAN physical control scenario runs on the connected iOS device and macOS host
- **THEN** both Apple hosts complete the configuration and control sequence through their unchanged safe Rust codec and Apple bridge without linking or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin codec

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Kotlin command construction, signed-integer encoding, chunk, values-envelope, and acknowledgement parsing are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, session, configuration, route, control, no-motor-movement, and cleanup assertion remains mandatory

### Requirement: Android Rust controller-data cutover evidence

Android command-4 realtime and command-14 current-limit structural ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, canonical, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical controller runs on Android exercise the Rust decoder without a Kotlin fallback while iOS and macOS preserve their established Rust path. Evidence SHALL cover committed normal and faulted realtime payloads; exact consumed prefixes and permitted tails; every selected scalar, scale, signed value, ERPM, raw fault, and classification; all four current limits; every supported serialization signature and series-cell offset; absent, zero, and positive optional cell counts; auto-float sign, zero, range, and invalid forms; wrong commands; every consumed truncation; zero and non-finite limits; ABI mismatch; invalid operation, null, alias, array shape, capacity, result kind, flag, extent, reserved, numeric, fault, and optional-cell values; pending exceptions; panic containment; and payload-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, controller-data, route, framing, telemetry, state, session, control, background, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise fixture-derived, boundary, and malformed controller data through the compiled JNI ABI
- **THEN** fixed version, operation, status, tagged-record, field, established Kotlin-error, bound, redaction, exception, and panic behavior agrees with the safe component

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android controller-data tests execute Rust through the linked two-symbol bridge and no Kotlin byte reader, fixed-point, auto-float, signature/offset, fault-classification, fallback, generated binding, committed binary, or extra symbol is present

#### Scenario: Verify physical Android controller-data selection

- **WHEN** focused identity and direct/CAN control physical conformance runs on the connected Android device, with mobile background coverage, against the ESP32 fixture
- **THEN** Android completes real-BLE current-limit and realtime decoding, routing, telemetry, state, session/control behavior, and cleanup through the production Rust decoder

#### Scenario: Preserve Apple controller-data regression

- **WHEN** the same applicable focused controller scenarios run on the connected iOS device and macOS host
- **THEN** both Apple hosts complete them through their unchanged safe Rust decoder and Apple bridge without linking, packaging, or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** Kotlin command-4/command-14 byte reading, scaling, auto-float, signature/offset lookup, optional-cell, and fault-classification algorithms are absent and the complete software gate remains green

#### Scenario: Do not waive non-location failures

- **WHEN** a physical run does not observe a location sample within its bounded dwell
- **THEN** only that sample observation is deferred and every permission, capability, monotonic-boundary, BLE, controller-data, route, framing, telemetry, state, session, control, background, and cleanup assertion remains mandatory

### Requirement: Rust Refloat legacy telemetry candidate evidence

The Rust Refloat legacy telemetry candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source fixture parity, adversarial structural and numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover detail levels 1 through 4; the active-firmware-fault form; every present and absent optional tail; package state, mode, footpad, stop-condition, setpoint-adjustment, alert, and firmware-fault mappings; signed and scaled numeric boundaries; speed conversion; finite distance; sentinel field weakening; exact and inexact lengths; wrong envelopes and commands; deterministic fixed-size output; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat telemetry decoder, scheduling, telemetry, session, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, telemetry, sessions, controls, background continuity, or cleanup. Later host-cutover changes SHALL rerun focused command-10 reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, telemetry, routing, state, session, control, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode every committed command-10 detail-level and fault fixture
- **THEN** every envelope, scalar, scale, enum, flag, nullable tail, fault, and sentinel behavior agrees with pinned Refloat source and the stable normalized telemetry requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every input-length boundary, invalid form, wrong identifier, numeric extreme, non-finite distance, sentinel value, and unknown bounded code
- **THEN** the candidate either returns the exact bounded value or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical telemetry regression

- **WHEN** focused identity, direct/CAN control, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, routing, telemetry, state, session/control behavior, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust command-10 execution, sentinel normalization, and superseded-code deletion independently

### Requirement: Shared Apple FFI support conformance evidence

Shared Apple FFI support SHALL NOT be accepted until component tests prove exact valid, null, zero-length, alignment, overflow, overlap, capacity, publication, and panic behavior; every retained Apple bridge passes its existing C ABI and adversarial fixtures; all supported Apple target architectures package the existing archives and symbols; Android software remains unchanged; and `just verify` succeeds. Physical regression SHALL exercise production BLE and selected bridge artifacts on Android, iOS, and macOS against the ESP32 fixture without bypassing host routing, protocol, state, session, background, or cleanup paths.

#### Scenario: Verify the shared boundary component

- **WHEN** focused Rust tests exercise valid and adversarial regions, input borrowing, byte/scalar publication, and contained panics
- **THEN** every operation is bounded and deterministic, invalid boundaries fail before unsafe access or partial output, and diagnostics contain neither addresses nor payloads

#### Scenario: Verify every Apple bridge ABI

- **WHEN** the six focused Apple C ABI suites and architecture/symbol audits run after migration
- **THEN** every public symbol, status, record, archive, header, dependency boundary, and Swift integration remains compatible while copied primitives are absent

#### Scenario: Verify all-host physical regression

- **WHEN** focused identity, direct/CAN control, BMS, and background scenarios run as applicable on the connected Android device, connected iOS device, and macOS host
- **THEN** production behavior and cleanup pass through the existing selected host paths, proving the Apple refactor caused no observed cross-platform regression

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking

### Requirement: Shared Android JNI support conformance evidence

Repository conformance SHALL prove that centralizing Android JNI boundary mechanics changes no public contract, JNI ABI, packaged artifact, selected protocol behavior, native application behavior, or cross-platform ownership. Evidence SHALL include exhaustive support-unit failures, all six Android bridge host/JNI suites, unchanged Apple ABI suites, dependency and raw-JNI-ownership audits, clean packaging, canonical software conformance, and the required physical fixture checkpoint.

#### Scenario: Exercise shared JNI failures

- **WHEN** support tests supply null environment/function tables, pending and newly raised exceptions, null or aliased references, invalid/excessive lengths, insufficient publication capacity, exact and zero lengths, and contained panics
- **THEN** each operation returns its bounded protocol-neutral failure without retaining VM state, allocating past the caller limit, partially publishing later results, or exposing private diagnostics

#### Scenario: Preserve every host contract

- **WHEN** focused suites invoke every established Android JNI and Apple C symbol with valid and adversarial inputs after migration
- **THEN** statuses, records, lengths, metadata, symbols, libraries, archives, dependency boundaries, and native mappings remain exact

#### Scenario: Verify all-host physical regression

- **WHEN** identity, direct/CAN control, BMS, and mobile background scenarios run as applicable on the connected Android device, connected iOS device, and macOS host against the ESP32 fixture
- **THEN** all non-location discovery, BLE, routing, protocol, telemetry, control, state, session, background, and cleanup assertions pass through the selected production paths

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run receives no location sample during its bounded automated dwell
- **THEN** only that observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, state, session, background, and cleanup failures remain blocking

### Requirement: Shared firmware identity record regression evidence

Shared firmware-identity record ownership SHALL NOT be complete until focused safe-Rust, Apple C-ABI, Android JNI, Swift/JVM adapter, dependency, source-ownership, line-reduction, exact-symbol, and clean packaging checks; strict OpenSpec validation; the complete `just verify` gate; and focused Android/iOS/macOS ESP32 identity regression pass without changing the published ABI or production behavior. Semantic record evidence SHALL be exhaustive in the safe component while each bridge SHALL retain every distinct platform-boundary failure class.

#### Scenario: Verify exact shared records

- **WHEN** focused tests exercise every committed normal, optional-tail, replacement-UTF-8, maximum-bound, unsupported-command, malformed, and truncated identity vector
- **THEN** the safe component produces the existing record bytes and both compiled bridges publish exactly those bytes with unchanged statuses and metadata

#### Scenario: Preserve independent boundary failures

- **WHEN** focused C and raw-JNI tests exercise invalid pointers or environments, nulls, alignment or aliasing, array shapes, capacities, pending exceptions, output metadata, and contained panics as applicable
- **THEN** each bridge preserves its established bounded failure and publication behavior without weakening the other platform's independent contract

#### Scenario: Delete duplicate record ownership

- **WHEN** repository ownership and source audits run
- **THEN** both bridge-local record encoders, parallel layout constants/outcome types, and duplicate semantic fixture helpers are absent, the combined bridge production source is materially smaller, and the safe component contains exactly one canonical record implementation

#### Scenario: Preserve software and packaging behavior

- **WHEN** locked focused suites, Apple macOS/iOS builds, Android host and four-ABI Debug/Release packaging, exact symbol checks, and `just verify` run
- **THEN** the existing headers, adapters, two-symbol ABIs, archive/library names, source dependencies, production call paths, and all unrelated repository behavior pass unchanged without generated or committed native artifacts

#### Scenario: Preserve all-host physical behavior

- **WHEN** applicable identity, direct/CAN control, BMS, and mobile-background scenarios run against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** every host reaches ready with the expected identity and route through its production bridge and preserves protocol, control, BMS, session, background, and BLE cleanup behavior, with only bounded-time location sample absence eligible for the established manual deferral

### Requirement: Apple Rust Refloat legacy telemetry cutover evidence

The Apple Refloat legacy-telemetry cutover SHALL NOT be complete until focused safe-record, compiled C-ABI, Swift-adapter, dependency, source-ownership, exact-symbol, clean-packaging, Android-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover detail levels 1 through 4, active known and unknown firmware faults, every enum/flag and optional tail, field-weakening sentinel absence, numeric boundaries, exact record bytes and extents, malformed inputs and records, caller-region failures, contained panics, payload-safe diagnostics, and deletion of superseded Swift wire logic.

Physical Apple evidence SHALL exercise the production Rust command-10 decoder through real CoreBluetooth, direct/CAN routing, framing, polling, application state, sessions, controls, and cleanup. Android SHALL exercise its unchanged Kotlin decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell MAY be deferred to manual coverage but MUST NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial command-10 form
- **THEN** the safe component alone produces the exact bounded versioned record or stable failure with no allocation, unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the compiled Apple boundary

- **WHEN** compiled C and Swift tests exercise every valid and invalid ABI, region, capacity, status, record, and model-mapping path
- **THEN** the two public symbols, fixed statuses, exact records, errors, and publication behavior remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean macOS/iOS packaging, dependency and source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** both Apple hosts link the correct Rust bridge from source, superseded Swift algorithms are absent, Android artifacts and Kotlin ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Apple physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, telemetry, and mobile-background scenarios run on connected iOS and macOS against the ESP32 fixture
- **THEN** both hosts complete through production Rust legacy decoding with unchanged routing, state, session, control, background, and cleanup behavior

#### Scenario: Verify Android physical regression

- **WHEN** the same applicable scenarios run on the connected Android device
- **THEN** Android completes through its unchanged Kotlin legacy decoder without resolving or packaging the Apple bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Apple Rust Refloat configuration codec cutover evidence

The repository SHALL prove source parity, canonical-record safety, ABI safety, strict Swift adaptation, clean packaging, sole production selection, and all-host physical behavior before accepting the Apple Rust Refloat configuration codec cutover.

#### Scenario: Exercise safe canonical records

- **WHEN** safe-crate tests run
- **THEN** they SHALL round-trip every schema field/default/option/choice/description/group and every typed value
- **AND** SHALL reject truncation, trailing bytes, count and length overflow, unknown versions/kinds/types, nonzero reserved bytes, invalid UTF-8, non-finite numbers, non-canonical booleans, duplicate/missing/extra fields, schema mismatch, and configured size bounds
- **AND** SHALL preserve upstream fixture fingerprint, wire signature, value bytes, and checksum evidence

#### Scenario: Exercise the compiled Apple ABI

- **WHEN** Rust bridge tests and a compiled-C harness invoke all four symbols
- **THEN** they SHALL prove ABI versioning, exact record bytes, fixed status mapping, caller-owned output behavior, too-small output handling, null and oversize rejection, misaligned scalar rejection, overlapping-region rejection, deterministic outputs, panic containment, and no retained state

#### Scenario: Exercise the strict Swift adapter

- **WHEN** focused Apple tests run
- **THEN** they SHALL prove exact projection of schema and values, mechanical value-record construction, stale-generation rejection, current/default/write parity, bridge-error mapping, ABI mismatch rejection, malformed-record rejection, and schema/write/readback service behavior
- **AND** SHALL prove production code cannot select or fall back to superseded Swift codec algorithms

#### Scenario: Verify clean packaging and source ownership

- **WHEN** repository audits and clean package builds run
- **THEN** macOS, iOS device, and iOS simulator SHALL build the selected bridge from locked source and link the intended archive and C module
- **AND** no generated binding or native archive SHALL be committed
- **AND** Android's typed bridge catalogue, Gradle tasks, packages, and artifacts SHALL remain exactly unchanged
- **AND** source audits SHALL find one safe Rust protocol owner and only bounded ABI/record adaptation in Apple production sources

#### Scenario: Pass complete software regression

- **WHEN** the standard repository software gate runs after production selection and Swift deletion
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass

#### Scenario: Verify every production host with the ESP32 fixture

- **WHEN** the flashed ESP32 fixture and production Android, iOS, and macOS apps run the physical checkpoint
- **THEN** Android SHALL continue exercising its selected Kotlin configuration codec
- **AND** iOS and macOS SHALL exercise the selected Rust configuration codec
- **AND** each host SHALL pass BLE connection, direct and CAN telemetry, Refloat configuration schema/current/default/write/readback behavior, controller and BMS routing, reconnect, files, theme, and applicable lifecycle/background checks
- **AND** mobile background logging SHALL remain successful
- **AND** only a bounded unavailable location sample MAY be recorded for later manual verification
- **AND** no BLE, codec, configuration, packaging, routing, persistence, background, or application failure MAY be deferred

### Requirement: Rust Refloat configuration codec candidate evidence

Candidate acceptance SHALL combine exhaustive source-derived schema/value parity, adversarial bounded-input evidence, repository ownership and production-isolation audits, the complete software gate, and unchanged selected-path physical regression on Android, iOS, and macOS. Physical runs during this unselected extraction SHALL prove regression freedom only and MUST NOT be reported as Rust production selection. Only absence of a location sample during the bounded acquisition window MAY remain deferred manual coverage; every other assertion and cleanup MUST pass.

#### Scenario: Exercise source-derived parity

- **WHEN** focused candidate tests consume all nine immutable Refloat schema families and their default, current, accepted-write, and expected-readback values
- **THEN** exact manifest fingerprints, signatures, counts, normalized records, typed values, and byte-for-byte encodings match the committed native and conformance evidence

#### Scenario: Exercise adversarial bounds

- **WHEN** focused tests cover compressed and expanded extents, zlib completion and trailing input, XML constructs and limits, metadata and ordering faults, every supported wire type, signature/truncation/remainder faults, numeric extrema, string validity, and missing/extra/wrong values
- **THEN** the candidate returns deterministic payload-free failures without panic, partial publication, unbounded work, or broadened schema acceptance

#### Scenario: Prove software isolation

- **WHEN** focused checks, native parity tests, dependency and unsafe-code audits, bridge/build/package/symbol ownership audits, strict OpenSpec validation, and `just verify` run
- **THEN** all gates pass while both production host codecs and every selected artifact remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** the connected Android device, iOS device, and macOS host run their applicable identity, direct/CAN control, BMS, configuration schema/current/default/write/readback, telemetry, ride, unexpected-disconnect, and background/session scenarios against the ESP32 fixture
- **THEN** every non-location assertion and cleanup passes through the unchanged selected host paths and the evidence is recorded as regression-only

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile device has authorized background location capability but produces no sample within the harness acquisition window
- **THEN** only that sample assertion is recorded for later manual verification while BLE, logging, lifecycle, persistence, and all other scenario requirements remain mandatory

### Requirement: Android Rust Refloat compatibility cutover evidence

The Android Refloat compatibility cutover SHALL NOT be complete until focused safe-record, JNI-boundary, host-JVM adapter, dependency, source-ownership, exact-symbol, clean-packaging, Apple-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact INFO request bytes; minimal/source legacy, compact major/minor, and complete version-2 responses; legacy patch absence; every audited release/profile; unknown releases; canonical compatibility records and corruption; required layout groups; bounded future identifiers/order; every malformed payload/record/layout/environment/reference/array/capacity boundary; contained panics; payload-safe diagnostics; and deletion of superseded Kotlin algorithms.

Physical Android evidence SHALL exercise production Rust INFO/profile/layout admission through real BLE, direct/CAN routing, framing, discovery, telemetry, controls, application state, sessions, background execution, reconnect recovery, and cleanup. Apple SHALL exercise its unchanged selected compatibility owner and real CoreBluetooth path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe compatibility records

- **WHEN** focused Rust tests exercise every source-derived and adversarial INFO/profile/layout form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes the exact bounded decision or stable failure with no unsafe code, private diagnostic, or bridge-local serializer

#### Scenario: Verify the Android boundary and adapter

- **WHEN** focused JNI and host-JVM tests exercise every valid and invalid ABI, environment, exception, reference, array, capacity, status, record, request, layout, and model-mapping path
- **THEN** the exact public symbols, fixed statuses, canonical records, errors, and publication behavior remain deterministic with no partial output or fallback policy

#### Scenario: Verify software selection and isolation

- **WHEN** clean host and four-ABI Android packaging, macOS/iOS packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** Android loads the correct Rust bridge from source, superseded Kotlin algorithms are absent, Apple artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Android physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on the connected Android device against the ESP32 fixture
- **THEN** Android completes through production Rust compatibility decisions with unchanged routing, state, session, control, background, recovery, and cleanup behavior

#### Scenario: Verify Apple physical regression

- **WHEN** the same applicable scenarios run on connected iOS and macOS
- **THEN** both Apple hosts complete through their unchanged selected compatibility bridge without resolving, packaging, or calling the Android JNI bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Apple Rust Refloat compatibility cutover evidence

The Apple Refloat compatibility cutover SHALL NOT be complete until focused safe-record, compiled C-ABI, Swift-adapter, dependency, source-ownership, exact-symbol, clean-packaging, Android-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact INFO request bytes; minimal/source legacy, compact major/minor, and complete version-2 responses; legacy patch absence; every audited release/profile; unknown releases; canonical compatibility records and corruption; required layout groups; bounded future identifiers/order; every malformed payload/record/layout/region/capacity boundary; contained panics; payload-safe diagnostics; and deletion of superseded Swift algorithms.

Physical Apple evidence SHALL exercise production Rust INFO/profile/layout admission through real CoreBluetooth, direct/CAN routing, framing, discovery, telemetry, controls, application state, sessions, reconnect recovery, and cleanup. Android SHALL exercise its unchanged Kotlin compatibility owner and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe compatibility records

- **WHEN** focused Rust tests exercise every source-derived and adversarial INFO/profile/layout form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes the exact bounded decision or stable failure with no unsafe code, private diagnostic, or bridge-local serializer

#### Scenario: Verify the compiled Apple boundary

- **WHEN** compiled C and Swift tests exercise every valid and invalid ABI, region, capacity, status, record, request, and model-mapping path
- **THEN** the exact public symbols, fixed statuses, canonical records, errors, and publication behavior remain deterministic with no partial output or fallback policy

#### Scenario: Verify software selection and isolation

- **WHEN** clean macOS/iOS packaging, Android host/four-ABI packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** both Apple hosts link the correct Rust bridge from source, superseded Swift algorithms are absent, Android artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Apple physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on connected iOS and macOS against the ESP32 fixture
- **THEN** both hosts complete through production Rust compatibility decisions with unchanged routing, state, session, control, background where applicable, recovery, and cleanup behavior

#### Scenario: Verify Android physical regression

- **WHEN** the same applicable scenarios run on the connected Android device
- **THEN** Android completes through its unchanged Kotlin compatibility path without resolving, packaging, or calling the Apple bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Rust Refloat compatibility candidate evidence

The Rust Refloat compatibility candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source and canonical fixture parity, adversarial identity/catalogue/layout tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover exact INFO request bytes; minimal and source legacy response forms; compact major/minor and complete version-2 identities; legacy patch absence; every audited source/wire catalogue entry; telemetry, lighting, and fatal-clear profiles; unknown and prerelease-equivalent rejection; all required identifier names and groups; bounded extras, ordering, counts, lengths, aggregate bytes, UTF-8, uniqueness, missing/wrong-group names; wrong envelopes, unsupported protocol versions, truncation, excessive payloads; deterministic complete results; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat INFO/compatibility, telemetry, scheduling, session, control, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust component did not regress discovery, direct/CAN readiness, profile selection, telemetry, rides, controls, background continuity, unexpected-disconnect recovery, or cleanup. Later host-cutover changes SHALL rerun focused INFO/profile selection through Rust before native deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify source-derived compatibility parity

- **WHEN** focused Rust tests exercise every source/canonical INFO shape, audited release, profile, and required layout
- **THEN** request bytes, structural identities, catalogue/profile results, and layout admission agree exactly with pinned source and canonical compatibility requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every envelope, protocol byte, extent, identity, catalogue miss, field count, name length, aggregate length, UTF-8, uniqueness, ordering, group, and missing-name boundary
- **THEN** the candidate either returns the exact bounded decision or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols, build catalogues, packaged artifacts, and production call sites remain unchanged

#### Scenario: Verify all-host physical compatibility regression

- **WHEN** focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, INFO/profile admission, routing, telemetry, state, session/control behavior, background continuity where applicable, recovery, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust INFO/profile execution, and superseded-code deletion independently

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking

### Requirement: Android Rust Refloat dynamic telemetry cutover evidence

The Android Refloat dynamic-telemetry cutover SHALL NOT be complete until focused safe-record, JNI-boundary, host-JVM adapter, connection-lifetime, dependency, source-ownership, exact-symbol, clean-packaging, Apple-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact source-derived command-32 layouts; 1.1.x and 1.2.x command-31 running, non-running, charging, active-alert, and firmware-fault forms; complete source float16 values; every field/name/aggregate/extension bound; ordered extensions; exact record bytes/extents; layout reset/replacement; malformed inputs/records; JVM environment, exception, reference, array, distinctness, and capacity failures; contained panics; payload-safe diagnostics; and deletion of superseded Kotlin wire logic.

Physical Android evidence SHALL exercise the production Rust command-31/32 decoder through real BLE, direct/CAN routing, framing, polling, application state, sessions, controls, background execution, reconnect recovery, and cleanup. Apple SHALL exercise its unchanged selected Rust decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial layout/realtime form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes exact bounded versioned records or stable failures with no unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the Android boundary and adapter

- **WHEN** focused JNI and host-JVM tests exercise every valid and invalid ABI, environment, exception, reference, array, capacity, status, record, layout-lifetime, and model-mapping path
- **THEN** the three public symbols, fixed statuses, exact records, errors, reset behavior, and publication remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean host and four-ABI Android packaging, macOS/iOS packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** Android loads the correct Rust bridge from source, superseded Kotlin algorithms are absent, Apple artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Android physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on the connected Android device against the ESP32 fixture
- **THEN** Android completes through production Rust dynamic decoding with unchanged routing, state, session, control, background, recovery, and cleanup behavior

#### Scenario: Verify Apple physical regression

- **WHEN** the same applicable scenarios run on connected iOS and macOS
- **THEN** both Apple hosts complete through their unchanged selected Rust decoder without resolving, packaging, or calling the Android JNI bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Apple Rust Refloat dynamic telemetry cutover evidence

The Apple Refloat dynamic-telemetry cutover SHALL NOT be complete until focused safe-record, compiled C-ABI, Swift-adapter, connection-lifetime, dependency, source-ownership, exact-symbol, clean-packaging, Android-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover exact source-derived command-32 layouts; 1.1.x and 1.2.x command-31 running, non-running, charging, active-alert, and firmware-fault forms; complete source float16 values; every field/name/aggregate/extension bound; ordered extensions; exact record bytes/extents; layout reset/replacement; malformed inputs/records; caller-region failures; contained panics; payload-safe diagnostics; and deletion of superseded Swift wire logic.

Physical Apple evidence SHALL exercise the production Rust command-31/32 decoder through real CoreBluetooth, direct/CAN routing, framing, polling, application state, sessions, controls, background execution, reconnect recovery, and cleanup. Android SHALL exercise its unchanged Kotlin decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell SHALL be deferred manual coverage only and SHALL NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial layout/realtime form and corrupt every record boundary
- **THEN** the safe component alone produces and consumes exact bounded versioned records or stable failures with no unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the compiled Apple boundary

- **WHEN** compiled C and Swift tests exercise every valid and invalid ABI, region, capacity, status, record, layout-lifetime, and model-mapping path
- **THEN** the three public symbols, fixed statuses, exact records, errors, reset behavior, and publication remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean macOS/iOS packaging, Android host/four-ABI packaging, dependency/source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** both Apple hosts link the correct Rust bridge from source, superseded Swift algorithms are absent, Android artifacts and Kotlin ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Apple physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on connected iOS and macOS against the ESP32 fixture
- **THEN** both hosts complete through production Rust dynamic decoding with unchanged routing, state, session, control, background where applicable, recovery, and cleanup behavior

#### Scenario: Verify Android physical regression

- **WHEN** the same applicable scenarios run on the connected Android device
- **THEN** Android completes through its unchanged Kotlin dynamic decoder without resolving, packaging, or calling the Apple bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Rust Refloat dynamic telemetry candidate evidence

The Rust Refloat dynamic telemetry candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source fixture parity, adversarial layout/structural/numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover the exact shared command-32 layout; 1.1.x and 1.2.x running responses; running-only absence and presence; charging and active-alert tails; source ticks; package state, mode, footpad, stop, adjustment, alert, fatal-latch, and firmware-fault mappings; all known fields; unknown bounded extensions; every field/name/aggregate/extension boundary; the complete finite source float16 domain; exact and inexact extents; wrong envelopes, commands, masks, and tail relations; deterministic complete output; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat telemetry decoder, scheduling, telemetry, session, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, dynamic telemetry, rides, controls, background continuity, unexpected disconnect recovery, or cleanup. Later host-cutover changes SHALL rerun focused command-31/32 reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, telemetry, routing, state, session, control, background, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode every committed command-32 and command-31 fixture
- **THEN** every envelope, identifier list, mask, extent, source tick, scalar, semantic code, flag, optional group, alert, fault, and extension behavior agrees with pinned Refloat source and the stable normalized telemetry requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every field count, identifier length, aggregate length, UTF-8, uniqueness, extension, mask, payload extent, float16, semantic code, flag, and tail-relation boundary
- **THEN** the candidate either returns the exact bounded layout/sample or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical telemetry regression

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic telemetry, ride, unexpected-disconnect, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, routing, telemetry, state, session/control behavior, background continuity where applicable, recovery, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, variable record bounds, packaging, production selection, physical Rust command-31/32 execution, complete float16 behavior, and superseded-code deletion independently

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking

### Requirement: Android Rust Refloat legacy telemetry cutover evidence

The Android Refloat legacy-telemetry cutover SHALL NOT be complete until focused safe-record, JNI-boundary, host-JVM adapter, dependency, source-ownership, exact-symbol, clean-packaging, Apple-isolation, strict OpenSpec, and complete `just verify` checks pass, followed by focused Android, iOS, and macOS ESP32 regression. Evidence SHALL cover detail levels 1 through 4; active known and unknown firmware faults; every enum, flag, and optional tail; field-weakening sentinel absence; numeric boundaries; exact record bytes and extents; malformed inputs and records; JVM environment, exception, reference, array, distinctness, and capacity failures; contained panics; payload-safe diagnostics; and deletion of superseded Kotlin wire logic.

Physical Android evidence SHALL exercise the production Rust command-10 decoder through real BLE, direct/CAN routing, framing, polling, application state, sessions, controls, background execution, and cleanup. Apple SHALL exercise its unchanged selected Rust decoder and real BLE path to prove platform isolation. A missing location sample within a bounded automated dwell MAY be deferred to manual coverage but MUST NOT conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify canonical safe records

- **WHEN** focused Rust tests decode every source-derived and adversarial command-10 form
- **THEN** the safe component alone produces the exact bounded versioned record or stable failure with no allocation, unsafe code, non-finite value, private diagnostic, or bridge-local serializer

#### Scenario: Verify the Android boundary and adapter

- **WHEN** focused JNI and host-JVM tests exercise every valid and invalid ABI, environment, exception, reference, array, capacity, status, record, and model-mapping path
- **THEN** the two public symbols, fixed statuses, exact records, errors, and publication behavior remain deterministic with no partial output or fallback parser

#### Scenario: Verify software selection and isolation

- **WHEN** clean host and four-ABI Android packaging, dependency and source audits, strict OpenSpec validation, and `just verify` run after selection
- **THEN** Android loads the correct Rust bridge from source, superseded Kotlin algorithms are absent, Apple artifacts and ownership are unchanged, and all unrelated behavior passes

#### Scenario: Verify Android physical selection

- **WHEN** focused identity, direct/CAN control, applicable BMS, telemetry, ride, unexpected-disconnect, and mobile-background scenarios run on the connected Android device against the ESP32 fixture
- **THEN** Android completes through production Rust legacy decoding with unchanged routing, state, session, control, background, and cleanup behavior

#### Scenario: Verify Apple physical regression

- **WHEN** the same applicable non-lifecycle and mobile-background scenarios run on connected iOS and macOS
- **THEN** both Apple hosts complete through their unchanged selected Rust decoder without resolving, packaging, or calling the Android JNI bridge

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile run receives no location sample within its bounded dwell
- **THEN** only that observation is deferred and every non-location assertion and cleanup remains mandatory

### Requirement: Android Rust Refloat configuration codec cutover evidence

The repository SHALL prove safe-record reuse, JNI safety, strict Kotlin adaptation, clean packaging, sole Android production selection, Apple isolation, and all-host physical behavior before accepting the Android Rust Refloat configuration codec cutover.

#### Scenario: Exercise the compiled Android JNI boundary

- **WHEN** focused Rust bridge tests and host-JVM calls invoke all four methods
- **THEN** they SHALL prove ABI versioning, exact schema/value records and payloads, fixed status mapping, atomic two-pass output behavior, null/exception/reference/length/capacity/distinctness rejection, deterministic outputs, panic containment, and no retained state

#### Scenario: Exercise the strict Kotlin adapter

- **WHEN** focused Android tests run against every audited schema and value fixture
- **THEN** they SHALL prove exact schema/value projection, mechanical value-record construction, every typed value, stale-generation rejection, current/default/write/readback parity, ABI/status mapping, malformed-record rejection, and service behavior
- **AND** SHALL prove production code cannot select or fall back to superseded Kotlin codec algorithms

#### Scenario: Verify clean packaging and source ownership

- **WHEN** repository audits and clean package builds run
- **THEN** host JVM and every supported Android ABI in Debug and Release SHALL build and consume the selected JNI library from locked source
- **AND** no generated binding or native artifact SHALL be committed
- **AND** Apple's package graph, bridge ABI, adapter, archives, symbols, and selected behavior SHALL remain exactly unchanged
- **AND** source audits SHALL find one safe Rust protocol owner and only bounded JNI/record adaptation in Android production sources

#### Scenario: Pass complete software regression

- **WHEN** the standard repository software gate runs after Android production selection and Kotlin algorithm deletion
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass

#### Scenario: Verify every production host with the ESP32 fixture

- **WHEN** the flashed ESP32 fixture and production Android, iOS, and macOS apps run the physical checkpoint
- **THEN** every host SHALL exercise its selected Rust Refloat configuration codec
- **AND** each host SHALL pass BLE connection, direct and CAN telemetry, configuration schema/current/default/write/readback behavior, controller and BMS routing, reconnect, files, theme, persistence, and applicable lifecycle/background checks
- **AND** mobile background logging SHALL remain successful
- **AND** only a bounded unavailable location sample MAY be recorded for later manual verification
- **AND** no BLE, codec, configuration, packaging, routing, persistence, background, application, or cleanup failure MAY be deferred

### Requirement: Rust Refloat configuration service candidate evidence

The Rust Refloat configuration service candidate SHALL NOT be treated as a production owner until locked formatting, warnings-denied linting, dependency and workspace audits, source-derived catalogue/fixture parity, exhaustive native-transition and adversarial tests, the complete `just verify` software gate, and Android/iOS/macOS ESP32 regression pass. Candidate evidence SHALL cover cache hit/miss/corruption, every audited release, legacy patch resolution, direct/CAN opaque routes, schema paging bounds, generation and operation identity, current/default revisions, read-only and flywheel policy, write acknowledgement/readback, normalization, conflicts, failures, aborts, reset/reconnect, counter boundaries, deterministic snapshots/effects, and payload-safe diagnostics.

#### Scenario: Replay every selected native transition

- **WHEN** focused Rust tests replay the complete Swift and Kotlin service vectors using immutable release fixtures
- **THEN** every snapshot, request effect, cache decision, payload, revision, operation, error, recovery, and normalization result SHALL match the selected native behavior

#### Scenario: Exercise adversarial lifecycle sequences

- **WHEN** focused tests permute stale generations, operations, routes, indexes, commands, chunks, offsets, lengths, responses, acknowledgements, failures, flywheel transitions, resets, and counter boundaries
- **THEN** the candidate SHALL remain bounded and deterministic, fail closed with stable payload-free errors, and publish no partial state or effect

#### Scenario: Verify locked isolation

- **WHEN** workspace, dependency, source-selection, bridge-catalogue, symbol, generated-artifact, and package audits run
- **THEN** the candidate SHALL depend only on the existing safe codec, contain no unsafe or host integration, and leave every production artifact and selected owner unchanged

#### Scenario: Pass complete software regression

- **WHEN** `just verify` runs with the isolated candidate present
- **THEN** all frontend, Rust, Swift, Kotlin, conformance, formatting, lint, audit, and packaging checks SHALL pass without a contract or persistent-format change

#### Scenario: Verify all-host physical regression

- **WHEN** production Android, iOS, and macOS execute their complete applicable ESP32 physical matrices
- **THEN** each host SHALL preserve BLE, direct/CAN configuration schema/current/default/write/readback, telemetry, BMS, controls, reconnect, sessions, persistence, files, theme, cleanup, and mobile background logging behavior through its unchanged selected native service
- **AND** only bounded mobile location-sample absence MAY be deferred for manual verification
- **AND** no non-location, application, protocol, persistence, background, or cleanup failure MAY be deferred

#### Scenario: Defer host cutover evidence

- **WHEN** candidate parity and regression checks pass
- **THEN** the change SHALL record extraction readiness only and SHALL require separate Apple and Android changes to prove adapter mapping, production selection, physical Rust execution, and native-service deletion

### Requirement: Rust VESC request scheduler candidate evidence

The Rust request scheduler extraction SHALL be backed by source-derived parity, adversarial bounds, production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior with the ESP32 fixture before the candidate may be treated as ready for later composition.

#### Scenario: Replay source-derived scheduler behavior

- **WHEN** the focused Rust suite runs vectors derived from the complete Swift and Kotlin scheduler tests and implementations
- **THEN** every priority, FIFO, route/context match, generation, deadline, timeout, cancellation, one-way, deferred, diagnostic, and completion transition SHALL match the selected behavior

#### Scenario: Exercise adversarial scheduler bounds

- **WHEN** focused tests reach maximum queue and payload bounds, invalid facts, stale or repeated events, cancellation races, deferred pumping, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public inputs, and free of duplicate or misattributed completions

#### Scenario: Prove production isolation

- **WHEN** dependency, symbol, package, build-catalogue, source, and generated-artifact audits run
- **THEN** neither host SHALL select or package the candidate and no new Apple FFI, Android JNI, callback, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** `just verify` and strict OpenSpec validation SHALL pass with the native schedulers still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, connection, protocol, session, background, import/export, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust Refloat telemetry service candidate evidence

The Rust Refloat telemetry-service extraction SHALL be backed by source-derived parity, adversarial bounds, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior with the ESP32 fixture before it may be treated as ready for later unified-runtime composition.

#### Scenario: Replay complete native telemetry policy

- **WHEN** focused Rust tests run vectors derived from the complete Swift and Kotlin telemetry services, protocol models, and canonical telemetry requirements
- **THEN** layout retry, exact request facts/bytes, pending attribution, timeout recovery, cadence, compact replacement/merge, normalization, sequence, elapsed time, reset, and ordered effects SHALL match selected behavior

#### Scenario: Exercise adversarial service bounds

- **WHEN** tests reach invalid settings/time arithmetic, maximum routes/generations/tokens/sequences/extensions, stale or repeated timers/completions/responses, active faults, cache replacement, decreasing totals, stop/reset races, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public inputs, and free of duplicate, stale, or misattributed publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, symbol, package, build-catalogue, source, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe Rust components, neither host SHALL select or package it, and no new Apple FFI, Android JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests/audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native services still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, connection, telemetry cadence, session, background, import/export, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust VESC compatibility candidate evidence

The Rust VESC compatibility extraction SHALL be backed by immutable fixture composition, exhaustive source-policy parity, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before it may be used by later Rust application services.

#### Scenario: Compose immutable firmware fixtures

- **WHEN** focused Rust tests unwrap and decode the audited THOR300 6.02, 6.05, 6.06, and 7.00 firmware fixtures
- **THEN** each exact stable identity SHALL receive the expected read/write and shutdown decisions through the safe identity and compatibility components

#### Scenario: Exhaust the finite compatibility policy

- **WHEN** focused tests enumerate every major/minor pair, hardware-type and test-marker boundaries, both operations, exact shutdown releases, adjacent versions, and unrelated identity-field changes
- **THEN** every decision SHALL match the complete selected Swift/Kotlin policy without inference or partial admission

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the safe firmware-identity component, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native compatibility owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, compatibility, access, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust controller-discovery candidate evidence

The Rust controller-discovery extraction SHALL be backed by shared source-derived vectors, exhaustive and adversarial parity, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime composition.

#### Scenario: Replay shared discovery vectors

- **WHEN** focused Rust tests run every committed controller-discovery TSV vector and source-derived native lifecycle case
- **THEN** route, ordered effects, candidates, inventory, generations, readiness, and stable issue outcomes SHALL match the selected Swift and Kotlin state machines

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover every CAN ID boundary, duplicate/order normalization, 64/65 devices, 16/17 controllers, supported/unsupported/test firmware, meaningful and sentinel UUIDs, configured/hinted/continuity paths, timeouts, stale facts, invalid selections, reset, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe components, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests/audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native discovery owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, compatibility, candidate selection, continuity, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust auxiliary-node discovery candidate evidence

The Rust auxiliary-node discovery extraction SHALL be backed by shared source-derived vectors, exhaustive and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime composition.

#### Scenario: Replay shared auxiliary vectors

- **WHEN** focused Rust tests run every committed auxiliary-discovery vector derived from both selected native suites and canonical BMS requirements
- **THEN** exact identity admission, route, ordered effects, candidates, status, generations, observation presence, and stable issue outcomes SHALL match the source behavior except that the documented seventeenth-BMS edge SHALL follow canonical fail-closed behavior

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover every CAN ID, duplicate/order normalization, 64/65 devices, 16/17 BMS identities, direct/preferred/reused/enumerated routes, unsupported identities, timeouts, stale and mismatched work, reset, repeated transitions, counter rollover, and private values
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on safe controller discovery, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native auxiliary owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, BMS behavior, compatibility, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust VESC BMS telemetry service candidate evidence

The Rust VESC BMS telemetry-service extraction SHALL be backed by shared source-derived lifecycle vectors, exhaustive and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime composition.

#### Scenario: Replay shared BMS lifecycle vectors

- **WHEN** focused Rust tests run every committed source-derived controller-attached and standalone BMS discovery/polling lifecycle vector
- **THEN** request bytes/facts, ordered effects, route, cadence, failures, status, observation presence, generations, reset, and stable diagnostics SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover direct/preferred/reused/enumerated routes, every CAN ID, maximum inventories and command-96 collections, all scheduler completions, malformed payloads, stale and mismatched input, three-failure recovery, timer replacement, reset, repeated transitions, counter rollover, and private values
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, read-only, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, executor, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the three named safe crates, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native BMS lifecycle owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, BMS polling, standalone behavior, compatibility, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust session SQLite pruning candidate evidence

The Rust session-pruning extraction SHALL be backed by exact canonical-database fixtures, exhaustive transaction and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged available-device physical behavior. Android physical behavior SHALL pass the canonical fixture matrix before any later Rust catalog/runtime selection; an unavailable handset MAY be recorded and deferred from this unselected slice but MUST NOT be replaced by emulator evidence.

#### Scenario: Replay exact preview and commit vectors

- **WHEN** focused tests replay empty, dense, endpoint, field/event/summary-source, phase-transition, extrema-tie, location-neighborhood, initial-prune, repeated-coarser, and no-op vectors
- **THEN** selected sequences, preview summaries, retained rows/evidence, deleted locations, metadata, recorded/stored counts, commit markers, history revisions, and reopen state SHALL match immutable expected projections

#### Scenario: Exercise stale, transactional, and resource bounds

- **WHEN** tests cover every resolution/alignment/source-count/integer/order/data bound, stale marker/resolution/identity, foreign keys, active WAL, busy/locked state, cancellation, corrupt/incompatible schema, and injected failure point
- **THEN** pruning SHALL remain bounded and deterministic, commit no partial mutation, preserve prior durable state, release every transaction/resource, and expose only stable payload-free failures

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, copied-source, schema-identity, arbitrary-SQL, public-SQLite-type, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, catalog, server, generated-binding, and native-file audits run
- **THEN** `session-sqlite` SHALL call `session-history-kernel` directly through only its allowed graph, neither host SHALL select or package pruning, and no pruning FFI/JNI, runtime fallback, copied schema, production source, or canonical SQL change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused locked tests/audits, warnings-denied workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native pruning owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification

#### Scenario: Defer unavailable Android hardware honestly

- **WHEN** macOS and iOS pass their canonical ESP32 matrices but no physical Android handset is connected for an unselected, production-isolated extraction
- **THEN** the change SHALL record Android as unavailable, SHALL NOT claim emulator results as physical evidence, and SHALL retain the complete Android ESP32 matrix as a mandatory migration-final gate before Rust runtime selection

### Requirement: Rust session SQLite reader candidate evidence

The Rust session-reader extraction SHALL be backed by exact canonical database/query fixtures, exhaustive read/adversarial/concurrency behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later catalog/runtime selection.

#### Scenario: Preserve writer behavior through consolidation

- **WHEN** the writer-only package is renamed into the consolidated session SQLite package
- **THEN** every existing exact schema, creation, reopen, sample/location transaction, rollback, marker, bound, and diagnostic fixture SHALL pass unchanged and no superseded crate, fixture, recipe, audit, or bridge path SHALL remain

#### Scenario: Replay exact reader query vectors

- **WHEN** focused tests read committed metadata/marker/field facts and published stable/extension series, sample, event, and location vectors across nulls, ties, gaps, pages, continuation keys, active WAL, and reopen
- **THEN** every ordered key, nullable value, descriptor, generation, details payload, location gap/boundary, source-row count, and has-more result SHALL match immutable expected projections

#### Scenario: Exercise read failure and resource bounds

- **WHEN** tests cover every field/range/bucket/source/page/integer bound, stale/future/revision boundary, corrupt/incompatible schema, malformed/nonfinite durable value, unsupported field, invalid continuation, busy/locked state, cancellation, and diagnostic boundary
- **THEN** the reader SHALL remain bounded and deterministic, return no partial result, release every transaction/resource, remain usable after interruption, and expose only stable payload-free failures

#### Scenario: Prove WAL writer compatibility

- **WHEN** adversarial tests hold a published read snapshot while `SessionWriter` commits a newer sample/location batch
- **THEN** writer progress SHALL remain independent, the reader SHALL observe only its requested boundary, a later transaction SHALL observe the newer marker, and no read shall mutate or checkpoint the database

#### Scenario: Prove dependency and production isolation

- **WHEN** lockfile, dependency, copied-source, schema-identity, arbitrary-SQL, public-SQLite-type, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, server, generated-binding, and native-file audits run
- **THEN** one consolidated candidate SHALL own per-session SQLite write/read behavior through only its allowed graph, neither host SHALL select or package it, and no session SQLite FFI/JNI, runtime fallback, copied schema, or production source change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests/audits, warnings-denied offline locked workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native stores and readers still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust session SQLite writer candidate evidence

The Rust session-writer extraction SHALL be backed by exact canonical-schema and database-state fixtures, exhaustive transaction/adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later catalog/runtime selection.

#### Scenario: Prove exact schema creation

- **WHEN** focused tests create a session database from the compiled schema
- **THEN** the schema input SHALL be byte-identical to `backends/sql/session.sql` and required pragmas, tables, indexes, constraints, metadata, singleton marker, and initial values SHALL match canonical expected evidence

#### Scenario: Replay exact sample state vectors

- **WHEN** focused tests replay committed direct/dynamic/legacy, nullable, extension, transition, duplicate, extrema-tie, multi-batch, and reopen sample vectors
- **THEN** sample rows, extension JSON, numeric-field sources, event rows, recorded/stored/event counts, committed sequences/time, riding elapsed, distance, extrema/source sequences, and last state SHALL match immutable expected database projections

#### Scenario: Replay exact location state vectors

- **WHEN** focused tests replay unordered, duplicate, boundary-coordinate, nullable-accuracy, multi-batch, and reopen location vectors
- **THEN** stored order/sequences and every committed/recorded/stored/status marker field SHALL match immutable expected database projections

#### Scenario: Exercise transaction and adversarial bounds

- **WHEN** tests cover every empty/minimum/maximum batch, integer conversion, order, enum, finite numeric, extension count/name/JSON, timestamp/elapsed, coordinate/accuracy, target existence, schema/singleton/cadence, corrupt database, busy/locked, injected mid-transaction failure, reopen, and diagnostic boundary
- **THEN** the writer SHALL remain bounded and deterministic, commit no partial batch, preserve prior durable state, and expose only stable payload-free failures

#### Scenario: Prove dependency and production isolation

- **WHEN** lockfile, dependency, copied-source, schema-identity, source-selection, bridge-catalogue, symbol, Gradle, SwiftPM/Xcode, packaged-artifact, callback, executor, catalog, server, generated-binding, and native-file audits run
- **THEN** the candidate SHALL own the SQLite write behavior through only its allowed graph, neither host SHALL select or package it, and no writer FFI/JNI, runtime fallback, copied schema, or production source change SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests/audits, warnings-denied offline locked workspace linting, `just verify`, `git diff --check`, and strict OpenSpec validation SHALL pass with native stores still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass through unchanged production BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session SQLite/history/pruning, background, files, theme, recovery, and cleanup behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust session-history kernel candidate evidence

The Rust session-history extraction SHALL be backed by shared immutable expected vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical behavior before later persistence/runtime selection.

#### Scenario: Replay shared series vectors

- **WHEN** focused Rust tests replay committed edge, tie, gap, late-field, null, phase-cadence, retained-resolution, bucket-count, dense, and ordered-finalization vectors
- **THEN** M4 source sequences, finite values, continuity generations, field order, role deduplication, and bounds SHALL match the canonical expected evidence

#### Scenario: Replay shared prune vectors

- **WHEN** focused Rust tests replay committed endpoints, field starts, phase boundaries, event/summary protection, aligned buckets, extrema ties, multi-role coverage, eight-row-cap, and repeated-coarser vectors
- **THEN** retained sequences, protected/ordinary classification, uncovered roles, chronological finalization, and bounds SHALL match the canonical expected evidence

#### Scenario: Exercise complete adversarial bounds

- **WHEN** tests cover every empty/minimum/maximum field, bucket, row, resolution, threshold, cadence, retained-resolution, elapsed, sequence, numeric, ordering, arity, generation, role, output, and arithmetic boundary plus malformed fixture input
- **THEN** the candidate SHALL remain deterministic, bounded, standard-library-only, panic-free for public input, and free of partial or private-data diagnostics

#### Scenario: Prove direct future ownership and production isolation

- **WHEN** dependency, copied-source, source-selection, bridge-catalogue, symbol, package, callback, SQLite, persistence, network, generated-artifact, and native-file audits run
- **THEN** the candidate SHALL own the safe algorithms without importing a native/frontend implementation, neither host SHALL select or package it, and no FFI/JNI, generated binding, database/runtime dependency, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native session owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, discovery, configuration, controls, shutdown, telemetry, BMS, session creation/history/pruning, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust controller current-limit composition evidence

The Rust controller current-limit application composition SHALL be backed by shared source-derived activation/completion vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later runtime selection.

#### Scenario: Replay shared current-limit composition vectors

- **WHEN** focused Rust tests replay committed direct/CAN activation order, exact command-14 routing, valid/optional-cell responses, standalone-BMS exclusion, malformed/timeout/queue/cancellation, reconnect, and reset vectors
- **THEN** scheduler facts, decoded values, snapshots, publications, diagnostics, and effect order SHALL match the selected Swift and Kotlin application behavior

#### Scenario: Exercise the complete current-limit safety matrix

- **WHEN** tests cover supported controller and standalone-BMS identities, every route, valid/unknown/truncated signatures, absent/zero/positive cell counts, finite/nonzero/invalid auto-floats, every scheduler completion, stale/mismatched/repeated input, maximum payload, generation replacement, reconnect, and reset
- **THEN** the composition SHALL remain bounded, deterministic, panic-free for public input, non-fatal to readiness, and free of stale, partial, or private-data publication

#### Scenario: Prove direct dependency and production isolation

- **WHEN** dependency, copied-logic, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the coordinator SHALL directly use the safe controller-data crate, neither host SHALL select or package the coordinator, and no new FFI/JNI, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the composition is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native application owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN current-limit publication, discovery, configuration, controls, shutdown, telemetry, BMS, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust device configuration composition evidence

The Rust device/configuration connection-coordinator composition SHALL be backed by shared source-derived lifecycle vectors, exhaustive adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later runtime selection.

#### Scenario: Replay shared application composition vectors

- **WHEN** focused Rust tests replay committed direct/CAN activation, cache hit/miss/corruption, schema paging, current/default reads, control/shutdown/configuration exclusion, write/acknowledgement/readback, Tune lease, telemetry/lighting confirmation, flywheel, failure, reconnect, and reset vectors
- **THEN** snapshots, scheduler facts, mutation tokens, configuration operations, cache effects, confirmations, failures, diagnostics, and effect order SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise the complete composition safety matrix

- **WHEN** tests cover every supported/read-only/unsupported identity, route, generation, configuration state, mutation state, scheduler completion, response family, stale/mismatched input, malformed/maximum payload, cache outcome, nested effect bound, counter rollover, cancellation, reconnect, and reset
- **THEN** the composition SHALL remain bounded, deterministic, panic-free for public input, fail closed before unsafe work, release every terminated lease, and publish no stale or private data

#### Scenario: Prove safe dependencies and production isolation

- **WHEN** dependency, copied-logic, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the coordinator SHALL directly use only named safe Rust owners/codecs, neither host SHALL select or package it, and no new FFI/JNI, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the composition is ready for its implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native application owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, controls, mutation exclusion, configuration schema/current/default/write/readback, shutdown, telemetry, BMS, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust device mutation coordinator candidate evidence

The Rust device mutation coordinator extraction SHALL be backed by shared source-derived mutation vectors, exhaustive and adversarial safety behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later connection-coordinator composition.

#### Scenario: Replay shared mutation lifecycle vectors

- **WHEN** focused Rust tests run committed source-derived package, flywheel, lighting, fatal-clear, configuration-lease, Tune, shutdown, confirmation, failure, cancellation, reconnect, and reset vectors
- **THEN** authorization, request facts, token identity, snapshots, pending state, confirmation order, failures, and stable diagnostics SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise the complete safety matrix

- **WHEN** tests cover every supported/read-only/unsupported identity, Refloat profile, route, ride/package/footpad/fault state, idempotent value, competing mutation, flywheel transition, lighting response family, scheduler completion, stale/mismatched event, maximum bound, reset, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, fail closed before emitting unsafe work, and publish no stale or private data

#### Scenario: Prove direct safe composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the named safe Rust components, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native mutation owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, controls, mutation exclusion, configuration safety, shutdown, telemetry, BMS, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Rust connection application coordinator candidate evidence

The Rust connection application coordinator extraction SHALL be backed by shared source-derived lifecycle vectors, exhaustive and adversarial behavior, dependency and production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior before later unified-runtime selection.

#### Scenario: Replay shared connection lifecycle vectors

- **WHEN** focused Rust tests run committed source-derived direct, gateway, explicit-selection, reconnect, standalone-BMS, telemetry, attached-BMS, timeout, failure, activation, and reset vectors
- **THEN** generations, request bytes/facts, scheduler/deadline order, child timers, selected facts, states, observations, samples, failures, hints, and diagnostics SHALL match the selected Swift and Kotlin behavior

#### Scenario: Exercise finite bounds and adversarial order

- **WHEN** tests cover every route and response family, maximum CAN/queue/telemetry/BMS collections, all scheduler completions, malformed payloads, activation order, child timer replacement, stale and mismatched input, nested-effect limits, reset, repeated transitions, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public input, read-only during discovery/telemetry, and free of stale or private-data publication

#### Scenario: Prove direct composition and production isolation

- **WHEN** dependency, source-selection, bridge-catalogue, symbol, package, callback, executor, persistence, network, and generated-artifact audits run
- **THEN** the candidate SHALL depend only on the named safe Rust components, neither host SHALL select or package it, and no new FFI/JNI, callback, generated binding, opaque runtime boundary, or host executor SHALL exist

#### Scenario: Pass complete software verification

- **WHEN** the extraction is ready for implementation commit
- **THEN** focused tests and audits, warnings-denied workspace linting, `just verify`, and strict OpenSpec validation SHALL pass with native connection orchestration owners still selected

#### Scenario: Preserve all-host physical behavior

- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, direct/CAN discovery, BMS polling, standalone behavior, compatibility, controls, shutdown, telemetry, sessions, background, files, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification

### Requirement: Refloat 1.3.0 selectable telemetry evidence

The repository SHALL prove exact Refloat 1.3.0 compatibility, complete bounded command-33 behavior, configuration identity, retained control behavior, older-release isolation, and shared production-host composition using immutable source-derived fixtures, focused safe-component tests, canonical conformance, root audits, and the complete software and physical gates. Evidence MUST cover every documented non-reserved command-33 mask field in float32 form, echoed request identity, exact variable response extents, direct/CAN routes, fast/slow merge, generation replacement, persistence/export, limited alert semantics, configuration read/write/readback, and malformed/adversarial inputs without requesting unstable command 35.

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

#### Scenario: Pass all-host gates

- **WHEN** the change is ready for completion
- **THEN** focused tests, warnings-denied linting, dependency/source-layout audits, `just verify`, strict OpenSpec validation, and the Android/iOS/macOS ESP32 physical checkpoint pass with production using the shared Rust runtime and no superseded host decoder added

### Requirement: Shared server migration evidence

The shared server change SHALL pass focused Rust and host-boundary tests, canonical mock/macOS/iOS/Android software conformance, clean native packaging, dependency/deletion audits, and `just verify`. Canonical assertions SHALL remain in `conformance` and cover all existing methods and routes. They SHALL cover authentication, malformed and oversized input, response ordering, multiple and slow clients, cancellation, lifecycle recovery, and archive cleanup. Component tests SHALL cover hostile handles, envelopes, generation races, and shutdown bounds.

#### Scenario: Exercise each selected host

- **WHEN** canonical software scenarios run after each host migration
- **THEN** they exercise that host's selected shared server and fail on missing prerequisites or contract drift

#### Scenario: Verify platform packaging

- **WHEN** clean Apple host/device/simulator and Android host/four-ABI Debug/Release builds run
- **THEN** the required shared server symbols and dependencies are present within the existing binding artifacts without obsolete server libraries

#### Scenario: Verify lifecycle and contention

- **WHEN** clients reconnect, consume slowly, cancel reads, or race listener stop with host completion during recording
- **THEN** the suite verifies bounded work, monotonic state, exactly-once terminal results where connected, and uninterrupted permitted persistence

### Requirement: Explicit shared server physical checkpoint

The change SHALL include a separate Android, iOS, and macOS physical checkpoint with the flashed ESP32 fixture. It SHALL exercise the selected production path for connection, telemetry, saved recording, reconnect, and frontend/lifecycle recovery. Physical availability MUST NOT weaken the software gate. Missing hardware or deferred coverage SHALL remain explicitly incomplete unless the user authorizes a narrower checkpoint.

#### Scenario: Run the physical checkpoint

- **WHEN** each physical host connects through the fixture and its frontend is recreated during recording
- **THEN** the canonical runner observes continued durable progress and current authenticated state through the shared server

#### Scenario: Hardware is unavailable

- **WHEN** a required device or fixture is unavailable
- **THEN** software verification completes independently and the missing physical result is recorded without claiming completion

### Requirement: Complete shared service acceptance

The shared service change SHALL pass direct-host and native canonical conformance, focused storage/ABI/lifecycle fault tests, warnings-denied checks, clean Apple host/simulator/device and Android host/four-ABI packaging, strict OpenSpec validation, deletion audits and the complete `just verify` gate. Assertions and protocol simulation SHALL remain in conformance; target adapters SHALL contain only launch and OS-capability integration.

#### Scenario: Direct host exercises product behavior

- **WHEN** canonical product, archive, recording and reconnect scenarios run through the direct Rust test host
- **THEN** they use the same selected service as native production without test-host product policy, SQL or archive implementations

#### Scenario: Fault and saturation coverage

- **WHEN** storage faults occur between session commit and catalog publication, or cancellation races with saturated queues and worker shutdown
- **THEN** the suite verifies durable recovery, bounded resources, monotonic publication and no duplicate accepted mutation

#### Scenario: Delete selected native ownership

- **WHEN** each host migration completes
- **THEN** its software/package checks pass and audits reject remaining portable stores, policy, domain dispatch or snapshot reconstruction in native production

### Requirement: Shared service physical checkpoint

The change SHALL pass explicit Android, iOS and macOS physical conformance with the ESP32 fixture through the selected shared service. It SHALL cover saved/guest connection, telemetry, recording, reconnect, permitted background location, frontend/lifecycle recovery and applicable native file presentation. Unavailable hardware or coverage SHALL remain incomplete unless the user explicitly authorizes a narrower checkpoint; physical availability MUST NOT weaken the software gate.

#### Scenario: Exercise production OS adapters

- **WHEN** the flashed fixture drives each native host and mobile lifecycle transitions occur during recording
- **THEN** shared policy preserves current authenticated state and durable progress while real OS adapters execute Bluetooth, location and background work

#### Scenario: Hardware unavailable

- **WHEN** a required phone, platform or fixture cannot run its checkpoint
- **THEN** its result remains explicitly incomplete while independent software verification continues
