# Programmable VESC Bluetooth test device

Status: active implementation plan. T0 and T1 are complete; T2 is next.

This document defines a programmable Bluetooth Low Energy peripheral for
testing Float View's real CoreBluetooth and Android GATT paths without requiring
a powered board for every run. The first implementation target is an ESP32-S3
connected to the development Mac over USB and to one Float View client over BLE.

The test device complements, but never replaces:

- pure Swift/Kotlin protocol fixtures;
- application-service tests with fake Bluetooth, clock, and storage;
- black-box conformance against scripted fake hosts;
- read-only acceptance against declared physical VESC/Refloat hardware.

The emulator is not evidence that a firmware version or physical board is
compatible. Physical hardware and the reference precedence in
[`reference-implementations.md`](reference-implementations.md) remain
authoritative.

## Decision and hardware suitability

An ESP32-S3 is sufficient for the first slice and the intended longer-term
harness. It provides:

- a BLE peripheral/GATT server capable of advertising the VESC UART service;
- writable and notifiable custom characteristics;
- enough CPU and memory for bounded byte queues and transport control;
- a bidirectional USB serial connection for host orchestration;
- programmatic advertising, notification, and connection termination.

The first slice does not require PSRAM, Wi-Fi, an SD card, or additional
electronics. A normal ESP32-S3 development board with a working data-capable USB
connection and usable antenna should suffice. Four MiB of flash is ample for the
planned transport firmware. Boards with a stable USB-UART bridge or native USB
Serial/JTAG are both acceptable.

Milestone T0 verifies the exact board, USB path, and BLE behavior before the
repository depends on a particular development-board layout. The harness must
identify the board by USB VID/PID and, where available, serial number rather than
by an unstable `/dev/cu.*` path.

Use ESP-IDF with NimBLE, pinned to one reviewed release. Do not base the
repository harness on an Arduino sketch or an unpinned IDE installation.
ESP-IDF provides the required GATT and USB behavior directly and gives the test
firmware a reproducible command-line build.

## Intended test boundary

The ESP32 exercises the real platform path:

```text
Float View application service
        |
CoreBluetooth / Android BluetoothGatt
        |
real BLE scan, connection, discovery, subscription, writes, notifications
        |
ESP32-S3 VESC test peripheral
        |
versioned USB control protocol
        |
host scenario engine on the development Mac
```

This makes it suitable for:

- scan filtering and advertisement handling;
- connection and service-discovery state transitions;
- characteristic-property and notification-subscription checks;
- ATT writes and BLE notification boundaries;
- packet fragmentation, coalescing, latency, silence, and disconnects;
- platform reconnection, WebView teardown, background ownership, and durable
  log behavior;
- repeated connection/log/disconnect cycles without powering a motor controller.

It is not sufficient by itself for:

- proving VESC or Refloat wire behavior not already grounded in a fixture,
  published protocol fact, pinned reference, or hardware capture;
- reproducing every VESC Express controller/link-layer quirk;
- deterministic RSSI, radio-off, OS permission, or RF out-of-range behavior;
- sensor accuracy or motor/controller behavior;
- release qualification against Thor300, Thor301, XRV, PintV, or VBMS.

OS permission and radio-state tests stay platform-owned. Actual compatibility
and sensor behavior stay in the physical hardware matrix.

## Architecture

### Thin ESP32 transport firmware

Keep the firmware intentionally small. It owns BLE mechanics, USB framing, and
bounded transport state; it does not own the complete VESC/Refloat behavioral
model.

Responsibilities:

- remain non-advertising after boot until armed over USB;
- advertise a clearly named connectable test peripheral with the VESC service
  UUID and a test-only manufacturer-data marker;
- expose the canonical VESC UART profile:
  - service `6e400001-b5a3-f393-e0a9-e50e24dcca9e`;
  - client-write characteristic `6e400002-b5a3-f393-e0a9-e50e24dcca9e`;
  - server-notify characteristic `6e400003-b5a3-f393-e0a9-e50e24dcca9e`;
- report connect, disconnect, subscription, negotiated MTU, and GATT writes to
  the host;
- accept host commands to notify byte chunks, delay them, stop advertising,
  reject writes where supported, or terminate the connection;
- bound every USB and BLE queue, payload, retry, and pending acknowledgement;
- reset to a known disconnected, non-advertising state after host loss or a
  watchdog timeout;
- emit structured events only; ordinary diagnostic text must not corrupt the
  control stream.

