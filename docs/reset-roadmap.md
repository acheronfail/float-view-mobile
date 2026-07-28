# Float View backend reset roadmap

Status: implementation in progress through the R0-R5 foundation.

Last updated: 2026-07-28.

This document is the execution plan for replacing the legacy mobile
implementations with a small, testable base shared by iOS, Android, and macOS.
It supersedes the incremental native migration described in the original
`api-v1-proposal.md` migration section. The API design in that document remains
the contract direction.
Accepted platform, firmware, UX, and release boundaries are locked in
[`implementation-scope-lock.md`](implementation-scope-lock.md).

The first delivery target is deliberately narrow:

> Each platform launches the shared product UI, discovers and connects to a
> supported VESC over native Bluetooth, reads firmware information, records
> periodic read-only telemetry to a native log, stops logging, and disconnects
> cleanly.

See [`vertical-slice-v1.md`](vertical-slice-v1.md) for the normative behavior and
acceptance scenarios for that target.
[`verification-plan.md`](verification-plan.md) defines the commands and evidence
that make every exit gate executable.
[`reference-implementations.md`](reference-implementations.md) defines the
canonical VESC Tool oracle, authorized Floaty reuse, provenance requirements,
and retirement of the original implementation.
[`durable-logging.md`](durable-logging.md) defines the application's primary
capability: native durable logs that outlive the frontend and behave honestly
under each platform's background constraints.
[`product-experience.md`](product-experience.md) defines the Device/Sessions
navigation, automatic recording, adaptive sampling, review, diagnosis, and
user-facing data model.
[`board-profiles.md`](board-profiles.md) defines local per-board settings,
read-only discovery/manual precedence, corrections, visual identity, and
immutable session snapshots.

## How to use this roadmap

- Work in milestone order. A milestone is complete only when every exit gate is
  satisfied on every platform named by that milestone.
- Run the milestone's cumulative `just verify-rN` command. A prose review or
  successful compile alone cannot complete a milestone.
- Update the status table and evidence links in the same change that completes a
  milestone.
- Record architecture choices in the decision log below before depending on
  them in multiple modules.
- Do not add configuration writes, firmware updates, motor control, multi-board
  support, Linux, or Windows to this vertical slice.
- Complete R0-R5 in the first large implementation pass and scaffold, but do not
  claim completion of, the R6 native adapters. R6-R8 require physical hardware.
- Treat native logging and recovery as a foundation, not a UI feature or final
  integration step. No sampling path may depend on WebView lifetime.
- Do not copy legacy files into the new architecture. Small protocol behavior may
  be reimplemented only after it is represented by a fixture or test.
- Resolve protocol behavior using the precedence and licensing rules in
  `reference-implementations.md`; the original implementation is never the
  protocol authority.

## Status

| Milestone | Outcome                                                          | Status      |
| --------- | ---------------------------------------------------------------- | ----------- |
| R0        | Preserve evidence and define the deletion boundary               | Complete    |
| R1        | Remove legacy product code and establish the new module skeleton | Complete    |
| R2        | Run iOS, Android, and macOS shells against fake backends         | Blocked     |
| R3        | Freeze the vertical-slice API and product experience foundation  | In progress |
| R4        | Pass shared protocol fixtures in Swift and Kotlin                | In progress |
| R5        | Pass durable logging/service tests and fake conformance          | In progress |
| R6        | Connect and stream through real native Bluetooth adapters        | In progress |
| R7        | Prove platform background durability and disconnect cleanup      | Not started |
| R8        | Pass repeated hardware acceptance on all three platforms         | Not started |

Allowed status values are `Not started`, `In progress`, `Blocked`, and
`Complete`. When blocked, add the reason and owner immediately below the table.

R2 blocker (remaining fake/conformance verification): the iOS host passes a
physical iPhone 12/iOS 18.6.1 shell/bootstrap smoke test, the Android host passes
the equivalent test on a Galaxy S9+/Android 10/API 29, and a usable iOS
simulator runtime is installed. The complete scripted-fake shell/conformance
gate has not yet run across all three platforms. macOS passes the system suite.
Details and evidence are in
[`implementation-notes.md`](implementation-notes.md).

