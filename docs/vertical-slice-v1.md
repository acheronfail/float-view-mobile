# First VESC connection and logging vertical slice

Status: acceptance specification for roadmap milestones R3-R8.

This document defines the smallest end-to-end product slice that must behave the
same on iOS, Android, and macOS. JSON shapes come from `api/`; this document owns
the lifecycle semantics and observable acceptance criteria.
[`verification-plan.md`](verification-plan.md) defines how each behavior is
executed and measured.
Protocol interpretation and fixtures follow
[`reference-implementations.md`](reference-implementations.md): VESC Tool is the
canonical implementation, while authorized Floaty code is supplementary and the
original Float View code is historical evidence only.
Native persistence, background execution, and WebView teardown follow
[`durable-logging.md`](durable-logging.md). Those semantics are core acceptance
requirements, not later UI enhancements.

## Scope

The slice lets a user:

1. launch the shared Device/Sessions UI;
2. inspect backend and Bluetooth state;
3. request permission where the OS permits;
4. discover supported VESC devices;
5. save and connect to one device and verify its VESC GATT interface;
6. automatically create a durable session and enter Live;
7. read and display firmware information;
8. switch native sampling from idle to 10 Hz while riding and back after the
   post-ride tail;
9. evaluate live alerts and display bounded telemetry;
10. disconnect, flush the session, and find it under Sessions;
11. open bounded Review and Diagnosis queries.

Configuration writes, firmware installation, motor-affecting commands,
reconnection after force-quit, multi-device operation, imports, GPS, cloud sync,
Linux, and Windows are out of scope. Lossless archive/CSV export and explicit
Session deletion are in scope.

## Required API surface

The schema is authoritative for payload fields. These operations are required:

- `system.hello`
- `system.getSnapshot`
- `bluetooth.getState`
- `bluetooth.requestPermission`
- `discovery.start`
- `discovery.stop`
- `device.connect`
- `device.disconnect`
- `device.getActive`
- `savedDevices.list`
- `savedDevices.get`
- `savedDevices.rename`
- `savedDevices.forget`
- `boardProfiles.get`
- `boardProfiles.update`
- `boardProfiles.resetFields`
- `boardProfiles.refreshDiscovery`
- `boardProfiles.getDiscovery`
- `vesc.getFirmwareInfo`
- `telemetry.subscribe`
- `telemetry.unsubscribe`
- `logging.stop`
- `logging.get`
- `logging.getActive`
- `logging.getStorageState`
- `sessions.list`
- `sessions.getFilterOptions`
- `sessions.get`
- `sessions.getSummary`
- `sessions.querySeries`
- `sessions.listEvents`
- `sessions.getDiagnosticWindow`
- `sessions.export`
- `sessions.delete`
- `sessionGroups.list`
- `sessionGroups.get`
- `sessionGroups.unlink`
- `settings.get`
- `settings.update`
- `alerts.listRules`
- `alerts.replaceRules`

Required events:

- `bluetooth.stateChanged`
- `discovery.deviceFound`
- `discovery.deviceUpdated`
- `discovery.completed`
- `device.stateChanged`
- `telemetry.sample`
- `sessions.created`
- `sessions.updated`
- `boardProfiles.changed`
- `boardProfiles.discoveryChanged`
- `logging.jobChanged`
- `logging.samplingModeChanged`
- `logging.storageChanged`
- `logging.samplesCommitted`
- `alerts.triggered`

Snapshots contain Bluetooth state, active scans, saved/known devices,
profile summary/warnings, connections, active session/job, and telemetry
subscriptions. A reconnecting UI must not require event replay to recover current
truth.

## Backend ports

Both native cores expose semantically equivalent injected ports:

```text
BluetoothAdapter
  observeState()
  startDiscovery(filter)
  stopDiscovery(scanId)
  connect(deviceId)
  discoverVescService(deviceId)
  enableNotifications(deviceId)
  write(deviceId, bytes)
  notifications(deviceId)
  disconnect(deviceId)

Clock
  monotonicNow()
  utcNow()
  schedule(deadline, callback)

LogStore
  createSessionAndJob(metadata)
  append(jobId, records)
  flush(jobId)
  close(jobId, terminalState)
  getJob(jobId)
  listSessions(filter, sort, cursor)
  querySamples(sessionId, range, resolution, cursor)
  recoverJobs()

AlertEvaluator
  replaceRules(version, rules)
  evaluate(sample)

EventSink
  publish(event)
```

Production ports use platform APIs. Tests use deterministic scripts and a fake
clock. The application service, not the WebSocket or Bluetooth adapter, owns
state transitions and error mapping.

## Connection lifecycle

The externally visible path is:

```text
discovered
  -> connecting
  -> connected
  -> discovering_services
  -> enabling_notifications
  -> ready
```

Any intermediate state may transition to `failed` and then `disconnected`.
`device.connect` accepts the attempt and returns its current device state; the
`ready` event is asynchronous. A compatible duplicate connect is idempotent.
An incompatible second device request returns `invalid_state`. Exactly one
active connection slot is a permanent product constraint.

Unexpected loss enters bounded `reconnecting` state for the same Session when
the user setting permits it. The default retry window is five minutes; explicit
disconnect and force-quit never reconnect. Attempts use bounded exponential
backoff and are persisted as events.

The backend reaches `ready` only after it has:

- connected at the OS Bluetooth layer;
- found the required VESC service and characteristics;
- enabled and, where applicable, confirmed notifications;
- initialized the packet decoder;
- completed any required transport handshake.

Firmware reads and telemetry/logging requests before `ready` return
`device_not_ready`.

## Telemetry and logging

For the first profile, the exact fields are finalized in the API schema before
R3 completes. Every field name includes or documents its unit. At minimum the
profile should capture the values required to assess a basic ride/log stream,
subject to what the initial fixture firmware supports.

UI telemetry and native logging are separate consumers:

- UI preview uses a requested interval clamped to backend limits;
- each sample has a monotonic sequence number and UTC observation timestamp;
- the per-client queue is bounded and may coalesce/drop old samples;
- the next delivered sample reports drops since the previous delivery;
- losing the WebSocket cancels or expires its UI subscription;
- losing the WebSocket never implicitly stops a native logging job;
- destroying the WebView never implicitly stops a native logging job;
- logging uses bounded native buffers and records sample, dropped, and error
  counts;
- reaching ready automatically persists session/job metadata before ordinary
  Live state is reported;
- idle probes Refloat ride state at 2 Hz and persists at 1 Hz, riding persists at
  10 Hz, and post-ride retains 10 Hz for 15 seconds before returning to idle,
  all within advertised limits;
- faults, alerts, footpad edges, and state/mode transitions persist independently
  of periodic cadence;
- committed samples can be paged/downsampled through the API while the log is
  active, independently of live preview delivery;
- `logging.stop` stops sampling, flushes data, closes storage, then publishes a
  terminal job state.

The initial test profile logs for 60 seconds at the effective supported interval
with the mobile app backgrounded/screen off or the macOS window closed. It does
not require the WebView to remain alive. If iOS suspends polling, the log contains
an explicit gap and the capability/job state reports best-effort behavior.

The initial hardware/protocol profile is VESC 6.06 with Refloat 1.2. Refloat's
reported ride state is authoritative. Hardware acceptance covers Thor300,
Thor301, Floatwheel XRV, and PintV; relevant scenarios distinguish a VESC Express
transport from the primary motor-controller CAN ID and support VESC VBMS only.

## Ordered disconnect contract

`device.disconnect` is idempotent and safe in every connection state. Its
observable cleanup order is:

1. mark the device `disconnecting` and reject new device commands;
2. stop UI telemetry subscriptions for that device;
3. stop, flush, and terminally close active log jobs, unless the request has
   explicitly been rejected because policy requires logging to be stopped first;
4. cancel pending protocol commands, timers, retries, and queued writes with a
   stable `cancelled` or `invalid_state` result;
5. disable notification consumption and detach callbacks;
6. request the platform Bluetooth disconnect;
7. release GATT/peripheral references and protocol buffers;
8. publish exactly one terminal `disconnected` state and advance the snapshot
   revision.

A bounded timeout forces local cleanup if the OS never confirms disconnection.
Late platform callbacks are ignored by connection generation/session identity.
Calling disconnect again returns the terminal state without duplicating terminal
events or corrupting a completed log.

An unexpected radio loss or remote disconnect follows the same local cleanup
path and includes a stable reason. The first slice does not automatically
reconnect.

## Product and diagnostic UI

The shared UI implements the two-tab and detail flow in
`product-experience.md`. It must expose:

- selected API/backend/platform version and capabilities;
- radio and permission state with the available recovery action;
- scan state and discovered opaque device IDs/names;
- complete connection state and last stable error;
- firmware information;
- live sample sequence/rate/drop count;
- current log job, effective interval, samples, bytes, errors, and state;
- live alerts plus committed fault/footpad/state markers;
- explicit disconnect plus advanced/test stop-log control where enabled;
- a current snapshot/reconnect action useful during testing.

It must not expose BLE MAC addresses as portable identity or contain native
platform branches for domain behavior.

## Deterministic fake scenarios

Every backend test host supports the same test-only scripts:

1. happy path: powered radio, discovery, connection, firmware response, telemetry
   stream, log stop, disconnect;
