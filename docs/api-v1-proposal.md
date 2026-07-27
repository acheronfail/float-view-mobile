# Float View local API v1 proposal

Status: accepted direction; Phase 1 implementation is in progress.

Implemented so far:

- `api/asyncapi.yaml` and JSON Schema 2020-12 definitions for common, system,
  Bluetooth, device, telemetry, and logging state;
- schema-validated `system.hello` request/response examples;
- committed generated TypeScript, Swift, and Kotlin models with drift checks;
- a transport-neutral TypeScript JSON-RPC client;
- `/api/v1/ws` mock support for `system.hello` and `system.getSnapshot`;
- stable error envelopes and focused client/mock-handler tests;
- Swift and Kotlin example round-trip tests;
- a reusable black-box system conformance suite running against the real mock
  WebSocket.

The legacy `/ws` contract remains the active UI/native path. Authentication,
other operations, native service boundaries, native v1 dispatch, and
hardware-backed conformance remain to be implemented.

## Decision summary

Use JSON-RPC 2.0 messages over one authenticated, same-origin WebSocket. Describe
the channel and operations with AsyncAPI 3, and define payloads with JSON Schema
2020-12. Generate TypeScript, Swift, and Kotlin data models from those schemas.

This retains the inspectability and easy browser mocking of the current JSON
protocol while adding correlation, stable errors, notifications, version
negotiation, and a standard request/response shape. Protobuf is not recommended
for v1 because binary encoding and browser tooling add complexity without solving
the harder problem: defining identical lifecycle behavior on every platform.

VESC protocol behavior follows the precedence and provenance policy in
[`reference-implementations.md`](reference-implementations.md). VESC Tool is the
canonical implementation reference; the API contract does not expose its
internal Qt types or make either upstream repository a build dependency.

The HTTP server should serve only:

- the static UI;
- a small bootstrap document containing the WebSocket path, a per-launch session
  token, and supported API major version;
- an optional health endpoint;
- test-control endpoints in explicitly marked test builds only.

## Source tree

Once accepted, create:

```text
api/
  asyncapi.yaml
  schemas/
    common.schema.json
    system.schema.json
    bluetooth.schema.json
    devices.schema.json
    telemetry.schema.json
    logging.schema.json
  examples/
  compatibility.md
  generated/
    typescript/
    swift/
    kotlin/
conformance/
  src/
  scenarios/
  fixtures/
    vesc/
    refloat/
```

Commit the schemas and generated output, or regenerate in CI and fail on a diff.
Do not continue with ignored generated models once the contract is established;
reviewers need to see the native impact of a schema change.

## Connection and security

The server binds to `127.0.0.1` and `::1` only, on an ephemeral port. At launch it
generates a cryptographically random token and gives it only to the hosted UI via
the bootstrap document or an injected initial script. The WebSocket handshake:

- uses a versioned path such as `/api/v1/ws`;
- supplies the token as a WebSocket subprotocol or one-time query value;
- validates the exact expected `Origin`;
- rejects missing, expired, or reused bootstrap credentials;
- sets size, message-rate, idle, and connection-count limits.

The current bare `/ws` endpoint remains only during migration.

## Envelope