## Target repository shape

The exact build-system names may vary, but dependency direction must match this
shape:

```text
api/                         versioned schemas, examples, generated models
conformance/                 transport-level black-box behavior suite
fixtures/
  vesc/                      framing and standard VESC response fixtures
  refloat/                   Refloat response and telemetry fixtures
logging/
  sql/                       shared SQLite schema, migrations, and query templates
web/                         shared Svelte UI and development mock
apple/
  FloatViewCore/
    Sources/
      API/                   generated models and JSON-RPC translation
      Application/           connection, telemetry, logging state machines
      Bluetooth/             port plus shared CoreBluetooth implementation
      Logging/               native persistence implementation
      Protocol/              pure VESC/Refloat codec
      TestSupport/           scripted fakes
  iOS/                       thin lifecycle and WKWebView host
  macOS/                     thin lifecycle and WKWebView host
android/
  api-models/                generated API models
  core/                      application service and ports
  protocol/                  pure VESC/Refloat codec
  logging/                   storage and background job implementation
  bluetooth/                 Android GATT adapter
  app/                       thin lifecycle and WebView host
```

The Apple core is a Swift package shared by the iOS and macOS hosts. Android
uses separate Gradle modules so protocol and application behavior can be tested
on the JVM without an emulator. Host targets depend on the core; core code never
depends on a WebView, `Activity`, `View`, `UIApplication`, or `NSApplication`.

## Minimum dependency and backend policy

Use OS frameworks and language standard libraries first. The desired native
runtime dependency set is:

- Apple: Foundation, CoreBluetooth, Network, WebKit, SQLite3, and only the
  lifecycle/notification/audio frameworks required by native behavior;
- Android: platform Bluetooth/GATT, foreground service/lifecycle, WebView,
  SQLite, notification/audio APIs, Kotlin, and generated serialization support;
- shared frontend: Svelte/TypeScript plus narrowly justified browser libraries.

Prefer shared specifications and assets over duplicated libraries:

- generated API models from `api/`;
- identical protocol fixtures;
- shared SQLite schema/migrations/query templates;
- shared alert-rule schemas and black-box behavior;
- shared TypeScript visualization, review, and diagnosis logic.

The shared frontend uses custom Svelte/TypeScript/SVG visualization initially.
Do not add a charting framework or analytics SDK. The loopback HTTP/WebSocket
architecture is mandatory across iOS, Android, and macOS unless an accepted
decision records a measured platform limitation. A small audited server
dependency is allowed after the normal dependency review when it avoids
maintaining a fragile protocol implementation.

Do not add a native ORM, cross-platform UI runtime, generic BLE wrapper, embedded
database server, analytics SDK, or large HTTP framework merely for convenience.
A new native runtime dependency requires:

- the platform capability that cannot reasonably be implemented with an OS API;
- size, memory, background, security, maintenance, and upgrade impact;
- license and supply-chain review;
- equivalent behavior and testability on every affected platform;
- an accepted decision-log entry.

Backend code exists only for native Bluetooth/protocol I/O, durable local data
work, background/lifecycle ownership, native alert output, local transport, and
security boundaries. Product presentation, ride review, charts, maps, alert
authoring, trend interpretation, and diagnostic workflows belong in the shared
frontend. Generic SQL aggregation stays native because transferring whole rides
to a mobile WebView would violate the resource and performance requirements.

## Legacy removal boundary

The old product implementation is frozen during R0 and removed in R1 after its
useful behavior is captured. Git history plus an immutable baseline ref is the
archive; the repository will not keep a second shipping `legacy/` source tree.

Remove:

- handwritten Android and iOS loopback/WebSocket servers;
- legacy native command dispatch and generated legacy command/message files;
- legacy Android and Swift BLE classes whose callbacks broadcast directly to the
  server;
