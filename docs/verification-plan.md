# Build, run, and milestone verification plan

Status: required verification contract for the backend reset roadmap.

This document defines how a developer or CI worker proves that a roadmap
milestone is functional. A milestone is not complete because its files exist or
its main target compiles. It is complete when its canonical verification command
passes and produces the required behavioral evidence.

## Verification principles

- Every roadmap milestone adds or extends an executable `just verify-rN` target.
- Milestone verification is cumulative: `verify-r5` includes the relevant gates
  from R0-R4 before running R5 checks.
- Commands return zero only when all required checks pass. Skipped required
  platforms or scenarios are failures, not successes.
- Fake-backed tests are deterministic, use a fake clock, and require no radio or
  physical board.
- Platform builds, platform fake integration, and real-hardware acceptance are
  separate gates. Passing one never implies another.
- Hardware tests are opt-in, read-only by default, and require a named,
  versioned device profile.
- Test-only Bluetooth control and fake selection must be impossible in release
  builds.
- CI-readable results use JUnit XML where practical. Each verification run also
  writes a small JSON manifest containing commit, tool versions, platform,
  scenario, duration, and outcome.

Generated local evidence belongs under `artifacts/verification/` and is ignored
by Git. Stable summaries and links to CI or hardware runs are recorded in
[`reset-roadmap.md`](reset-roadmap.md).

## Platform requirements

The first iOS, Android, and macOS slice can be developed and software-tested from
one Apple-silicon macOS machine. No Linux or Windows host is required for R0-R8.

What the macOS host can run:

- all schema, generator, web, mock, protocol, and application-service tests;
- Android JVM tests, Android builds, and Android fake-host tests in an emulator;
- Swift package tests, iOS builds, and iOS fake-host tests in a simulator;
- macOS builds, fake-host tests, and real CoreBluetooth tests;
- the shared Node black-box conformance runner against every locally launched
  test host.

What requires physical hardware:

- real iOS CoreBluetooth acceptance requires a signed app on a physical iPhone
  or iPad; the iOS simulator is fake-adapter testing only;
- real Android GATT acceptance requires a physical BLE-capable Android device;
  the emulator is fake-adapter testing only;
- macOS hardware acceptance requires a BLE-capable Mac;
- all hardware acceptance requires a powered, safely supported VESC/Refloat board
  matching a declared test profile.

Linux and Windows become required only when their native backends are started.
Pure contract, fixture, and application tests should remain portable, but BlueZ
integration needs a Linux host with D-Bus/BlueZ and WinRT integration needs a
Windows host with the Windows SDK. Those platforms are outside the first
vertical slice.

### Release qualification matrix

| Area               | Required first profile                                                                       |
| ------------------ | -------------------------------------------------------------------------------------------- |
| iOS                | Physical devices on iOS 17, iOS 18, and latest iOS 26                                        |
| Android            | Samsung Galaxy S9 on Android 10/API 29 plus one current Android destination                  |
| macOS              | Apple-silicon BLE-capable Mac on macOS 14 or newer                                           |
| Controllers/boards | Thor300, Thor301, Floatwheel XRV, Floatwheel PintV                                           |
| Firmware           | VESC 6.06 with Refloat 1.2                                                                   |
| Transport/topology | direct supported BLE plus applicable VESC Express forwarding to the primary motor controller |
| BMS                | VESC VBMS only                                                                               |

The intended compatibility floor is VESC 6.02. Versions 6.02-6.05 and
representative older Refloat releases receive a separate post-baseline fixture
and hardware qualification pass. Float package firmware is a negative
unsupported-firmware test, not a compatibility target.

## Development dependencies

### Required on the primary macOS machine