Requests and responses use JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": "01K123ABC",
  "method": "device.connect",
  "params": {
    "deviceId": "opaque-session-device-id",
    "timeoutMs": 10000
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": "01K123ABC",
  "result": {
    "device": {
      "deviceId": "opaque-session-device-id",
      "state": "ready"
    }
  }
}
```

Errors use JSON-RPC's error envelope and project-owned stable codes:

```json
{
  "jsonrpc": "2.0",
  "id": "01K123ABC",
  "error": {
    "code": -32012,
    "message": "Bluetooth permission is required",
    "data": {
      "kind": "permission_required",
      "permission": "bluetooth_scan",
      "recoverable": true
    }
  }
}
```

Server events are JSON-RPC notifications:

```json
{
  "jsonrpc": "2.0",
  "method": "telemetry.sample",
  "params": {
    "subscriptionId": "sub_01K123",
    "sequence": 42,
    "observedAt": "2026-07-27T11:15:02.123Z",
    "deviceId": "opaque-session-device-id",
    "sample": {
      "voltageV": 56.2,
      "motorCurrentA": 12.4,
      "pitchDeg": 1.7
    }
  }
}
```

Requests must receive one terminal response even if the underlying work later
produces lifecycle events. Cancellation is an explicit method. Notifications are
best effort; reconnecting clients recover truth through snapshots, not replaying
every UI event.

## Version negotiation

The first request is `system.hello`:

```json
{
  "jsonrpc": "2.0",
  "id": "hello-1",
  "method": "system.hello",
  "params": {
    "client": { "name": "float-view-web", "version": "0.1.0" },
    "api": { "major": 1, "minMinor": 0, "maxMinor": 2 }
  }
}
```

The result includes:

- selected API major/minor;
- backend name/version/platform;
- a typed capability list with constraints;
- Bluetooth/radio/permission state;
- current devices, connections, telemetry subscriptions, and logging jobs;
- limits such as maximum connections, sample rate, and message size.

Major versions are incompatible and use different WebSocket paths. Minor versions
add optional fields, operations, enum values, and capabilities without changing
existing semantics. Clients ignore unknown optional fields and events. Backends
must not claim a capability whose conformance scenarios they do not pass.

## Proposed operations

Names are grouped by stable domain rather than OS framework.

### System and Bluetooth

- `system.hello`
- `system.getSnapshot`
- `system.getCapabilities`
- `bluetooth.getState`
- `bluetooth.requestPermission` — triggers the appropriate native prompt; result
  reports the resulting state rather than assuming success.

Events:

- `system.snapshotChanged` for coarse invalidation;
- `bluetooth.stateChanged`.

### Discovery

- `discovery.start({ service: "vesc", timeoutMs }) -> { scanId }`
- `discovery.stop({ scanId })`
- `discovery.get({ scanId })`

Events:

- `discovery.deviceFound`
- `discovery.deviceUpdated`
- `discovery.completed`

A scan always reaches one terminal state: `completed`, `cancelled`, `timed_out`,
or `failed`. Duplicate advertisements update the same opaque `deviceId`.

### Devices and connections

- `device.getActive`
- `device.connect({ deviceId, timeoutMs })`
- `device.disconnect({ deviceId })`
- `device.get({ deviceId })`
- `device.cancelConnect({ deviceId })`

Events:

- `device.stateChanged`
- `device.ready`
- `device.disconnected`

Connection state is an enum with specified transitions:

```text
discovered -> connecting -> connected -> discovering_services -> ready
                       \-> failed