- duplicated Swift/Kotlin VESC codec implementations after their behavior has
  been captured as fixtures;
- the Zod legacy wire contract, legacy generator, `/ws` mock behavior, and UI
  code that consumes that contract;
- placeholder native and web tests that assert no product behavior.

Retain:

- project metadata, signing configuration, bundle/application identifiers,
  entitlements, permissions, icons, launch assets, and build wrappers;
- the `web/build` bundling mechanism where it remains useful;
- `api/`, `conformance/`, generated API v1 models, the typed v1 client, and the
  v1 development mock;
- sanitized VESC service/characteristic identifiers and protocol facts captured
  in R0 fixtures or documentation.

Before deleting a file, classify it in an R0 inventory as `retain`, `replace`, or
`extract fixture`. Do not delete the only known example of a protocol response
until the corresponding fixture has a passing decoder expectation.

## Milestones

### R0: preserve evidence and define the deletion boundary

Canonical verification: `just verify-r0`.

Deliverables:

- an inventory of tracked legacy files with `retain`, `replace`, or
  `extract fixture`;
- sanitized raw fixtures for framing, firmware response, real-time values, and
  at least one Refloat response currently understood by the project;
- metadata for every fixture: source, firmware family/version if known, chunk
  boundaries, expected normalized result, and whether CRC is valid;
- a written list of known service/characteristic UUIDs and required GATT setup;
- pinned VESC Tool, Floaty, and prior Float View revisions plus a protocol and UX
  traceability matrix following `reference-implementations.md`;
- a durable owner permission/provenance record for any Floaty or prior Float
  View code to be reused;
- a clean baseline report of commands that pass and known pre-existing failures.

Exit gate:

- fixture bytes decode independently of Bluetooth in at least one temporary
  reference test;
- no personal BLE identifiers or session tokens are present;
- every first-slice protocol expectation names its provenance and does not depend
  solely on the original implementation;
- every file planned for removal is listed in the inventory.

### R1: remove legacy code and create the module skeleton

Canonical verification: `just verify-r1`.

Deliverables:

- legacy product files removed according to the approved inventory;
- no compiled or active copy of the legacy implementation remains elsewhere in
  the normal source tree;
- the target module/package structure present;
- repository commands for building and testing each module;
- a minimal shared backend interface with injected Bluetooth, clock, log store,
  and event sink ports;
- `AGENTS.md` and repository maps updated to describe only the new active path.

Exit gate:

- Swift core, Android JVM modules, web app, iOS host, Android host, and macOS host
  all compile, even though adapters may still be fake;
- no active import or runtime reference to the legacy contract or `/ws` remains.

### R2: runnable shells with scripted fakes

Canonical verification: `just verify-r2`.

Deliverables:

- iOS and macOS WKWebView hosts and an Android WebView host;
- an authenticated loopback server bound only to an ephemeral loopback port;
- bootstrap data containing the v1 WebSocket URL and per-launch token;
- exact-origin validation and bounded message limits;
- scripted fake Bluetooth and log-store implementations selectable only in
  debug/test builds.

Exit gate:

- all three apps launch the same bundled product/diagnostic UI;
- `system.hello` and `system.getSnapshot` pass the black-box suite against every
  host using fakes;
- release builds cannot enable the test-control interface.

### R3: freeze the vertical-slice API and product experience foundation

Canonical verification: `just verify-r3`.

Deliverables:

- schemas, examples, generated models, mock behavior, and conformance scenarios
  for the operations in `vertical-slice-v1.md`;
- migrate the Phase 1 `maxConnections`/ambiguous `devices` snapshot into explicit
  known-device collections plus a nullable singleton active connection;
- the durable catalog/session schema, storage-state and paginated sample-query API,
  recovery rules, and mock downsampling behavior in `durable-logging.md`;
- saved-device, session, riding-interval, device-snapshot, event, rollup, and
  active-job models plus non-destructive linked-session groups from
  `product-experience.md`;
