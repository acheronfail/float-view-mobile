# Float View agent guide

Float View contains one SvelteKit frontend and native backends for Apple
(iOS/macOS) and Android. Keep the repository small and delete superseded paths;
there is no generated API layer or platform-specific frontend code.

## Contract and ownership

`frontend/src/lib/backend/contract.ts` is the wire-contract source of truth.
Frontend code follows the ownership-first layout documented in
`frontend/src/lib/README.md`.
The UI uses only its authenticated request/result/error envelopes and complete,
monotonic `state.changed` snapshots. Apple and Android implement that contract
by hand. Contract changes must update the frontend client/mock, both backends,
and the canonical `conformance/` suite together.

Native services own Bluetooth, background work, logging, persistence,
permissions, exports, and lifecycle. WebViews are disposable. Bind only to
loopback, validate origin and launch token, and bound messages, queues,
collections, retries, and files. Never log addresses, tokens, or full telemetry
by default.

Dependencies point downward: host shell; loopback transport; application
service; OS Bluetooth adapter; VESC protocol; persistence/background work.
Bluetooth and protocol code emit typed events and do not know about WebSockets
or views.

## Retained paths

- `frontend/`
- `backends/apple/`
- `backends/android/`
- `backends/sql/`
- `backends/fixtures/`
- `conformance/`
- `tools/vesc-test-device/`
- `openspec/`

The ESP32 tool is pure ESP-IDF firmware. Scenario logic, serial orchestration,
and VESC interpretation belong only to `conformance/`.

Use `just verify` as the software gate. Real Android/iOS devices and the flashed
fixture are required only for the explicit physical BLE checkpoint.