| Dependency              | Purpose                                                                   | Current/reference version                                  |
| ----------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------- |
| macOS on Apple silicon  | Primary three-platform development host                                   | macOS 15.7.7, arm64                                        |
| Full Xcode installation | Swift, SwiftPM, iOS/macOS SDKs, signing, simulator, `xcodebuild`, `xcrun` | Xcode 26.3 / Swift 6.2.4                                   |
| Node.js and npm         | Svelte UI, generators, mock backend, conformance runner                   | Node 22.12.0 / npm 10.9.0                                  |
| Java Development Kit    | Gradle, Kotlin JVM tests, Android builds                                  | JDK 21 reference; enforce supported range in `just doctor` |
| Android SDK             | Android compile, packaging, emulator/device tooling                       | Platform/compile SDK 36                                    |
| Android platform tools  | `adb` installation, launch, logs, instrumentation                         | Version managed by installed Android SDK                   |
| Gradle wrapper          | Reproducible Android/Kotlin build                                         | Gradle 8.13, checked in                                    |
| Android Gradle Plugin   | Android build logic                                                       | AGP 8.11.1                                                 |
| Kotlin toolchain        | Android/JVM core and generated models                                     | Kotlin 2.0.21                                              |
| `just`                  | Stable repository command interface                                       | Any version accepted by `just doctor`                      |
| Git                     | Source and evidence revision identification                               | System or Homebrew installation                            |
| GitHub CLI (`gh`)       | Inspect/refresh pinned upstream references, including private Floaty      | Reference maintenance only                                 |

The exact minimum supported Xcode, Node, JDK, Android SDK, and `just` versions
must be pinned or range-checked when `just doctor` is implemented in R1. The
versions above describe the known current machine, not an indefinite support
promise.

### Installed from the repository

- web dependencies from `web/package-lock.json` via `npm ci`;
- SvelteKit, TypeScript, Vite, Vitest, Playwright, Ajv, `ws`, and `tsx`;
- Gradle/Kotlin/Android libraries through the checked-in Gradle wrapper and
  version catalog;
- Swift package dependencies through `Package.resolved` if external Swift
  packages are introduced.

Prefer platform/system frameworks for the native implementation:

- `CoreBluetooth`, `Network`, `WebKit`, and Foundation/Swift concurrency on
  iOS/macOS;
- Android Bluetooth/GATT, lifecycle/service, WebView, and storage APIs on
  Android.

Adding a large embedded server, database, BLE abstraction, or cross-platform UI
runtime requires an explicit decision-log entry. The architecture does not
currently require CocoaPods, React Native, Flutter, Docker, Rust, a cloud
service, or an externally installed database.

### One-time setup downloads

- `npm ci` for locked JavaScript dependencies;
- Gradle wrapper distribution and Maven/Google artifacts;
- Android SDK platform 36, build tools, platform tools, and an emulator image if
  emulator tests are desired;
- an iOS simulator runtime matching the chosen Xcode test destination;
- `npx playwright install chromium` for browser UI tests.

After caches are populated, pure software tests should run without network
services. Runtime app operation is local-only and does not require a cloud
backend.

Refreshing the reference manifest additionally requires authenticated `gh`
access. Ordinary setup, builds, and tests must not require cloning VESC Tool or
accessing Floaty.

### Signing and physical-device prerequisites

- iOS device runs require an Apple development team, signing certificate, and
  provisioning profile accepted by the device. Distribution and long-term
  background testing should use the project's intended Apple Developer account.
- Android device runs require USB or wireless debugging authorization; release
  distribution signing is not needed for debug hardware tests.
- Test phones/tablets must support BLE and run OS versions represented in the
  accepted support matrix.
- The VESC must be secured so read-only telemetry cannot cause motion, with a
  known firmware/profile and adequate power. Hardware tests never send motor,
  firmware, or configuration writes.

### CI hosts

A macOS CI runner is unavoidable for compiling and testing iOS/macOS. Android
and web jobs may run on either macOS or Linux, although using the same macOS host
locally is sufficient initially. Hardware jobs require labelled self-hosted
runners physically paired with the appropriate phone and board; ordinary hosted
CI cannot provide meaningful VESC Bluetooth acceptance.

## Command contract

The following is the target repository command interface. A command marked
“available now” already has a useful implementation. All others are delivered
by the milestone shown; they must not be added as no-op placeholders.