- Board Profile schemas/API/mock for battery topology/chemistry, motor, rails/
  appearance, CAN/BMS discovery and overrides, calibration, and visual identity;
- one central UI API client/store with reconnect, snapshots, request promises,
  event routing, and bounded telemetry state;
- the Device and Sessions tabs, saved-device/scan states, automatic Live entry,
  session filtering, bounded Review shell, and Diagnostic zoom shell.
- configurable sampling/reconnection/session-link settings, exact first-release
  alert types, session deletion, and lossless-archive/CSV export contracts.

Exit gate:

- `api:check` reports no schema or generated-code drift;
- the development mock passes all vertical-slice conformance scenarios;
- unsupported capabilities are explicit rather than represented by missing UI
  behavior.
- connecting through the mock automatically creates a session; idle/riding/
  post-ride transitions and session-list/review/diagnostic queries pass browser
  tests.
- profile editing, automatic/manual precedence, warnings, and session snapshot
  behavior pass shared UI and contract tests.
- automatic session linking is disabled by default and, when enabled, links only
  eligible same-device sessions without rewriting member databases.

### R4: shared protocol behavior

Canonical verification: `just verify-r4`.

Implement independent, pure codecs in Swift and Kotlin. They accept byte chunks
and emit typed packets without importing Bluetooth or host frameworks.

Required fixture cases:

- short and long VESC frames;
- packets split across arbitrary notification boundaries;
- multiple packets in one notification;
- corrupt CRC, invalid length, noise before a frame, and truncated input;
- firmware information and the telemetry fields used by the first log profile;
- Refloat data needed by the supported initial board profile.
- VESC 6.06 and Refloat 1.2 identity/state/telemetry behavior, with Refloat ride
  state driving the first adaptive profile;
- read-only CAN discovery and smart-BMS detection responses used by Board
  Profiles, including VESC Express forwarding and VBMS.

Exit gate:

- Swift and Kotlin produce the same normalized output for every fixture;
- selected canonical outputs agree with the pinned VESC Tool and declared
  hardware profiles;
- invalid streams fail or recover with the same stable classification;
- no protocol test uses sleeps, a radio, or platform UI frameworks.

### R5: durable logging, application service, and fake conformance

Canonical verification: `just verify-r5`.

Status evidence (2026-07-28): the cumulative software gate passes with catalog
schema v3, crash-resumable idempotent deletion, persisted/clamped sampling and
reconnection settings loaded at every native startup, reversible same-device
session groups, canonical CSV generation, and a self-contained SQLite archive
containing samples/events plus immutable device/profile/rate/integrity metadata.
The shared UI can configure supported logging/linking settings, review grouped
segments, prepare either export, and confirm deletion. The macOS and mock
black-box suites pass; the Galaxy S9/API 29 passes all six real SQLite
instrumentation cases. R5 remains in progress until the complete black-box suite
runs against Android and iOS hosts and prepared exports have a uniform
user-visible save/share handoff on all three platforms.

Deliverables:

- explicit discovery, connection, telemetry, logging, and disconnect state
  machines in both native cores;
- automatic session creation plus adaptive idle/riding/post-ride sampling with
  persisted requested/effective settings and riding intervals;
- bounded active-session reconnection, catalog-only optional session grouping,
  exact alert semantics, idempotent deletion, and archive/CSV export;
- versioned Board Profile persistence, validation, optimistic concurrency,
  discovered/manual/effective resolution, and shared battery/calibration
  fixtures;
- native SQLite catalog/per-session adapters using system Swift/Android APIs, with WAL,
  bounded transactions, migrations, recovery, integrity checks, and storage
  pressure handling;
- deterministic fake clock, Bluetooth adapter, log store, and event recorder;
- equivalent scripted scenarios exposed through test-only backend control;
- bounded UI telemetry queues and a logging path independent of the WebSocket.

Exit gate:

- the complete vertical-slice black-box suite passes against iOS, Android, and
  macOS test hosts;
