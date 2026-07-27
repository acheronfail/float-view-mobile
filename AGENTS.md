# Float View agent guide

## Product intent

Float View is a cross-platform toolkit for connecting to VESC-based devices, with
Onewheel-style boards and Refloat firmware as the first use case.

The product architecture is deliberate:

- One responsive Svelte UI should provide the same experience everywhere.
- Each platform shell should stay thin: host the web UI and start/stop the native
  backend, but contain little product logic.
- Bluetooth, background execution, logging, durable storage, permissions, and
  platform lifecycle handling must remain native so each OS can use its strongest
  APIs and iOS can remain especially resource-efficient.
- The UI talks only to a versioned, platform-neutral API served on the local
  device. It must not know about CoreBluetooth, Android GATT, BlueZ, WinRT, or
  platform-specific device identifiers.
- iOS and Android are the first supported backends. macOS is next; Linux and
  Windows are intended later.
- Every backend must be testable with fake Bluetooth devices and against the same
  black-box conformance suite. A feature is not cross-platform merely because it
  has the same method name.

The proposed API and test architecture is in
[`docs/api-v1-proposal.md`](docs/api-v1-proposal.md). Treat it as a proposal until
its open decisions are resolved, then promote the accepted contract into an
`api/` source-of-truth directory.

## Repository map

- `web/`: Svelte 5/SvelteKit static SPA, API schemas/client, mock backend, tests,
  and the current TypeScript-to-native generators.
  - `web/src/lib/backend-client.types.ts`: current Zod command/message contract.
  - `web/src/lib/backend-client.ts`: current same-origin `/ws` client.
  - `web/src/lib/vesc.commands.ts` and `refloat.commands.ts`: current VESC and
    Refloat wire models.
  - `web/backend-mock/index.ts`: development WebSocket backend with simulated
    devices and telemetry.
  - `web/scripts/generate.ts`: generates native command/message types.
  - `web/build/`: ignored static build output consumed by both native apps.
- `android/`: Kotlin Android shell and native backend.
  - `MainActivity.kt`: WebView/bootstrap and current command dispatch.
  - `LocalServer.kt`, `LocalWebSocket.kt`, `LocalWebContext.kt`: embedded HTTP and
    WebSocket implementation.
  - `VescBle.kt`: permissions, scanning, one active GATT connection, service
    discovery, and notification handling.
  - `VescBlePacketLayer.kt`, `VescBuffer.kt`, `VescCommandGenerator.kt`, and
    `VescCommandParser.kt`: VESC framing, encoding, and decoding.
  - `android/app/src/main/assets/web` is a tracked symlink to `web/build`.
- `ios/`: SwiftUI iOS shell and native backend.
  - `FloatViewApp.swift` and `ContentView.swift`: startup, command dispatch, and
    WKWebView host.
  - `LocalServer.swift`, `LocalWebSocket.swift`, `LocalServerTypes.swift`: embedded
    HTTP and WebSocket implementation.
  - `VescBle.swift`: CoreBluetooth scanning, one active connection, service
    discovery, and notification handling.
  - `VescBlePacketLayer.swift`, `VescBuffer.swift`,
    `VescCommandGenerator.swift`, and `VescCommandParser.swift`: VESC framing,
    encoding, and decoding.
  - Xcode includes `../web/build` as the `web` folder resource.
- `justfile`: repository-level setup, generation, web watch, and device launch
  shortcuts.

There is currently no desktop shell.

## Current runtime flow

1. Build the SvelteKit app as static files in `web/build`.
2. The native app starts an HTTP server bound to an ephemeral loopback port.
3. That server serves `index.html` and its static assets.
4. Android WebView or iOS WKWebView opens the loopback origin.
5. The UI opens a same-origin WebSocket at `/ws`.
6. JSON commands are decoded into generated Swift/Kotlin types, dispatched to
   `VescBle`, framed as VESC packets, and written over BLE.
7. BLE notifications are decoded and broadcast as JSON messages to all connected
   WebSocket clients.

The browser-only development path runs Vite and
`web/backend-mock/index.ts`; Vite proxies `/ws` to the mock on port 3001.

## Source-of-truth and generated code

Today, the Zod definitions under `web/src/lib/` are the contract source. Running
`just generate` creates:

- `ios/float-view/Generated_*.swift`
- `android/app/src/main/java/fail/acheron/floatview/Generated_*.kt`

Those files are intentionally ignored. Never hand-edit them. Change the Zod
schemas or generator, run generation, and then build both native targets.

