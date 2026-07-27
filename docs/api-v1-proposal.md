# Float View local API v1 proposal

Status: design proposal, not yet the implemented contract.

## Decision summary

Use JSON-RPC 2.0 messages over one authenticated, same-origin WebSocket. Describe
the channel and operations with AsyncAPI 3, and define payloads with JSON Schema
2020-12. Generate TypeScript, Swift, and Kotlin data models from those schemas.

This retains the inspectability and easy browser mocking of the current JSON
protocol while adding correlation, stable errors, notifications, version
negotiation, and a standard request/response shape. Protobuf is not recommended
for v1 because binary encoding and browser tooling add complexity without solving
the harder problem: defining identical lifecycle behavior on every platform.

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

- `device.list`
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
when compatible with the existing attempt. The v1 capability may set
`maxConnections: 1` without baking that limitation into response shapes.

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

### Logging

- `logging.start({ deviceId, profile, intervalMs, destination? })`
- `logging.stop({ jobId })`
- `logging.list`
- `logging.get({ jobId })`
- `logging.listFiles`
- `logging.export({ logId, format })`
- `logging.delete({ logId })` — idempotent, explicit, and separately tested.

Events:

- `logging.jobChanged`
- `logging.storageWarning`

`logging.start` returns a durable `jobId`. A job state includes requested versus
effective interval, timestamps, sample/error counts, bytes written, background
capability, and last error. The backend persists job metadata before confirming
start. WebSocket disconnect and WebView destruction do not stop a job. App/OS
termination behavior is capability-defined and visible in later snapshots.

## Stable errors

Reserve project error codes and pair them with a string `data.kind`. Initial
families:

- `invalid_state`
- `unsupported`
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

## Migration plan

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
- Move direct `LocalServer.broadcastMessage` calls out of BLE/protocol classes.
- Run conformance scenarios in Android and iOS fake-adapter test hosts.
- Replace or fully harden the handwritten WebSocket framing.

### Phase 3: read-only device milestone

- Implement discovery, connection lifecycle, firmware, values, Refloat data, and
  bounded telemetry on both platforms.
- Require identical fixture and black-box results.

### Phase 4: durable logging/background behavior

- Add native log stores and jobs.
- Add Android foreground-service ownership and iOS background/restoration design.
- Test WebView loss, process/lifecycle transitions, disk pressure, and export.

### Phase 5: desktop

- Extract reusable Swift modules and add the macOS host.
- Add Linux/BlueZ and Windows/WinRT adapters after macOS conformance passes.

## Open decisions before implementation

1. Is v1 intentionally read-only, with configuration and firmware writes deferred?
   This proposal recommends yes.
2. Must logging resume after full app process termination, or only survive WebView
   and foreground/background transitions where the OS keeps the app alive?
3. What log format is canonical: a compact binary event log with CSV/JSON export,
   or a directly writable SQLite/columnar format?
4. Is one active board an acceptable advertised v1 limit, while retaining
   multi-device-shaped APIs?
5. Which VESC and Refloat firmware versions form the initial compatibility and
   hardware test matrix?
6. Should opaque device identity be session-only initially, or should the native
   backend provide a separately scoped persisted board identity?