- service-level tests cover timeout, cancellation, duplicate requests, radio
  loss, malformed packets, WebSocket/WebView loss, process interruption,
  migration, disk-full, and storage errors;
- logging continues through complete frontend destruction in fake lifecycle
  tests, and the recreated UI catches up from committed sequence data;
- saved-device rename/forget and session filtering preserve immutable historical
  device snapshots;
- profile edits preserve raw values, tag derived values with the effective
  revision, calculate corrections non-destructively, and never rewrite completed
  session snapshots;
- snapshot recovery reconstructs truth after UI reconnection.

### R6: real native Bluetooth

Canonical software verification: `just verify-r6`. Completion additionally
requires a passing `hardware-smoke` run on iOS, Android, and macOS.

Status: in progress. Android 10, an iPhone 12, and macOS have physically passed
discovery, Nordic UART
setup/CCCD confirmation, Express-to-primary CAN discovery, VESC 6.06 and Refloat
1.2.3 identification, modern dynamic telemetry decoding, adaptive durable idle
persistence, SQLite integrity, and explicit disconnect. Android's sanitized
frames pass the shared Swift/Kotlin protocol suite. The formal profile-aware
hardware-smoke validator now passes all three redacted records. Full workflow
orchestration and the remaining platform radio/permission framework gates are
still required before R6 is complete. Shared-core unexpected-disconnect,
out-of-range-style recovery, bounded exponential backoff/window expiry, and
explicit-disconnect cancellation are implemented and deterministically tested.
The remaining physical failure paths now share a checked-in scenario definition,
redacted evidence template/validator, and guided per-platform command. Android
permission denial/restoration is reproducible through an API-level-aware ADB
recipe. Native cores now terminally close an active session when connect
permission becomes denied/restricted instead of retrying indefinitely.
Android 10 has physically passed four of the five guided scenarios after adding
radio broadcasts and a bounded GATT-attempt watchdog: permission denial,
radio-cycle recovery, VESC power-loss recovery, and explicit disconnect. The
same four native scenarios pass on an iPhone 12/iOS 18.6.1 using identifier-free
CoreBluetooth/application-service traces and durable SQLite inspection. The
same scenarios pass on macOS from a signed user-local app bundle. Its 218572 ms
session passed catalog/session integrity with 306 committed observations and
106 adaptively persisted samples. Board Profile get/update is now wired through
the shared UI and both native SQLite stores with bounds validation, optimistic
revision checks, immutable session-start snapshots, and active-session change
events. Swift persistence tests, macOS black-box conformance, and an Android 10
instrumentation run pass. Android 10, iPhone 12/iOS 18.6.1, and macOS have now
physically passed local-only 18S/2P/Hypercore overrides, native persistence,
process restart, reconnect identity reuse, and historical session-link
preservation. Legacy platform-derived saved/history identifiers are atomically
rekeyed without changing the private platform binding or losing profiles.

Deliverables:

- a shared CoreBluetooth adapter for iOS/macOS;
- an Android GATT adapter with version-correct permission behavior;
- service discovery, characteristic validation, notification enablement and CCCD
  confirmation where required, ordered writes, and callback-to-domain error
  mapping;
- read-only firmware and telemetry commands over real devices.

Exit gate:

- each platform discovers a supported fixture board, connects, verifies required
  GATT characteristics, reads firmware information, automatically creates a
  session, and streams/persists decoded samples at adaptive rates;
- supported hardware runs compare automatic controller CAN/smart-BMS discovery
  with the declared fixture profile and prove manual overrides remain local;
- radio-off, permission-denied, out-of-range, and unexpected-disconnect paths
  end in a defined API state.

### R7: platform background durability and deterministic disconnect

Canonical software verification: `just verify-r7`. Completion additionally
requires the 60-second hardware logging scenario on all three platforms.