The initial firmware supports exactly one BLE central. This matches Float
View's permanent one-active-board rule and avoids scenario state leaking between
clients. Parallel platform runs use one ESP32 per destination.

### Host scenario engine

The host-side engine owns behavior that should be easy to change and inspect:

- VESC frame assembly, encoding, CRC, and request matching;
- transport firmware identity;
- CAN node topology and forwarded command handling;
- primary controller identity and firmware;
- Refloat INFO, dynamic field IDs, and telemetry responses;
- deterministic telemetry values and state transitions;
- response fragmentation, coalescing, delay, silence, corruption, and
  disconnect rules;
- assertions over commands, ordering, cadence, retries, and cleanup;
- evidence manifests and bounded transcripts.

Implement it in TypeScript/Node so it can share the repository's fixture loading,
schema validation, conformance conventions, and command tooling. It must be
independently tested against the committed raw fixture corpus. It must not copy
or import the frozen legacy web/native implementation.

The host should assemble VESC frames from raw GATT writes. The ESP reports the
actual write callback boundaries rather than interpreting application frames.
In the other direction, the host supplies an ordered list of notification
chunks, allowing tests to preserve captured boundaries or request adversarial
ones.

### Versioned USB control protocol

Use a small versioned, acknowledged protocol. JSON Lines is adequate for the
expected traffic if binary values are encoded as hex and strict message/line
limits are enforced.

Every message contains:

- `protocolVersion`;
- monotonically increasing `sequence`;
- command/event `kind`;
- bounded structured payload;
- acknowledgement or terminal error for each host command;
- ESP monotonic timestamp for BLE-side events.

Minimum host commands:

- `hello`;
- `reset`;
- `configureGattProfile`;
- `startAdvertising`;
- `stopAdvertising`;
- `notify`;
- `setWritePolicy`;
- `disconnect`;
- `getState`.

Minimum device events:

- `booted`;
- `configured`;
- `advertisingChanged`;
- `connected`;
- `mtuChanged`;
- `subscriptionChanged`;
- `gattWrite`;
- `notificationSent`;
- `disconnected`;
- `fault`.

`notify` includes exact bytes, chunk order, optional inter-chunk delay, and a
command identifier used to correlate completion. Disconnect and notification
commands must acknowledge completion rather than merely queueing work.

### Scenario model

Scenarios are declarative, versioned, and schema-validated. They reference raw
fixtures or independently encoded values with documented provenance instead of
duplicating unexplained packet hex.

A scenario can define:

- advertisement name and test marker;
- transport hardware and firmware;
- zero or more CAN nodes;
- the selected primary VESC;
- Refloat package/version and dynamic field layout;
- telemetry values and transitions;
- response rules keyed by decoded request and occurrence number;
- notification chunking and latency;
- protocol and connection fault actions;
- expected request ordering, counts, cadence, and terminal state.

State transitions should use observable barriers, such as "after the fifth
telemetry request," rather than uncoordinated sleeps. The host records actual
request timestamps so cadence assertions can still verify the 2 Hz idle probe
and 10 Hz riding behavior.

One scenario definition should be reusable across Swift/Kotlin fixture tests,
fake-host conformance where applicable, and the ESP transport. Platform-specific
copies of the same VESC personality are prohibited.

## Proposed repository shape

The exact build integration may be adjusted during T0, but dependency direction
should remain:

```text
tools/vesc-test-device/
  README.md
  firmware/
    CMakeLists.txt
    sdkconfig.defaults
    main/
      BLE peripheral and USB control implementation
  host/
    package.json
    src/
      serial transport
      VESC request matcher/response encoder
      scenario engine
      evidence writer
    test/
  schemas/
    control-v1.schema.json
    scenario-v1.schema.json

conformance/scenarios/
  vesc-test-device/
    named scenario manifests

fixtures/
  vesc/
  refloat/

artifacts/verification/
  vesc-test-device/
    ignored generated transcripts and manifests
```

The test-device host may depend on fixture and conformance assets. Product
native modules and the frontend must never depend on the test-device tool.
Firmware output and local ESP-IDF build directories are generated artifacts and
must not be committed.

## First slice

The first useful slice is one programmable, read-only VESC personality:

- advertisement and canonical VESC UART GATT profile;
- VESC Express T transport reporting VESC 6.05;
- a CAN ping response containing a Thor300 primary controller;
- forwarded Thor300 firmware identity reporting VESC 6.06;
- Refloat INFO reporting 1.2.3;
- Refloat dynamic telemetry field identifiers;
- deterministic telemetry responses following:
  - idle;
  - riding;
  - idle with the configured post-ride tail;