| Command                                            | Purpose                                                                                | Availability            |
| -------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------- |
| `just api-check`                                   | Validate schemas/examples and generated-model drift                                    | Available now           |
| `just api-native-check`                            | Compile and round-trip generated Swift/Kotlin models                                   | Available now           |
| `just api-conformance`                             | Run current system scenarios against the development mock                              | Available now           |
| `just doctor`                                      | Report required SDKs, simulators/emulators, tools, and optional hardware prerequisites | R1                      |
| `just dependency-check`                            | Enforce/report approved direct native and frontend dependencies                        | R1                      |
| `just build-web`                                   | Produce the static shared UI bundle                                                    | R1                      |
| `just build-android`                               | Compile the Android debug host and all JVM modules                                     | R1                      |
| `just build-ios`                                   | Compile the iOS host for the configured simulator                                      | R1                      |
| `just build-macos`                                 | Compile the macOS host and shared Swift package                                        | R1                      |
| `just ios-device-run <device-id>`                  | Build, sign, install, and launch the current bundle on physical iOS                     | Available now           |
| `just android-device-run [serial]`                 | Build, install, and launch the current bundle on physical Android                       | Available now           |
| `just run-mock`                                    | Run the web UI with the deterministic development backend                              | R2                      |
| `just run-android-fake`                            | Install/launch Android with a scripted fake adapter                                    | R2                      |
| `just run-ios-fake`                                | Build/launch iOS simulator with a scripted fake adapter                                | R2                      |
| `just run-macos-fake`                              | Launch macOS with a scripted fake adapter                                              | R2                      |
| `just test-contract`                               | Run schema, model, client, and contract tests                                          | R3                      |
| `just test-product`                                | Run Device/Sessions/Live/Review/Diagnosis browser journeys                             | R3                      |
| `just test-profiles`                               | Run Board Profile schema, precedence, derived-value, and snapshot tests                | R3/R5                   |
| `just test-protocol`                               | Run identical fixture assertions in Swift and Kotlin                                   | R4                      |
| `just test-core`                                   | Run Swift/Kotlin application-service tests with fakes                                  | R5                      |
| `just test-storage`                                | Run native log schema, migration, recovery, integrity, and pressure tests              | R5                      |
| `just test-alerts`                                 | Run shared declarative alert semantics against Swift/Kotlin implementations            | R5                      |
| `just benchmark-logging`                           | Measure live, commit, overview, and diagnostic-query budgets                           | R5                      |
| `just conformance-mock`                            | Run all implemented black-box scenarios against the mock                               | R3                      |
| `just conformance-android`                         | Run black-box scenarios against an Android fake test host                              | R5                      |
| `just conformance-ios`                             | Run black-box scenarios against an iOS fake test host                                  | R5                      |
| `just conformance-macos`                           | Run black-box scenarios against a macOS fake test host                                 | R5                      |
| `just test-bluetooth-android`                      | Run Android framework/GATT integration tests                                           | R6                      |
| `just test-bluetooth-apple`                        | Run CoreBluetooth adapter integration tests                                            | R6                      |
| `just test-background`                             | Run lifecycle, WebView teardown, service/restoration, and gap tests                    | R7                      |
| `just hardware-smoke <platform> <profile>`         | Perform one read-only device workflow                                                  | R6/R7                   |
| `just hardware-soak <platform> <profile> <cycles>` | Repeat workflow and check resources/cleanup                                            | R8                      |
| `just verify`                                      | Run the latest completed non-hardware cumulative gate                                  | R1 onward               |
| `just verify-ci`                                   | Run all software gates supported by the CI host matrix                                 | R2 onward               |
| `just verify-r0` … `just verify-r8`                | Run one cumulative roadmap acceptance gate                                             | Corresponding milestone |

`hardware-smoke` and `hardware-soak` must refuse to run if the requested profile
is unknown, the board firmware is outside the profile, or the scenario contains
a write classified as configuration, firmware, or motion-affecting.

## Developer workflows

### Fast edit loop

Use for contract, UI, protocol, or application-service development:

```sh
just doctor
just test-contract
just test-product
just test-profiles
just test-protocol
just test-core
just test-storage
just test-alerts
just conformance-mock
```

Protocol and core checks must run on macOS without launching Xcode, an emulator,
or a Bluetooth radio. Kotlin logic runs as JVM tests; Swift logic runs as Swift
package tests.

### Run the product without hardware

Each runnable fake host loads the same named scenario, such as `happy-path`:

```sh
just run-mock happy-path
just run-android-fake happy-path
just run-ios-fake happy-path
just run-macos-fake happy-path
```

The shared UI should be usable interactively, while the same host can also be
driven headlessly by conformance. Scenario names and behavior come from a shared
manifest under `conformance/scenarios/`; platforms do not maintain private
versions of a scenario.

### Verify the native hosts