Status: in progress. The Apple hosts now separate the lifetime native owner from
a disposable frontend lifecycle coordinator. A physical iPhone 12/iOS 18.6.1
run destroyed the loopback listener and WKWebView on background, created a fresh
port/token/WebView on foreground, and reconstructed the same connected durable
session after 60 seconds. The corresponding macOS run closed its window for 30
seconds, kept the native job and Dock process alive, and reopened a fresh
frontend generation showing the same connection; the eventual explicit close
produced an integrity-valid 218572 ms session. Startup recovers catalog rows left
active by process termination as interrupted rather than leaving false active
sessions.

The exported iOS session passed catalog/session integrity with
`completed`/`explicit_disconnect`, 391 committed observations, 143 persisted
samples, 197910 ms elapsed, and a maximum persisted-sample gap of 1561 ms. This
proves idle-rate native logging continued through the tested 60-second
background interval. Android foreground-service ownership, Apple process
restoration, macOS sleep/wake and active-session quit, pressure and locked-file
tests, and paginated sample reconstruction remain R7 blockers.

Deliverables:

- platform ownership appropriate for background work: Android foreground service
  with `connectedDevice`, iOS `bluetooth-central` restoration, and a visible
  macOS background owner with an honest capability model;
- an iOS 26 Live Activity logging experiment and implementation when physical
  testing confirms the required CoreBluetooth behavior; iOS 17/18 remain
  independently qualified;
- host lifecycle teardown/recreation that destroys the WebView while preserving
  the native logger, then recovers through snapshot and sample queries;
- bounded buffering with explicit dropped/error counters;
- the ordered, idempotent disconnect procedure defined in
  `vertical-slice-v1.md`.

Exit gate:

- a 60-second hardware session produces a readable, flushed log;
- screen-off/background mobile sessions and a window-closed macOS session produce
  readable logs or explicit OS-suspension gaps;
- closing, reloading, or destroying the WebView does not stop a logging job while
  the backend remains eligible to run;
- process kill/recovery, locked-device access, disk-full, migration, sleep/wake,
  and platform restoration tests satisfy `durable-logging.md`;
- mobile WebViews are gone within 30 seconds of backgrounding, or immediately
  under memory pressure, while eligible native logging continues;
- stopping and disconnecting leaves no active timers, writes, subscriptions,
  callbacks, or stale `ready` state.

### R8: three-platform hardware acceptance

Canonical verification: `just verify-r8 <profile>`.

Run the same read-only acceptance profile on iOS, Android, and macOS against
declared VESC/Refloat firmware fixtures.

Exit gate:

- all scenarios in `vertical-slice-v1.md` pass;
- 20 connect/log/disconnect cycles pass on each platform without leaked
  connections, duplicate samples, corrupt log tails, or process growth outside
  the agreed bound;
- completed hardware sessions are immediately filterable in Sessions, open
  through bounded Review rollups, and drill into aligned lossless Diagnosis
  windows;
- results record app version, OS/device, board type, firmware, duration, sample
  count, dropped count, and cleanup result.

The precise command behavior, test-host lifecycle, CI matrix, and evidence
manifest are specified in [`verification-plan.md`](verification-plan.md).

## Parallel work and dependency order

After R1, these tracks may progress in parallel:

```text
API schemas + conformance ───────────────┐
Swift protocol + service ────────────────┼─> real CoreBluetooth ─┐
Kotlin protocol + service ───────────────┼─> real Android GATT ──┼─> R8
Shared UI + mock ────────────────────────┘                       │
Durable logging + lifecycle ────────────────────────────────────┘
```

Cross-platform contract changes land before backend-specific implementations.
Real Bluetooth work does not block fake-backed state-machine work. iOS and macOS
hosts share a core and CoreBluetooth adapter from the beginning rather than
being reconciled later. Durable storage begins with the service cores in R3-R5;
R7 hardens platform background behavior rather than introducing logging late.

## Decision log

Record new entries without rewriting older decisions. Use `Proposed`, `Accepted`,
`Superseded`, or `Rejected`.