2. permission required and denied;
3. radio unavailable before scan and radio loss while ready;
4. scan timeout with no devices;
5. connect timeout and service/characteristic mismatch;
6. corrupt packet followed by recovery to a valid packet;
7. slow UI client causing bounded drops without affecting native log samples;
8. WebSocket loss and reconnect snapshot while logging continues;
9. complete WebView destruction/recreation plus durable catch-up while logging
   continues;
10. storage append/flush failure, disk full, crash recovery, and migration;
11. disconnect during connect, during a pending command, and during logging;
12. duplicate connect, stop, and disconnect requests;
13. late Bluetooth callbacks after forced cleanup;
14. OS suspension/background gap and platform restoration.
15. speed/duty/footpad threshold rules with debounce, hysteresis, cooldown, and
    UI coalescing that does not skip native evaluation;
16. bounded eight-hour overview, zoom, and high-resolution diagnostic-window
    queries.
17. partial first-connect profile discovery, manual battery/CAN/BMS overrides,
    voltage correction, and immutable session profile snapshots.

Fake scenarios advance a fake clock and never use real sleeps.

## Programmable VESC scenarios

The ESP32 fixture runs a complementary subset of these behaviors through the
production radio adapters. Its scenario engine may reuse the same committed raw
fixtures and expected normalized domain outcomes, but it does not replace
fake-clock tests or production-board acceptance.

During development, every new protocol, connection, logging, or lifecycle slice
is implemented for both native cores and then exercised physically on
Android 10 first. The fixture must use ordinary discovery, service validation,
notification subscription, VESC framing, application-service sequencing, and
durable storage; test control may select its manufacturer marker only in debug
or instrumentation builds.

The accepted ESP progression is:

1. Express/Thor300/Refloat identity plus deterministic adaptive ride and clean
   disconnect;
2. representative corrupt, delayed, silent, unsupported, topology, and
   unexpected-disconnect/reconnect paths;
3. 60-second background/WebView reconstruction and durable catch-up;
4. twenty clean cycles with resource and terminal-cleanup assertions.

At final R8 qualification, replay the same accepted scenario digests on iOS and
macOS before running the declared real-controller matrix. Android-only ESP
evidence can advance implementation, but cannot complete a cross-platform
milestone.

The accepted R7 Android lifecycle digest is
`express-thor-refloat-background`: it runs longer than 60 seconds, backgrounds
the Activity during an active durable session, observes the `connectedDevice`
foreground service, waits beyond the 30-second frontend grace period, recreates
the frontend, and requires an integrity-valid explicit close with no active
connection/session. R8 Android soak evidence contains 20 production-GATT
cycles, with every fifth cycle using that lifecycle digest and all other cycles
using the ordinary ride digest. Each cycle records bounded resident-memory and
file-descriptor counters. The complementary
`express-thor-refloat-same-process-soak` digest performs 20 complete
connect/log/disconnect cycles in one app process and requires 20 integral SQLite
sessions, terminally clear native state, cadence compliance, and bounded
per-cycle resident-memory and file-descriptor counters.

## Acceptance matrix

| Scenario                        | Mock     | Swift core | Kotlin core | iOS      | macOS    | Android  |
| ------------------------------- | -------- | ---------- | ----------- | -------- | -------- | -------- |
| Contract negotiation/snapshot   | Required | Required   | Required    | Required | Required | Required |
| Fake happy path                 | Required | Required   | Required    | Required | Required | Required |
| Fake failure and cleanup paths  | Required | Required   | Required    | Required | Required | Required |
| Shared packet fixtures          | N/A      | Required   | Required    | Via core | Via core | Via core |
| Real discovery/connect/firmware | N/A      | N/A        | N/A         | Required | Required | Required |
| Real 60-second native log       | N/A      | N/A        | N/A         | Required | Required | Required |
| Twenty clean connection cycles  | N/A      | N/A        | N/A         | Required | Required | Required |

## Definition of done

The vertical slice is complete only when:

- `just verify-r8 <profile>` passes according to
  [`verification-plan.md`](verification-plan.md);
- generated models have no drift and all examples validate;
- every backend passes the same black-box fake scenarios;
- live alert behavior and log query/diagnostic results agree across backends;
- Swift and Kotlin pass the shared packet fixture corpus;
- all three platform hosts complete real discovery, connection, firmware read,
  60-second background/window-closed logging, flush, and disconnect;
- 20 repeated cycles leave no active OS connection, timer, subscription, queued
  write, open log handle, or duplicate terminal event;
- radio loss and permission failures yield stable errors and recoverable
  snapshots;
- the results and supported firmware/OS matrix are recorded in
  [`reset-roadmap.md`](reset-roadmap.md).