```sh
just build-web
just build-android
just build-ios
just build-macos
just conformance-android
just conformance-ios
just conformance-macos
```

Platform conformance launches a test host, waits for a machine-readable readiness
record containing its loopback URL and one-use token, runs the shared Node suite,
then requests shutdown and verifies clean process/app termination. Tokens must
not be printed to ordinary release logs or retained in test artifacts.

### Verify real hardware

```sh
just hardware-smoke ios vesc-6_06-refloat-1_2
just hardware-smoke android vesc-6_06-refloat-1_2
just hardware-smoke macos vesc-6_06-refloat-1_2
just hardware-soak ios vesc-6_06-refloat-1_2 20
```

A profile records expected VESC service/characteristics, permitted read commands,
compatible firmware versions, telemetry expectations, sample interval limits,
and device setup instructions. The runner records opaque or redacted device
identity.

## Milestone verification

### R0: evidence preservation

Canonical command: `just verify-r0`

It must:

- validate pinned upstream repository/commit metadata and protocol traceability;
- validate every fixture metadata file against its schema;
- ensure referenced raw byte files exist and hashes match;
- scan fixtures and metadata for prohibited personal identifiers;
- decode each captured valid fixture with the temporary reference decoder;
- prove every planned legacy removal has an inventory classification;
- prove source-derived expectations comply with
  `reference-implementations.md` provenance and licensing rules;
- write the baseline pass/failure report without converting known failures into
  a successful test result.

Human check: confirm uncertain firmware provenance is explicitly marked
`unknown`, never guessed.

### R1: clean base builds

Canonical command: `just verify-r1`

It must run R0, then:

- reject imports/references to legacy contract modules and the `/ws` endpoint;
- compile the web bundle, Swift core, Kotlin modules, Android host, iOS host, and
  macOS host;
- report every direct runtime dependency and fail on any unapproved native
  dependency or duplicated platform ORM/schema;
- run the smallest generated-model smoke tests;
- verify dependency-boundary rules, including that core modules do not import
  WebView or host lifecycle frameworks.

The output lists an artifact path for each host build. Merely generating an Xcode
project or Gradle configuration is insufficient.

### R2: runnable fake shells

Canonical command: `just verify-r2`

It must run R1, then launch each platform test host and prove:

- the bundled UI health/bootstrap endpoint loads;
- the server binds only to loopback on an ephemeral port;
- missing token, wrong token, and wrong Origin are rejected;
- `system.hello` and `system.getSnapshot` pass over a real WebSocket;
- the host shuts down without a dangling listener;
- release configuration contains no reachable fake/test-control route.

An interactive smoke run through each `run-*-fake` command is also required once
before R2 completion; CI thereafter covers the headless checks.

### R3: contract and product experience foundation

Canonical command: `just verify-r3`

It must run R2, then:

- validate every operation/event example and generated model;
- validate log catalog/sample schemas, bounded query/cursor behavior, storage
  state, committed-sequence semantics, and mock downsampling;
- validate saved-device/session/job identity, immutable device snapshots,
  riding intervals, reversible linked-session groups, catalog filters/sorts, and
  the removal of user-facing `logId`;
- validate Board Profile schemas, S/P and correction bounds, optimistic
  concurrency, discovery/manual/effective precedence, and unknown catalog
  preservation;
- generate deterministic small, medium, eight-hour, and diagnostic-incident log
  corpora with faults, footpad transitions, alert rules, gaps, and high-rate
  signals;
- run client correlation, timeout, reconnect, and unknown-event tests;
- run the full vertical-slice scenarios against the development mock;
- build the product/diagnostic UI and exercise its happy and error paths in a browser
  test;
- run first-use scan/connect, returning saved-device connect, automatic logging,
  Live, disconnect, Sessions filtering, Review, and Diagnostic zoom browser
  journeys;
- verify mock transitions from 1 Hz idle to 10 Hz riding, preserve a 15-second
  post-ride tail, and return to idle with every mode/event persisted while the
  Refloat-state probe runs at 2 Hz;
- verify sampling, reconnection, session-linking, alert-output, and raw-capture
  settings are clamped and return requested/effective values;
- verify session linking is disabled by default, same-device-only, reversible,
  and does not mutate member databases;
- verify confirmed idempotent deletion plus lossless archive and CSV export;
- verify profile setup/edit/reset UX, ambiguous CAN selection, missing-data
  degradation, visual descriptor round trip, and immutable session snapshots;
- fail if a required backend capability lacks a conformance scenario.

### R4: protocol parity

Canonical command: `just verify-r4`

It must run R3, then:

- feed the same raw fixture/chunk manifest to Swift and Kotlin;
- compare normalized JSON output, not language-specific debug strings;
- cover arbitrary splits, combined frames, corrupt CRC, invalid lengths, noise,
  truncation, firmware information, telemetry, and Refloat data;
- run a deterministic chunk-boundary property/fuzz suite with its seed recorded;
- enforce parser buffer and maximum-frame bounds.

### R5: fake-backed application behavior

Canonical command: `just verify-r5`

It must run R4, then:

- run Swift and Kotlin service tests with fake clock/Bluetooth/storage;
- run real SQLite catalog/per-session tests for transactional batches, WAL recovery,
  migrations, integrity, storage pressure, and concurrent active-log reads;
- inject free-space boundaries around 1 GiB, 500 MiB, and 250 MiB and verify
  warning/escalation/clean-stop behavior without automatic deletion;
- populate thousands of saved devices/sessions and verify indexed cursor
  filtering/sorting by device, date, riding/total duration, distance, state, and
  integrity within the catalog performance budget;
- run identical alert threshold/edge/debounce/hysteresis/cooldown fixtures in
  Swift and Kotlin and prove alert evaluation sees every normalized sample before
  UI coalescing;
- cover duty, high/low voltage, motor/controller temperature, individual/combined
  footpad state, and connection-loss rules, including acknowledgement versus
  disabling;
- run identical battery estimation, non-destructive voltage/speed correction,
  VESC Express/primary-controller CAN, and VBMS
  precedence, and profile-revision fixtures in Swift and Kotlin;
- prove raw voltage remains unchanged through profile edits, frontend correction,
  alerts, export, and review queries;
- benchmark decode-to-alert, decode-to-UI, commit latency, eight-hour overview,
  pan/zoom queries, and full-resolution diagnostic windows against the budgets in
  `durable-logging.md`;
- run all shared black-box scenarios against Android, iOS, and macOS test hosts;
- compare stable errors, state ordering, terminal-event counts, snapshots, and
  effective rate limits, alert events, committed sequences, overview buckets,
  event indexes, and diagnostic windows;
- assert queues, retry counts, timers, and reconnect backoff remain bounded;
- verify unexpected disconnect retries only for the configured active-session
  window and explicit disconnect/force-quit never reconnects;
- destroy and recreate the entire fake frontend while native logging continues,
  then reconcile committed history and live samples by sequence;
- prove WebSocket reconnection reconstructs state from a snapshot.

No R5 test may depend on real elapsed sleeps.

### R6: native Bluetooth

Canonical software command: `just verify-r6`

It must run R5 plus adapter/framework integration tests for permissions, radio
changes, service discovery, notification setup, ordered writes, callback races,
and error mapping.

Hardware gate:

```sh
just hardware-smoke ios <profile>
just hardware-smoke android <profile>
just hardware-smoke macos <profile>
```

Each run must discover the board, connect, verify GATT, read firmware, receive a
minimum valid sample count, automatically create a durable idle-rate session,
disconnect, find that session through the catalog API, and confirm the OS no
longer reports an active app connection. R6 is not complete until all three
named hardware runs are recorded.

Run the profile across the declared Thor300, Thor301, XRV, and PintV inventory
before release qualification. R6 milestone completion may use a documented
representative subset only if the remaining board runs are explicitly assigned
to R8; it may not imply compatibility with an untested board.

### R7: logging and disconnect

Canonical software command: `just verify-r7`

It must run R6 software gates, then:

- verify atomic job creation, append, flush, close, and recovery on platform
  SQLite implementations;
- inject storage-full, append, and flush failures;
- destroy/recreate the WebView while logging and recover through a snapshot plus
  paginated sample query;
- check ordered cleanup and exactly one terminal disconnect event;
- inject missing OS disconnect callbacks and late callbacks after forced cleanup;
- inspect the completed log for a valid header, monotonic sequence/timestamps,
  declared gaps, and a non-truncated terminal record.