The current generator is narrow and assumes specific Zod JSON Schema shapes.
Exercise care with new unions, optional fields, booleans, arrays, maps, and nested
objects; add generator tests before relying on a new shape.

The target architecture should become spec-first: versioned JSON Schemas and an
AsyncAPI document in `api/`, with generated TypeScript, Swift, and Kotlin models.
Do not create a second hand-maintained definition of the same API.

## Architecture boundaries to preserve

Organize each native backend into the same conceptual layers, even when platform
framework details differ:

1. **Host shell**: window/WebView and lifecycle wiring only.
2. **Loopback server**: static assets, WebSocket protocol, authentication,
   serialization, and API version negotiation.
3. **Application service**: platform-neutral device state machine, command
   dispatch, telemetry subscriptions, logging jobs, and error mapping.
4. **Bluetooth adapter**: permissions, radio state, discovery, connection,
   restoration/reconnection, GATT reads/writes, and OS callbacks.
5. **VESC protocol**: packet framing, CRC, command encoding, response decoding,
   and Refloat extensions.
6. **Persistence/background work**: native logs, job metadata, settings, and
   platform background scheduling.

Dependencies point downward. BLE and protocol code must not broadcast directly to
the global WebSocket server in the target design; emit typed results/events to the
application service instead. The WebSocket is a presentation transport, not the
owner of connections or logging.

Use injected interfaces for Bluetooth, clocks/timers, persistence, and event
sinks. Production implementations call platform APIs; tests use deterministic
fakes. Avoid singletons and direct `Activity`, `View`, or `LocalServer` references
below the host/adapter boundary.

Keep VESC protocol fixtures independent of BLE. A recorded notification byte
stream should be decodable in a unit test without a phone or Bluetooth radio.

## Cross-platform API rules

- Expose product/domain operations, not platform framework concepts.
- Treat `deviceId` as an opaque string. Do not expose an Android MAC address or
  assume an iOS peripheral UUID has Android semantics.
- Every request has a correlation ID and exactly one success or error response.
  Unsolicited lifecycle/telemetry updates are events, not ad-hoc responses.
- Errors use stable machine-readable codes plus a human-readable message and
  optional structured details. UI behavior must never depend on matching message
  text.
- Specify state transitions, ordering, idempotency, cancellation, timeouts, and
  behavior after WebSocket/UI reconnection. Matching JSON shapes alone is
  insufficient.
- The backend owns long-running work. Logging must continue without a live
  WebSocket or active WebView where the OS permits it, and the UI must be able to
  query current jobs/state after reconnecting.
- Telemetry is a bounded subscription with an explicit rate and backpressure/drop
  policy. Never let a slow WebView create an unbounded native queue.
- Version the protocol with semantic major/minor versions. Reject incompatible
  majors during `system.hello`; advertise capabilities for optional platform or
  firmware features.
- Design collection-shaped responses even if v1 supports only one active BLE
  connection.
- Bind only to loopback, validate the WebSocket `Origin`, and require an
  unguessable per-launch session token. Do not expose a LAN server by default.
- Keep the static HTTP surface small. Use the WebSocket for API requests,
  responses, and events; reserve ordinary HTTP for assets, health/bootstrap, and
  test control in explicit test builds.
- Raw VESC packet access, firmware writes, configuration writes, and motor control
  are privileged capabilities. Add explicit validation and safety policy before
  exposing them to the UI.

## Platform constraints

### iOS and macOS

- Use CoreBluetooth for BLE. iOS background modes/restoration and native
  persistence must be designed around OS lifecycle rules; the WKWebView may be
  suspended or destroyed.
- Keep polling, decoding, and log writing out of the WebView. Rate-limit UI events
  separately from native log sampling.
- Do not assume an app can run arbitrary timers forever in the background.
  Surface capability/actual job state honestly.
- macOS should reuse the Swift application-service, CoreBluetooth, and protocol
  modules while replacing only the host/lifecycle pieces.

### Android

- Permission behavior differs before and after Android 12; preserve explicit
  radio/permission states in the API.
- Background BLE/logging belongs in an appropriate foreground service with a
  notification when required by Android.
- Do not let an `Activity` own a connection or logging job that must survive
  recreation.

### Future Linux and Windows

- Implement the same Bluetooth adapter boundary with BlueZ D-Bus and WinRT,
  respectively.
- Do not encode mobile-only lifecycle assumptions into the contract.

## Known current limitations

- Android and iOS support a single connected device in practice, despite
  address-bearing messages.
- `logStart` and `logStop` are mock-only; both native dispatchers contain TODOs.
- Native BLE/protocol layers publish directly through `LocalServer`, making them
  difficult to unit test.
- The API is fire-and-forget and message-kind based: it lacks request IDs,
  deterministic request completion, formal error codes, negotiation,
  capabilities, and reconnect snapshots.
- The handwritten WebSocket servers parse one read as one complete text frame and
  do not fully implement fragmentation, continuation, ping/pong, close frames, or
  stream reassembly. Replace or harden this before treating it as infrastructure.
- Android notification setup does not currently write/confirm the CCCD in the
  same explicit way as iOS, and several connection error/disconnect paths do not
  produce a consistent terminal event.
- Protocol coverage differs between Swift and Kotlin; some parsers and CRC checks
  are incomplete.
- After current schema generation, Android compilation fails in
  `Generated_BackendMessage.kt` where generated builders call methods on nullable
  `UInt` fields. Fix and regression-test the generator rather than patching the
  ignored output.
- The UI installs multiple page-local WebSocket listeners and stores only minimal
  connection state. A central typed client/store should own reconnect,
  subscriptions, request promises, snapshots, and event routing.
- Native tests and most web tests are placeholders. There is no iOS test target or
  cross-platform contract/conformance suite. The current Swift package also
  attempts to compile iOS-only app sources for the macOS host when running
  `swift test`, so it is not a valid test harness yet.

Do not silently build new features on these limitations. Either fix the relevant
boundary or document the temporary compromise and cover it with a regression
test.

## Testing expectations

Use the test pyramid described in `docs/api-v1-proposal.md`:

- schema examples and generated-code drift checks;
- pure VESC framing/parser golden tests against a shared fixture corpus;
- application-service state-machine tests with fake Bluetooth, clock, and storage;
- the same black-box API conformance suite against the mock, Android, iOS, and
  later desktop backends;
- platform integration tests for real framework behavior;
- opt-in hardware-in-the-loop tests with identified VESC fixtures.

For asynchronous tests, use a fake clock and scripted events instead of sleeps.
Record fixtures as raw bytes plus metadata and expected normalized events; remove
personal device identifiers. Hardware tests must be explicitly selected and must
not send configuration, firmware, or motion-affecting commands by default.

Any contract change should include:

1. schema and examples;
2. regenerated models with a no-diff check;
3. mock behavior;
4. conformance assertions;
5. Swift and Kotlin implementation or an advertised capability indicating that
   the feature is unavailable;
6. migration/versioning notes if compatibility changes.

## Development commands

From the repository root:

```sh
just setup       # npm install, Gradle dependency resolution, generation
just generate    # regenerate ignored Swift/Kotlin API models
just wdev        # Vite UI + mock backend
just wwatch      # continuously rebuild web/build for native development
just adev        # install/launch Android debug app; requires wwatch
just idev        # build/launch iOS simulator app; requires wwatch
```

Focused checks:

```sh
cd web && npm run check
cd web && npm run lint
cd web && npm test
cd android && ./gradlew test
cd ios && swift test
```

`npm test` currently needs the Playwright Chromium build matching the lockfile
(`npx playwright install chromium`). `swift test` is not yet a reliable iOS app
test command because the package has no test target and app sources import
iOS-only frameworks; prefer the Xcode scheme once test targets are added.

When changing bundled UI assets, run a production web build before the native
build. The native apps do not build the web project themselves.

## Working practices

- Read this file and `docs/api-v1-proposal.md` before changing boundaries or the
  wire contract.
- Keep changes narrow; do not mix an API migration, BLE rewrite, and UI redesign
  without independently testable stages.
- Prefer explicit state machines and typed events over callback-driven booleans.
- Preserve unknown enum/event values when practical or fail with a structured
  protocol error; never crash on a newer optional capability.
- Use monotonic time for intervals/timeouts and UTC timestamps for persisted or
  externally visible times.
- Include units in schema field names or descriptions. Normalize them at the API
  boundary (for example, volts, amps, degrees Celsius, ERPM, and milliseconds).
- Bound buffers, queues, log files, retry loops, and reconnect backoff.
- Never log BLE addresses, session tokens, or full board telemetry by default in
  release builds.
- Keep native UI minimal. Product UI belongs in `web/`; permission prompts,
  background notifications, and OS-required surfaces remain native.
