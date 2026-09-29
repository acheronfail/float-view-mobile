# Float View

Float View has one responsive SvelteKit frontend and a shared Rust backend with native OS hosts. The
frontend knows nothing about Apple or Android: every backend implements the
small hand-written contract in `frontend/src/lib/backend/contract.ts`.

## Repository

- `frontend/` — SvelteKit, TypeScript, Tailwind, the backend contract, client,
  deterministic mock, and UI tests. `npm run build` emits one self-contained
  `build/index.html`.
- `backends/apple/` — one Swift backend shared by the macOS executable and the
  thin iOS shell.
- `backends/android/` — one Android application module containing the backend
  and thin WebView shell.
- `backends/sql/` — the two current disposable-test-stage SQLite schemas.
- `backends/fixtures/` — immutable VESC and Refloat protocol fixtures.
- `conformance/` — the canonical black-box contract suite and programmable VESC
  simulator/orchestrator.
- `tools/vesc-test-device/` — pure ESP-IDF firmware, built with PlatformIO, that
  only bridges a serial line protocol to a fixed BLE GATT service.

Native hosts own Bluetooth, permissions, exports, lifecycle and the session
catalog. The shared Rust runtime owns protocol processing and durable session
work. `backends/core/loopback-server` owns authenticated HTTP/WebSocket transport,
complete state projection and historical responses through the existing C/JNI
binding families. Hosts provide embedded HTML and execute bounded OS capabilities; Rust owns
product storage, application policy, dispatch and complete state projection.

## Commands

```sh
just setup
just dev
just build
just run android|ios|macos [target]
just test frontend|conformance|apple|android|fixture|all
just conformance mock|android|ios|macos
just conformance-physical macos <fixture-port>
just conformance-physical android <fixture-port> <adb-serial>
just conformance-physical ios <fixture-port> <device-id>
just fixture setup|check|ports|build|flash|monitor|dev [port]
just verify
```

The fixture commands require PlatformIO Core (`pio`), but do not require a
separate ESP-IDF installation or a sourced ESP-IDF environment.

For manual native-Bluetooth iteration, flash the ESP32 and run
`just fixture dev [port]` beside `just dev <platform>`. The development-only
SvelteKit fixture controller is available at `http://127.0.0.1:11004`; it can
select direct or VESC Express topology, configure one CAN ID, choose the
THOR300 VESC/Refloat version composition, and switch power and riding state.
See `conformance/fixture-dev/README.md` for compatibility and ownership details.

`just verify` is the software gate. Physical BLE conformance is intentionally a
separate checkpoint because it requires a flashed ESP32 and real target devices.
Require it when a change affects OS adapters, permissions, lifecycle/background
behavior, radio timing, or firmware interoperability that software tests cannot
establish. Shared product, persistence, and frontend changes normally need only
the software gate. Record why physical testing is needed or skipped in the change.
macOS runs identity, ride, and reconnect scenarios; Android and iOS additionally
run mobile background-continuity coverage. VBMS physical checks cover each
available direct-controller/CAN-VBMS, Express/controller/VBMS, and direct-VBMS
gateway topology; unavailable hardware combinations are recorded rather than
replaced with weaker software coverage.

Supported BMS identities are deliberately exact: stock VESC BMS 6.00/type 1
and VESC Express `VBMS32` 6.06/type 2. See `backends/README.md` for the
compatibility and future-version audit policy and `conformance/physical-checkpoints/`
for retained hardware observations.