ready -> disconnecting -> disconnected
ready -> reconnecting -> ready | disconnected
```

`device.connect` succeeds when the backend accepts and begins the connection
attempt; readiness is reported by state/event. A duplicate connect is idempotent
when compatible with the existing attempt. Exactly zero or one device may be
actively connecting/connected. A request for a different device while one is
active returns `invalid_state`. Active-connection state is a nullable singleton;
discovery results, saved devices, and historical sessions remain collections.

An unexpected disconnect may enter bounded `reconnecting` state for the active
Session. The user setting defaults to five minutes and may disable or shorten
reconnection. Explicit disconnect never reconnects. Every attempt and gap is a
session event.

### Saved devices

- `savedDevices.list`
- `savedDevices.get({ savedDeviceId })`
- `savedDevices.rename({ savedDeviceId, name })`
- `savedDevices.forget({ savedDeviceId })`

A successful first connection saves a platform-local reconnect binding by
default. `savedDeviceId` is opaque and persistent within that installation.
Historical sessions retain an immutable device snapshot when a saved device is
renamed or forgotten.

### Board profiles

- `boardProfiles.get({ savedDeviceId })`
- `boardProfiles.update({ savedDeviceId, expectedRevision, patch })`
- `boardProfiles.resetFields({ savedDeviceId, expectedRevision, fields })`
- `boardProfiles.refreshDiscovery({ savedDeviceId })`
- `boardProfiles.getDiscovery({ savedDeviceId })`

Events:

- `boardProfiles.changed`
- `boardProfiles.discoveryChanged`

Profiles contain versioned app-local identity/visual, battery topology and
chemistry, motor/drivetrain, rail, voltage/speed calibration, controller CAN, and
VBMS settings. All fields are optional; saving a device creates a partial
profile and missing values are warnings, not logging blockers. Discoverable
fields retain discovered, configured, and effective values; manual mode wins
until explicitly returned to automatic. Updates use optimistic concurrency.

Profiles never write controller configuration. Effective profile/discovery
snapshots are copied into sessions so completed analysis remains reproducible.
Detailed fields and validation are specified in
[`board-profiles.md`](board-profiles.md).

### VESC domain

Prefer typed operations:

- `vesc.getFirmwareInfo`
- `vesc.getRealtimeValues`
- `vesc.getSetupValues`
- `refloat.getInfo`
- `refloat.getAllData`

Responses use normalized domain objects with explicit units. Keep low-level VESC
packet IDs internal. If a raw escape hatch is later necessary, gate
`vesc.rawCommand` behind a capability and strict payload/response size and safety
checks.

Mutating board configuration, firmware updates, and motor-affecting commands are
out of the read-only v1 milestone. They require a separate safety design,
confirmation policy, and hardware test plan.

### Telemetry

- `telemetry.subscribe({ deviceId, profile, intervalMs, fields? })`
- `telemetry.update({ subscriptionId, intervalMs, fields? })`
- `telemetry.unsubscribe({ subscriptionId })`
- `telemetry.list`

The result returns the effective interval and fields after backend clamping.
Each sample has a monotonically increasing sequence number. The backend maintains
only a small bounded queue per client; when the UI is slow it drops/coalesces old
samples and reports a `droppedSincePrevious` count. Native logging uses its own
sampling path and must never depend on WebSocket delivery.

Logging uses adaptive native rates. The initial defaults are 1 Hz while
connected but not riding, with a 2 Hz lightweight Refloat ride-state probe; 10 Hz
while riding; and a 15-second post-ride period at the riding rate.
Requested/effective rates and every mode transition are persisted. Faults,
footpad edges, alerts, and state transitions are recorded regardless of periodic
cadence.

### Logging

- `logging.stop({ jobId })`
- `logging.get({ jobId })`
- `logging.getActive`
- `logging.getStorageState`

Events:

- `logging.jobChanged`
- `logging.samplingModeChanged`
- `logging.storageChanged`
- `logging.samplesCommitted`

Normal product flow creates a durable session and starts its logging job
automatically when a connection reaches `ready`. Manual `logging.start` is a
test/advanced capability. A job state includes its `sessionId`, requested versus
effective rates, sampling mode, timestamps, sample/error counts, bytes written,
background capability, and last error. The backend persists job/session metadata
before reporting normal Live readiness. WebSocket disconnect and WebView
destruction do not stop a job. App/OS termination behavior is capability-defined
and visible in later snapshots.
The complete persistence, recovery, background-lifecycle, WebView teardown, and
bounded query semantics are defined in
[`durable-logging.md`](durable-logging.md).

### Sessions

- `sessions.list({ filters, sort, cursor, limit })`
- `sessions.getFilterOptions`
- `sessions.get({ sessionId })`
- `sessions.getSummary({ sessionId })`
- `sessions.querySeries({ sessionId, range, fields, resolution, cursor? })`
- `sessions.listEvents({ sessionId, range, types?, cursor? })`
- `sessions.getDiagnosticWindow({ sessionId, eventId?, centerElapsedMs?, beforeMs, afterMs, fields?, cursor? })`
- `sessions.export({ sessionId, format })`
- `sessions.delete({ sessionId })` — explicit, confirmed by the frontend,
  idempotent, and separately tested.

Session groups:

- `sessionGroups.list({ filters, sort, cursor, limit })`
- `sessionGroups.get({ sessionGroupId })`
- `sessionGroups.unlink({ sessionGroupId })`

An optional user setting automatically creates reversible catalog groups for
eligible Sessions with the same `deviceHistoryId` and a gap within the configured
window. It is disabled by default; the suggested enabled window is two hours.
Groups never merge per-session databases. Overview/review combines bounded
member queries in the shared frontend; a diagnostic window always names the
concrete member `sessionId`.

Events:

- `sessions.created`
- `sessions.updated`

`sessions.list` performs indexed server-side filtering by historical device,
date, riding/total duration, distance, state, and integrity, with cursor
pagination. `sessions.getFilterOptions` includes forgotten historical device
groups without exposing a deleted platform reconnect binding.
Review queries return bounded rollups; diagnostic queries return bounded lossless
windows. User-facing APIs use `sessionId`; the earlier proposed `logId` name is
retired.

### Settings

- `settings.get`
- `settings.update({ expectedRevision, patch })`

Settings include requested/effective idle probe and persistence rates, riding
rate, post-ride tail, active-session reconnection duration, automatic
session-linking enabled/window, alert output choices, and diagnostic raw-packet
caps. Updates use optimistic concurrency. Backends validate and clamp values to
capability-advertised limits and return both requested and effective values.

### Alerts

- `alerts.listRules`
- `alerts.replaceRules({ version, rules })`

Events:

- `alerts.triggered`

The shared UI authors declarative rules. A minimal native evaluator applies
threshold, boolean-edge, duration/debounce, hysteresis, and cooldown behavior to
every normalized sample before UI coalescing so alerts still work when the
WebView is absent. Rule semantics, ordering, and fixtures are contract-owned and
identical on every backend. Native code owns only background evaluation and
platform alert output; rule UX and visualization stay in the shared frontend.
The first configurable rules cover duty cycle, high/low pack voltage,
motor/controller temperature, individual/combined footpad state, and connection
loss. Alerts are advisory and acknowledgement does not disable the rule.

## Stable errors

Reserve project error codes and pair them with a string `data.kind`. Initial
families:

- `invalid_state`
- `unsupported`
- `unsupported_firmware`
- `permission_required`
- `bluetooth_unavailable`
- `scan_failed`
- `device_not_found`
- `connection_failed`
- `connection_timeout`
- `device_not_ready`
- `protocol_error`
- `command_timeout`
- `rate_limited`
- `storage_unavailable`
- `background_unavailable`
- `cancelled`
- `internal`

For every operation, the AsyncAPI description lists allowed errors and whether a
retry is meaningful. Native exceptions and GATT/CoreBluetooth status text are
diagnostic details, never the stable contract.

## Backend interfaces

Give Swift and Kotlin equivalent ports:

```text
BluetoothAdapter
  observeState()
  startDiscovery(filter)
  stopDiscovery(scanId)
  connect(deviceId)
  disconnect(deviceId)
  notifications(deviceId)
  write(deviceId, characteristic, bytes, mode)