| ID    | Status     | Decision                                                                                                                                                     |
| ----- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D-001 | Accepted   | Reset the native implementations instead of incrementally refactoring their server/BLE coupling.                                                             |
| D-002 | Accepted   | Preserve useful legacy behavior as sanitized fixtures and documentation, not a checked-in legacy source tree.                                                |
| D-003 | Accepted   | The first vertical slice is read-only: connect, read firmware/telemetry, log, stop, and disconnect.                                                          |
| D-004 | Accepted   | Build iOS, Android, and macOS together; iOS and macOS share a Swift core and CoreBluetooth adapter.                                                          |
| D-005 | Accepted   | Native logging is independent of WebSocket/UI telemetry delivery.                                                                                            |
| D-006 | Superseded | Replaced by D-028: exactly one active board is a permanent product constraint.                                                                               |
| D-007 | Accepted   | Use native SQLite WAL stores: a durable device/session/job catalog plus one isolated database per session; exports remain derived artifacts.                 |
| D-008 | Accepted   | Implement first against VESC 6.06 and Refloat 1.2 on Thor300, Thor301, XRV, and PintV; later qualify VESC 6.02-6.05 and prior Refloat releases.              |
| D-009 | Accepted   | Persist opaque platform-local saved-device identities while copying immutable device snapshots into sessions so history survives rename/forget.              |
| D-010 | Superseded | Replaced by the explicit platform behavior and honest gap semantics in `durable-logging.md` and D-016.                                                       |
| D-011 | Accepted   | Use Apple-silicon macOS as the primary R0-R8 development host; real mobile BLE acceptance uses physical iOS and Android devices.                             |
| D-012 | Accepted   | Treat VESC Tool as canonical, owner-authorized Floaty as supplementary/reusable, and old Float View as temporary historical evidence.                        |
| D-013 | Accepted   | Remove the original product implementation in R1 after R0 extracts fixtures and records an immutable recovery ref.                                           |
| D-014 | Accepted   | Logging is the primary capability; native storage and logging jobs are independent of WebView lifetime.                                                      |
| D-015 | Accepted   | Tear down the frontend while backgrounded and reconstruct it from snapshot plus durable sample queries on foreground.                                        |
| D-016 | Accepted   | Advertise platform-specific background guarantees and record gaps when an OS prevents sampling; never claim false continuity.                                |
| D-017 | Accepted   | Preserve prior `acheronfail/float-view` review/diagnosis behavior as a frontend reference while replacing whole-ride JS loading with bounded native queries. |
| D-018 | Accepted   | Minimize native dependencies and backend responsibilities; use system APIs/shared assets and move non-native product logic to the shared frontend.           |
| D-019 | Accepted   | The start experience has Device and Sessions tabs, with Live under Device and Review/Diagnosis under Sessions.                                               |
| D-020 | Accepted   | Reaching device readiness automatically creates a durable session and begins logging before normal Live state.                                               |
| D-021 | Superseded | Refined by D-031 to distinguish idle ride-state probing from persisted sample cadence.                                                                       |
| D-022 | Accepted   | User-facing history uses `sessionId`; `jobId` is active execution identity and the earlier planned `logId` is retired.                                       |
| D-023 | Accepted   | Store versioned app-local Board Profiles for identity, battery, motor, rails/visuals, CAN/BMS, and calibration without writing VESC configuration.           |
| D-024 | Accepted   | Manual Board Profile values override discovery until returned to automatic; ambiguous CAN discovery requires user selection.                                 |
| D-025 | Accepted   | Snapshot effective Board Profile revisions into sessions and preserve raw observations separately from corrected/estimated values.                           |
| D-026 | Accepted   | Complete R0-R5 and scaffold R6 in the first large pass; R6-R8 completion requires real Bluetooth and physical hardware evidence.                             |
| D-027 | Accepted   | Support iOS 17+, Android API 29+, and macOS 14+; qualify iOS 17, 18, and 26 plus a Galaxy S9 on Android 10.                                                  |
| D-028 | Accepted   | Permit exactly zero or one active board permanently; discoveries, saved devices, and history remain collections.                                             |
| D-029 | Accepted   | Unexpected disconnect uses bounded configurable active-session reconnection (five-minute default); explicit disconnect never reconnects.                     |
| D-030 | Accepted   | Optional same-device session linking is catalog metadata, disabled by default, reversible, and never merges canonical session databases.                     |
| D-031 | Accepted   | Use Refloat ride state, a 2 Hz idle probe/1 Hz idle persistence, 10 Hz riding persistence, and a configurable 15-second post-ride tail.                      |
| D-032 | Accepted   | Preserve raw voltage as canonical; frontend corrections remain reproducible, while native code may apply the same versioned correction only for alerts.      |
| D-033 | Accepted   | Initially support duty, voltage, motor/controller temperature, footpad, and connection-loss alerts; alerts are advisory only.                                |
| D-034 | Accepted   | Board Profiles are optional/partial; start with generic battery data, Hypercore XR, Superflux HS, VESC Express topology, and VBMS only.                      |
| D-035 | Accepted   | Warn below 1 GiB, escalate below 500 MiB, stop cleanly below 250 MiB, never auto-delete, and exclude logs from backup initially.                             |
| D-036 | Accepted   | First release includes idempotent confirmed deletion plus lossless archive/CSV export; imports, cloud sync, and GPS are post-release.                        |
| D-037 | Accepted   | Use authenticated loopback HTTP/WebSocket on every platform; allow a small audited server dependency and require a decision for transport divergence.        |
| D-038 | Accepted   | Tear down mobile WebViews after a 30-second background grace or immediately on memory pressure; native ownership continues independently.                    |
| D-039 | Accepted   | Custom Svelte/SVG owns charts and visuals; ship no analytics SDK initially.                                                                                  |
| D-040 | Accepted   | Use an iOS 26 Live Activity when verified to improve active-session CoreBluetooth behavior; never use background location solely as a keepalive.             |
| D-041 | Proposed   | AGPL-3.0 is the leading project-license candidate, but the final license remains a distribution gate.                                                        |
| D-042 | Accepted   | Distribute the first public release through the Apple App Store and Google Play Store.                                                                       |

