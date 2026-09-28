# Software fixture transport

Conformance owns `SoftwarePeripheral`, its independent protocol fixtures, and
`FixtureSession`. Direct Rust calls the dispatcher in process. Native test hosts
use `startFixtureRelay()` with standard HTTP clients. A future Linux test adapter
must implement this transport; it must not copy VESC interpretation.

Build explicitly for conformance, then pass the runner's endpoint, run ID, session
ID and independent 48-character random token. Only numeric `http://127.0.0.1:PORT/`
is valid. Android forwards that dedicated port with `adb reverse` and consumes a
private `files/fixture-run.json` at process startup. Ordinary Debug/Release sources
never read it. Apple requires `FLOAT_VIEW_CONFORMANCE`, Debug, and macOS or the iOS
simulator, plus `FLOAT_VIEW_FIXTURE_{ENDPOINT,TOKEN,RUN,SESSION}`. Physical launches
use ordinary builds. No production network policy or frontend route is added.

POST a JSON object to `/` with `Authorization: Bearer TOKEN`,
`X-Fixture-Session: SESSION`, and an exact Content-Length. Each object carries
`run`, `session`, `operation` (starts at 1, strictly consecutive), `generation`
(starts at 0, increments on each connect), and `command`. Commands are `scan`,
`stopScan`, `connect`, `disconnect`, `write`, `poll`, `reset`, `close`. Connect and
write carry `device`; write carries canonical base64 `bytes`. The runner creates
sessions rather than exposing a public creation endpoint. A run admits four.

Replies echo correlation fields and contain ordered `events`: `discovered`
(device, name, rssi), `link` (device, state), or `bytes` (device, base64 bytes).
Each event has its own monotonic sequence, originating operation and connection
generation. Polls return immediately, even when empty; adapters poll every 250 ms
for liveness. A view replacement must keep the process-owned adapter alive.
Disconnect clears partial frames; reconnect retains device configuration. Reset
explicitly resets configuration and framing. A new run has fresh credentials and
models. Never replay operations after an ambiguous network failure.

Bounds: 65,536 encoded bytes, 8,192 bytes per BLE payload, 64 pending commands,
128 queued events and 65,536 queued payload bytes per session. Validate before
allocating payloads. The dispatcher stages model changes before event admission.
Replies drain only what fits the encoded bound. Hosts serialize operations,
bound their queues, reject stale events, and finish or fail commands within five
seconds of admission. A transport failure must report connection loss, cancel
pending work and stop polling. Use `X-Fixture-Close: 1` for reserved cleanup;
runner shutdown also cancels all sessions and sockets independently of admission.
The relay rejects redirects, browser origins, non-loopback access and wrong tokens.

`test/software.test.ts` checks independent golden transcripts, malformed/fractured
streams, configuration reset, identity ordering, bounds, authentication, isolation
and saturation cleanup. Canonical host conformance checks saved telemetry,
configuration write/readback, reconnect, frontend replacement and harness loss.
These are software tests; explicit ESP32 runs provide OS Bluetooth coverage.
