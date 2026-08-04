# Backend Conformance Specification

## Purpose

Define the single canonical software and physical acceptance suite required of every supported backend.

## Requirements

### Requirement: One canonical suite

The repository SHALL contain one top-level `/conformance` TypeScript black-box suite that imports the hand-written frontend contract and is not copied into Swift or Kotlin implementations.

#### Scenario: Common execution

- **WHEN** conformance runs against mock, Android, iOS, or macOS
- **THEN** the same scenario and assertion implementations are used

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

The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Hardware scenarios SHALL exercise production native Bluetooth, protocol, application-service, and storage paths.

#### Scenario: Fast software gate

- **WHEN** `just verify` runs without physical hardware
- **THEN** the frontend, mock, native code, and deterministic software conformance are verified

#### Scenario: Physical BLE gate

- **WHEN** a hardware conformance command runs with a configured ESP32 fixture
- **THEN** the same relevant contract scenarios drive the backend through its real OS Bluetooth stack

#### Scenario: Production path

- **WHEN** physical conformance launches a Debug application with a test-only endpoint seam
- **THEN** Bluetooth, VESC protocol, application-service, session, and SQLite behavior still use the production native implementations

### Requirement: Deterministic state synchronization

The shared conformance client SHALL wait for bounded predicates over monotonic authoritative state revisions and SHALL be able to bootstrap a replacement WebSocket after mobile frontend recreation without restarting the native backend or fixture runner.

#### Scenario: Asynchronous connection transition

- **WHEN** a connection request is accepted before the backend reaches `ready`
- **THEN** conformance waits for a newer state satisfying the expected transition rather than using an unbounded sleep

#### Scenario: iOS foreground recovery

- **WHEN** iOS stops its loopback frontend server while backgrounded and starts it again in the foreground
- **THEN** conformance obtains current bootstrap credentials, opens a replacement WebSocket, and verifies the preserved native state

### Requirement: Conformance-owned VESC simulation

`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed BLE-transport and one-or-more CAN-controller identity/topology/Refloat configurations, telemetry-layout fields and phases, fragmentation, cadence assertions, disconnect injection, CAN discovery omissions and failures, and bounded failure diagnostics. Exact response encoding SHALL be checked against immutable protocol fixtures. The ESP32 firmware MUST NOT duplicate those behaviors.

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

Canonical conformance SHALL permit only required VESC identity, discovery, and telemetry requests and SHALL reject unknown, motor-control, firmware, and persistent-configuration commands.

#### Scenario: Mutating VESC command

- **WHEN** a backend sends a command capable of motion or persistent mutation
- **THEN** conformance fails the scenario and supplies no mutating response

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