Clock
  monotonicNow()
  utcNow()
  schedule(...)

LogStore
  createJob(...)
  append(...)
  updateJob(...)
  listJobs()
  export(...)

EventSink
  publish(typedEvent)
```

The application service owns the connection and logging state machines and
depends on these interfaces. CoreBluetooth and Android BluetoothGatt are adapter
implementations. Scripted fake adapters use the same interface in tests and test
builds. The loopback server translates JSON-RPC to application-service calls.

macOS should reuse the Swift service/protocol modules. Later Linux and Windows
backends implement the same application boundary with BlueZ and WinRT.

## Conformance test architecture

Create one Node/TypeScript black-box runner from the contract. It connects to a
backend URL/token and executes scenario files. The same suite runs against:

1. the development mock;
2. Android instrumentation with a fake `BluetoothAdapter`;
3. an iOS test host with a fake `BluetoothAdapter`;
4. macOS and future desktop backend processes;
5. optional real hardware profiles.

Each test backend exposes a launch-only test control channel, unavailable in
release builds, that loads a deterministic script such as:

```yaml
name: connect-and-stream
radio: powered_on
devices:
  - fixture: refloat-6.6
timeline:
  - atMs: 20
    emit: advertisement
  - afterWrite: COMM_FW_VERSION
    chunksFrom: fixtures/vesc/fw-6.6.bin
  - everyMs: 200
    chunksFrom: fixtures/refloat/rtdata-sequence.bin