## Progress evidence

Add links here as milestones complete:

- R0 inventory: [`../references/legacy-inventory.json`](../references/legacy-inventory.json)
- fixture corpus: [`../fixtures/`](../fixtures/)
- R1 boundaries: `just dependency-check` (7 modules; no legacy runtime references)
- API conformance: `just conformance-mock` (6 scenarios)
- Apple loopback conformance: `just conformance-macos` (6 system/profile/session scenarios)
- Swift tests: `just test-core` (31 deterministic tests)
- Android tests/build: `./gradlew test assembleDebug` (169 tasks; successful)
- protocol parity: `just test-protocol` (4 Swift and 4 Kotlin fixture tests)
- storage/alerts: `just test-storage && just test-alerts`
- Board Profiles: `just test-profiles`; Android 10 also passes
  `./gradlew :logging:connectedDebugAndroidTest`
- Session review foundation: catalog/session schema v2 migrations maintain
  indexed summaries; `just test-sessions` covers keyset filtering/sorting,
  bounded rollups, lossless diagnostic windows, mock conformance, and the real
  macOS loopback backend. `just test-sessions-android-device` passes all four
  SQLite instrumentation tests on the Galaxy S9/Android 10. The physical Android
  UI lists migrated sessions with reconstructed durations, renders a bounded
  45-point ride chart, and reads a 24-sample lossless diagnostic window without
  loading the complete session into the WebView. An iPhone 12/iOS 18.6.1 and the
  installed macOS app render the same shared Review UI and independently return
  lossless midpoint diagnostic windows (the verified macOS ride returned 21
  samples spanning 94–123 seconds).
- physical Board Profiles: Android 10, iPhone 12/iOS 18.6.1, and macOS each
  preserved revision 1 (`18S`, `2P`, `hypercore_xr`) across process restart and
  reconnected to one opaque saved board without rewriting prior session snapshots
- iOS hardware report:
- Android hardware report:
- macOS hardware report:
