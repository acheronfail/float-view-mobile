# Float View

Float View has one responsive SvelteKit frontend and three native backends. The
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

Native backends own Bluetooth, protocol decoding, durable sessions, permissions,
exports, lifecycle, and the authenticated loopback server. They serve only the
embedded HTML at `/`, bootstrap data at `/api/bootstrap`, and the contract
WebSocket at `/api/ws`.

## Commands

```sh
just setup
just dev
just build
just run android|ios|macos [target]
just test frontend|conformance|apple|android|fixture|all
just conformance mock|android|ios|macos
just fixture setup|check|ports|build|flash|monitor [port]
just verify
```

The fixture commands require PlatformIO Core (`pio`), but do not require a
separate ESP-IDF installation or a sourced ESP-IDF environment.

`just verify` is the software gate. Physical BLE conformance is intentionally a
separate checkpoint because it requires a flashed ESP32 and real target devices.