```

The runner asserts JSON shape and behavior: response correlation, allowed state
transitions, ordering, timeouts, idempotency, reconnection snapshots, queue
bounds, and stable errors.

Below that black-box layer:

- **Schema tests** validate every example and reject known-invalid messages.
- **Generator tests** snapshot Swift/Kotlin/TypeScript output and compile it.
- **Protocol golden tests** feed identical raw byte chunks—including split,
  combined, corrupt-CRC, and truncated packets—into every implementation and
  compare normalized results.
- **Service tests** use fake Bluetooth, clock, and storage to drive every state
  transition without sleeps.
- **Platform tests** cover permission prompts, radio changes, restoration,
  Activity recreation, foreground services, iOS background transitions, and
  filesystem behavior.
- **Hardware-in-the-loop tests** are tagged, opt-in, read-only by default, and run
  against declared device/firmware profiles.

CI should initially require schema/generation, web/mock conformance, native unit
tests, and fake-adapter conformance. Simulator/emulator and hardware jobs can be
separate while infrastructure matures.

## Historical migration plan

The phases below describe the original incremental migration. The project has
since chosen a clean native reset so iOS, Android, and macOS can begin from the
same boundaries. Use [`reset-roadmap.md`](reset-roadmap.md) as the active
execution plan and [`vertical-slice-v1.md`](vertical-slice-v1.md) as the
acceptance specification. This section remains as design history.

### Phase 0: freeze and characterize

- Capture current scan/connect/command flows as integration scenarios.
- Add raw packet fixtures for firmware, values, and Refloat real-time data.
- Record known Swift/Kotlin decoding differences as failing or quarantined cases.

### Phase 1: contract skeleton

- Decide the open questions below.
- Add `api/` AsyncAPI/JSON Schema, examples, generated models, and CI drift checks.
- Implement `system.hello`, snapshots, correlation, and stable errors in the mock.
- Build a central TypeScript client/store with request promises and event routing.

### Phase 2: testable native cores

- Introduce the application service and injected adapter/clock/store interfaces.
- Implement and recovery-test native durable log stores before relying on live
  UI telemetry as the product logging path.
- Move direct `LocalServer.broadcastMessage` calls out of BLE/protocol classes.
- Run conformance scenarios in Android and iOS fake-adapter test hosts.
- Replace or fully harden the handwritten WebSocket framing.

### Phase 3: read-only device milestone

- Implement discovery, connection lifecycle, firmware, values, Refloat data, and
  bounded telemetry on both platforms.
- Require identical fixture and black-box results.

### Phase 4: durable logging/background hardening

- Harden native log stores and jobs already established in the testable cores.
- Add Android foreground-service ownership and iOS background/restoration design.
- Test WebView loss, process/lifecycle transitions, disk pressure, and export.

### Phase 5: desktop

- Extract reusable Swift modules and add the macOS host.
- Add Linux/BlueZ and Windows/WinRT adapters after macOS conformance passes.

## Resolved implementation decisions and remaining gate

The first release is read-only and uses persisted opaque saved-device identity,
one permanent active-connection slot, the SQLite catalog/per-session design, and
honest platform-qualified restoration rather than a promise of uninterrupted
execution after force-quit. The initial protocol target is VESC 6.06 with Refloat
1.2; see [`implementation-scope-lock.md`](implementation-scope-lock.md) for the
hardware/OS matrix and later compatibility pass.

The only unresolved cross-cutting release decision is the project/distribution
license. AGPL-3.0 is the current candidate. It does not block independent R0-R5
architecture work, but it must be accepted before public distribution or code
reuse whose terms depend on it.