- at least one captured notification-boundary pattern and one deliberately
  fragmented pattern;
- a host-commanded unexpected disconnect and a separate explicit-disconnect
  happy path;
- a durably completed Float View session with valid SQLite integrity and
  expected sampling-mode evidence.

The first slice is complete only when the same scenario can run sequentially
against Android, iOS, and macOS. Android is the initial bring-up destination,
but an Android-only result does not complete the slice.

Required observations:

- discovery occurs through the advertised VESC UUID, not a test-only injection;
- the client enables notifications through the production adapter;
- the observed command transcript contains the expected Express, CAN, firmware,
  Refloat INFO, field-layout, and telemetry sequence;
- notification chunks cross arbitrary VESC frame boundaries and decode correctly;
- idle requests are approximately 2 Hz and riding requests approximately 10 Hz
  within platform-specific tolerance;
- idle persistence is approximately 1 Hz and riding persistence approximately
  10 Hz;
- the ride-state transition and post-ride behavior are stored;
- explicit disconnect flushes and closes the session before BLE teardown;
- the session database passes integrity checks;
- no configuration, firmware, or motor-affecting command is accepted or emitted.

## Milestones

Do not add repository commands as no-op placeholders. Each command is introduced
only when it performs its stated verification.

T0-T3 are the path to the first useful slice and are expected to take roughly
two to three engineering weeks. T4 makes that slice unattended and
repository-native; T5-T6 broaden fault and soak coverage incrementally.

### T0: board and toolchain spike — complete (2026-07-28)

Target effort: one to two days.

Deliverables:

- record the exact ESP32-S3 board/module, flash size, USB interfaces, and antenna;
- pin an ESP-IDF release and document command-line installation;
- build, flash, monitor, reset, and reconnect to the board from the development
  Mac;
- prove a minimal NimBLE peripheral can advertise a 128-bit service UUID;
- prove bidirectional structured USB serial traffic while BLE is active;
- decide whether native USB Serial/JTAG or the board's USB-UART bridge is the
  stable control interface.

Exit gate:

- [x] a clean checkout can build and flash the spike using documented commands;
- [x] the host reliably rediscovers the board after reset without a hard-coded
  device path;
- [x] a macOS CoreBluetooth inspector can see the advertised test service;
- [x] USB and BLE can operate concurrently for ten minutes without reset or stream
  corruption.

Evidence is recorded in
[`../tools/vesc-test-device/T0-EVIDENCE.md`](../tools/vesc-test-device/T0-EVIDENCE.md).

### T1: production-shaped VESC GATT transport

Target effort: two to four days after T0.

Deliverables:

- [x] canonical service/write/notify characteristic table;
- [x] connectable advertising with explicit test name and manufacturer marker;
- [x] one-central enforcement;
- [x] notification subscription tracking;
- [x] raw GATT write events over USB;
- [x] host-commanded notifications and disconnect;
- [x] bounded queues, watchdog, reset, and structured fault reporting;
- [x] firmware unit/component tests where logic is platform-independent.

Exit gate:

- [x] Float View discovers the ESP through its production macOS scan filter;
- [x] Android, iOS, and macOS each connect, discover characteristics, enable
  notifications, write a probe frame, receive a response, and disconnect;
- [x] captured client writes arrive byte-for-byte at the host using Float
  View's production macOS CoreBluetooth adapter;
- [x] host-supplied chunks arrive byte-for-byte at Float View's production
  macOS CoreBluetooth adapter;
- [x] unplugging or terminating the host returns the ESP to a safe
  non-advertising/disconnected state.

The implementation evidence and completed platform matrix are recorded in
[`../tools/vesc-test-device/T1-EVIDENCE.md`](../tools/vesc-test-device/T1-EVIDENCE.md).

### T2: host protocol and scenario engine — complete (2026-07-28)

Target effort: three to five days.

Deliverables:

- [x] versioned USB transport implementation with acknowledgements and timeouts;
- [x] schema-validated scenario loader;
- [x] VESC stream assembly and request matching;
- [x] fixture-backed response loading and notification chunk planning;
- [x] Express/CAN/primary-controller routing model;
- [x] Refloat INFO, field-layout, and deterministic telemetry response support;
- [x] bounded transcript and verification manifest;
- [x] unit tests for partial writes, combined frames, CRC errors, unknown commands,
  duplicate commands, host timeout, ESP reset, and stale acknowledgements.

Exit gate:

- [x] the engine passes its tests without an ESP;
- [x] the engine reproduces selected committed fixtures byte-for-byte;
- [x] the ESP round-trips the firmware, CAN, Refloat identity, field-layout, and
  telemetry request sequence;
- [x] unknown or mutating commands are rejected and recorded;
- [x] scenario failure messages identify the first violated expectation.

Host-only and completed physical-gate evidence are recorded in
[`../tools/vesc-test-device/T2-EVIDENCE.md`](../tools/vesc-test-device/T2-EVIDENCE.md).

### T3: deterministic Express/Thor/Refloat ride

Target effort: three to five days.

Deliverables:

- [x] the deterministic ride scenario portion of the first slice described above;
- [ ] idle, riding, post-ride, fault/footpad, and return-to-idle values
  (ride/footpad states complete; explicit fault normalization remains);
- [x] captured and adversarial notification chunk patterns;
- [x] request-cadence assertions;
- [x] application-session extraction and integrity assertions
  (implemented and physically verified on macOS);
- [x] one guided command that arms the ESP and runs the host-side scenario for a
  selected platform.

Exit gate:

- the complete first-slice workflow passes sequentially on Android, iOS, and
  macOS;
- all required observations in the first-slice section are present in one
  machine-readable evidence manifest per platform;
- each run ends with no BLE connection, pending notification, host command,
  application timer, or active logging job;
- the same scenario and response assets are used on all three platforms.

T3 is the first useful delivery. It should land before broad error injection.
Guided app interaction is acceptable at T3 as long as the scenario transcript
and session evidence are machine-readable. T4 removes that manual step.

Current host and physical macOS evidence and the remaining physical platform matrix are
recorded in
[`../tools/vesc-test-device/T3-EVIDENCE.md`](../tools/vesc-test-device/T3-EVIDENCE.md).

### T4: platform automation and repository verification

Target effort: one to two weeks, with physical iOS automation likely the largest
source of variability.

Deliverables:

- `just vesc-test-device-doctor`;
- `just vesc-test-device-build`;
- `just vesc-test-device-flash [port]`;
- `just vesc-test-device-run <scenario> [port]`;
- `just vesc-test-device-smoke <platform> <scenario>`;
- Android physical-device driver using ADB/instrumentation;
- iOS physical-device driver using XCUITest/CoreDevice-compatible tooling;
- macOS application/test driver;
- debug/test-only selection of the manufacturer marker through normal
  application-service operations;
- JUnit-compatible results and repository verification manifests.

Exit gate:

- one command performs setup validation, scenario arming, app launch, connection,
  logging, disconnect, database verification, and cleanup;
- test-control selection is absent from release builds;
- rerunning after a failed test resets both the ESP and app to known state;
- failure artifacts contain enough bounded context to distinguish USB, BLE,
  protocol, application, and storage failures.

### T5: representative negative paths

Target effort: one to two weeks incrementally.

Deliverables:

- malformed CRC, invalid length, noise, truncation, coalescing, silence, delayed
  response, and unexpected-command scenarios;
- disconnect during service startup, identity discovery, idle telemetry, riding,
  and log close;
- missing CAN primary, ambiguous CAN primary, unsupported firmware, malformed
  Refloat INFO, missing field layout, and changing field layout;
- reconnect success, bounded retry, and reconnect-window expiry;
- write rejection and notification pressure where the ESP/NimBLE APIs permit
  faithful simulation;
- a documented matrix identifying cases that remain software-fake-only or
  real-hardware-only.

Exit gate:

- every scenario names the layer and expected stable application error/state;
- no scenario depends on matching platform diagnostic text;
- Swift and Kotlin reach equivalent domain outcomes where platform semantics
  permit;
- transcripts and app evidence agree on ordering and terminal cleanup.

### T6: GATT personalities and soak

Target effort: one to three weeks incrementally.

Deliverables:

- boot-time GATT profiles for missing service, missing characteristic, wrong
  properties, and subscription/CCCD failure where accurately implementable;
- distinct peripheral identity/cache strategy for changing attribute tables;
- repeated 20-cycle connect/log/disconnect runs;
- background, WebView teardown, screen-off/window-closed, and restoration runs;
- optional multi-board lab support using one ESP per concurrent destination;
- USB hub recovery and board health reporting for unattended execution.

Exit gate:

- supported GATT-negative scenarios produce the expected adapter/domain state on
  all relevant platforms;
- twenty cycles complete without stale connections, duplicate samples, corrupt
  logs, unbounded queues, or unacceptable process growth;
- ESP results are clearly separated from real-board release evidence.

## Test orchestration

A platform run follows an acknowledged state machine:

```text
doctor
  -> reset ESP
  -> load scenario
  -> start advertising
  -> launch/reset app
  -> begin scan
  -> select test marker
  -> connect through production Bluetooth adapter
  -> wait for scenario barriers
  -> validate cadence and state transitions
  -> explicit or commanded disconnect
  -> verify app/session state and SQLite integrity
  -> stop advertising
  -> assert both sides quiescent
  -> write evidence
```

The runner uses event barriers and bounded deadlines, never fixed sleeps as proof
of success. Short polling may be used to observe external state, but every wait
has a named timeout and reports the last known app, ESP, and host state.

The test marker helps automation choose the emulator among real nearby VESC
devices. It must not bypass scan, connection, service discovery, notification
setup, protocol parsing, or application state machines. Release builds may
display the ESP as an ordinary compatible VESC if manually selected, but cannot
enable remote test control or automatic test-device selection.

## Evidence

Each completed run writes an ignored detailed transcript and a compact manifest
containing:

- repository commit and dirty-state flag;
- ESP firmware commit/build ID and control-protocol version;
- scenario ID, schema version, and content digest;
- platform, OS, physical device, app build, and test runner version;
- redacted ESP identity and USB selector;
- advertisement, connection, subscription, and disconnect timestamps;
- decoded request sequence and response-fixture IDs;
- BLE write and notification chunk sizes;
- idle/riding request cadence;
- observed and committed sample sequences/counts;
- ride-state transitions, gaps, drops, and errors;
- session database integrity result and terminal reason;
- cleanup state for ESP, host, app connection, and logging job.

Do not record BLE addresses, session tokens, unbounded telemetry, or unrelated
device identifiers. Stable summarized evidence may be referenced from the
roadmap only after its corresponding gate passes.

## Safety and failure containment

- The emulator implements read-only commands needed by the accepted slice.
- Configuration, firmware, raw privileged, and motor-affecting commands fail
  closed and are recorded.
- Wi-Fi and remote network control are disabled.
- Boot defaults to non-advertising.
- Host loss stops advertising and terminates the active connection after a
  bounded watchdog interval.
- USB and BLE input sizes, queues, notification rates, and scenario durations
  are capped.
- Test-device selection exists only in debug/test automation.
- The peripheral name contains `Test`; automation also requires the
  manufacturer marker so names alone are never trusted.
- One ESP serves one active client; parallel tests require explicit separate
  hardware identities.

## Risks and mitigations

### GATT caching

iOS and Android can cache attribute tables. GATT-negative personalities should
use stable personality-specific identities or a documented cache reset. Do not
change the GATT table mid-connection.

### False protocol authority

Generated responses can accidentally become an unsupported protocol claim.
Every response references provenance and is checked against committed fixtures.
Real hardware remains the compatibility authority.

### Harness drift

The host encoder is another implementation. Golden tests compare it with raw
fixtures, and scenario schemas reference semantic fixture IDs. Product code
never imports the harness.

### Timing flakiness

BLE and mobile scheduling are not deterministic. Scenario state changes use
request barriers; timing assertions use measured windows and declared
platform-specific tolerance. Exact state-machine tests remain fake-clock unit
tests.

### ESP reset and USB renumbering

The host discovers the board by stable USB metadata, waits for a versioned
`booted` event, and discards acknowledgements from an earlier boot ID.

### Incomplete BLE fault simulation

NimBLE may automatically manage CCCDs or prevent faithful reproduction of some
invalid profiles. T0/T5 record the supported matrix. Unsupported BLE faults stay
in adapter fakes or real-hardware tests rather than being approximated
misleadingly.

## Completion relationship to R6-R8

T0-T3 provide a repeatable real-radio development fixture and improve R6
regression coverage. T4-T6 strengthen R6-R8 automation, failure, lifecycle, and
soak testing.

These milestones do not independently complete R6, R7, or R8. The roadmap still
requires the declared physical boards, platform/OS destinations, background
evidence, and repeated hardware acceptance. Emulator evidence is labelled
`vesc-test-device`, never `hardware-smoke`, unless a future accepted verification
decision explicitly changes that terminology.

## External implementation references

- [ESP-IDF NimBLE GATT server and data-exchange
  guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/api-guides/ble/get-started/ble-data-exchange.html)
- [ESP32-S3 USB Serial/JTAG console
  guide](https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/usb-serial-jtag-console.html)
- [`reference-implementations.md`](reference-implementations.md) for VESC Tool,
  fixture provenance, licensing boundaries, and physical-hardware precedence
