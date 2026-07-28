# Float View agent guide

## Product and architecture

Float View connects to VESC-based devices. One responsive Svelte UI is shared by
Android, iOS, and macOS. Native shells own Bluetooth, background execution,
durable logging, permissions, lifecycle, and the authenticated loopback server.
The UI talks only to the versioned JSON-RPC API at `/api/v1/ws`.

Keep these dependencies pointing downward:

1. host shell: window/WebView and lifecycle wiring;
2. loopback transport: assets, authentication, JSON, API negotiation;
3. application service: state machine, commands, subscriptions, logging jobs;
4. Bluetooth adapter: OS permissions, discovery, connection, GATT callbacks;
5. VESC protocol: framing, CRC, encoding, decoding, Refloat extensions;
6. persistence/background work: native SQLite and platform scheduling.

The WebView is disposable. Bluetooth and logging state belongs to native
services, never JavaScript. BLE/protocol code emits typed events to the
application service; it does not know about WebSockets or views.

## Repository

- `web/`: SvelteKit static SPA, typed API client/store, deterministic mock, API
  generators, and browser tests.
- `api/`: committed schemas, examples, AsyncAPI description, and generated
  TypeScript/Swift/Kotlin models. Never edit generated files directly.
- `conformance/`: platform-neutral black-box JSON-RPC scenarios.
- `apple/`: shared Swift core, protocol, Bluetooth, logging, loopback host, and
  macOS app. FlyingFox provides HTTP/WebSocket protocol handling.
- `ios/`: thin SwiftUI/WKWebView shell and Xcode project consuming `apple/`.
- `android/`: thin app plus Kotlin core, protocol, Bluetooth, logging, and Ktor
  loopback host.
- `logging/sql/`: one current catalog schema and one current session schema.
- `fixtures/`: sanitized VESC/Refloat byte fixtures shared across platforms.
- `tools/vesc-test-device/`: ESP32 VESC fixture and TypeScript scenario runner.

There is no database migration history while the product is test-stage. A schema
version mismatch deletes Float View's logging directory and recreates the current
schema. Preserve current-schema WAL recovery and integrity handling.

## Runtime

1. Build `web/build`.
2. The native app binds an ephemeral loopback port and creates a random launch
   token.
3. It serves the static UI and `/api/bootstrap`.
4. The WebView connects to `/api/v1/ws` with the token and validated Origin.
5. `system.hello` negotiates API v1; requests receive exactly one result/error.
6. Application-service events produce bounded `system.snapshotChanged`
   notifications; the client refreshes authoritative state.

Bind only to loopback. Keep origin/token validation and message/queue bounds.
Never log BLE addresses, launch tokens, or full telemetry by default.

## Domain rules

- Exactly zero or one board is actively connected.
- Device identifiers are opaque and platform-neutral.
- Backend jobs survive WebView recreation where the OS permits it.
- A ready connection automatically starts a durable `Session`; `jobId` is only
  active execution identity and `logId` must not return.
- Saved-device history survives forgetting a platform binding.
- Board Profiles are optional, versioned, app-local metadata. Editing them never
  writes VESC/BMS configuration.
- One sequenced native sample pipeline supplies live display, review, alerts,
  and lossless diagnosis. Do not create a UI-only logging path.
- Raw packets, firmware/configuration writes, and motor control are privileged;
  the programmable fixture and default tests remain read-only.

Prefer OS frameworks for JSON/Bluetooth/storage and focused maintained packages
for protocol infrastructure. Do not add an ORM, generic BLE wrapper, database
server, or cross-platform native UI runtime.

## Commands

From the repository root:

```sh
just setup
just dev
just run android [serial]
just run ios [device-id]
just run macos
just test [web|core|protocol|storage|all]
just vesc check
just vesc doctor
just vesc flash [port]
just vesc android|ios|macos [target] [scenario]
just verify
```

`just run` always builds the current web bundle. `just verify` is the sole
current software gate. Historical `verify-rN`, hardware-smoke, and T0–T2
harnesses are intentionally removed.

## Changes and tests

Treat committed schemas as the current wire-contract source of truth and VESC
Tool as the canonical protocol reference.

A contract change includes schemas/examples, regenerated models, mock behavior,
conformance assertions, and Swift/Kotlin support or an advertised unavailable
capability. Never patch generated source.

Use deterministic fakes for Bluetooth, clocks, persistence, and event sinks.
Keep protocol fixtures independent of BLE. Avoid sleeps in asynchronous tests.
Preserve unknown optional values or return a structured protocol error. Use
monotonic time for intervals and UTC timestamps for persisted/external time.
Bound buffers, queues, files, retry loops, and reconnect backoff.