- verify iOS locked-device logging/restoration and honest suspension gaps;
- verify iOS 17, 18, and 26 separately; on iOS 26 compare active logging with the
  logging Live Activity enabled and disabled, then keep the mechanism only when
  it improves or preserves the required CoreBluetooth behavior;
- verify iOS requests no location permission and Android's API 29-30 location
  permission is scan-only, version-capped, and produces no location data;
- verify the mobile WebView is released by 30 seconds after backgrounding and
  immediately under injected memory pressure;
- verify Android `connectedDevice` foreground-service ownership and notification
  stop behavior;
- verify macOS window closure, sleep/wake, visible background control, and
  explicit-quit flush behavior.

The hardware smoke run now logs for at least 60 seconds with the mobile screen
off/app backgrounded or macOS window closed before stop/flush/disconnect. An
extended ride-duration soak threshold is selected before R7 completion.

### R8: repeated hardware acceptance

Canonical command:

```sh
just verify-r8 <profile>
```

Because R8 requires physical devices, it is never part of ordinary `just verify`
or unlabelled CI. The command orchestrates or verifies recorded results for:

```sh
just hardware-soak ios <profile> 20
just hardware-soak android <profile> 20
just hardware-soak macos <profile> 20
```

Every cycle performs discover/connect/firmware/stream/log/flush/disconnect.
The cycle set includes foreground/background transitions and full WebView
destruction/recreation while the native job remains active.
At least one declared manual ride profile per platform verifies idle/riding/
post-ride cadence transitions, review rollups, event markers, and diagnostic
drill-down without introducing automated motion-affecting commands.
Hardware evidence also compares read-only CAN/BMS discovery with the declared
fixture and verifies manual profile settings never generate configuration
writes.
Failure artifacts include the cycle, last state, stable error, sanitized native
diagnostics, log integrity result, and resource counters. The run fails for any:

- leaked OS connection or open log handle;
- duplicate terminal event or sample sequence;
- corrupt/truncated log tail;
- active command, timer, subscription, or queued write after disconnect;
- unbounded process memory, file descriptor, callback, or thread growth against
  the accepted platform threshold.

## CI matrix

Required software jobs as infrastructure becomes available:

| Job              | Host                         | Required gates                                     |
| ---------------- | ---------------------------- | -------------------------------------------------- |
| Contract and web | Linux or macOS               | API drift, client/UI tests, mock conformance       |
| Kotlin core      | Linux or macOS               | Generated models, protocol fixtures, service tests |
| Android          | macOS or Linux with emulator | Debug/release build and fake-host conformance      |
| Swift core       | macOS                        | Generated models, protocol fixtures, service tests |
| iOS              | macOS with simulator         | Debug/release build and fake-host conformance      |
| macOS            | macOS                        | Debug/release build and fake-host conformance      |
| Hardware         | Labelled self-hosted macOS   | Explicit profile/platform acceptance only          |

Pull requests require every available non-hardware job. Hardware runs can be
scheduled or manually dispatched, but R6-R8 completion requires links to their
recorded results.

## Evidence manifest

Every native fake or hardware run emits a JSON record with at least:

```json
{
  "schemaVersion": 1,
  "milestone": "R7",
  "scenario": "connect-log-disconnect",
  "platform": "ios",
  "hostOsVersion": "recorded-at-runtime",
  "appVersion": "recorded-at-runtime",
  "deviceProfile": "refloat-6_6",
  "firmwareVersion": "recorded-at-runtime",
  "startedAt": "UTC timestamp",
  "durationMs": 60000,
  "sampleCount": 0,
  "droppedSampleCount": 0,
  "logIntegrity": "passed",
  "disconnectCleanup": "passed",
  "outcome": "passed"
}
```

Counts shown above are placeholders populated by the runner. Evidence must never
contain a BLE address, session token, or full unredacted telemetry log.

## Current verification baseline

As of 2026-07-27:

- `just api-check` passes schema/example validation and generated model drift;
- `just api-native-check` compiles and round-trip tests Swift/Kotlin v1 models;
- `just api-conformance` passes the current mock `system.hello` and snapshot
  scenarios;
- the web type check, focused tests, and production build have passed;
- native reset modules, platform v1 test hosts, protocol fixtures, and hardware
  runners do not yet exist.

This baseline must not be described as R0 or R1 completion.
