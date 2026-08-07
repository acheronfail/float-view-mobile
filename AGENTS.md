# Float View agent guide

Float View contains one SvelteKit frontend and native backends for Apple
(iOS/macOS) and Android. Keep the repository small and delete superseded paths;
there is no generated API layer or platform-specific frontend code.

## Researching definitive upstream source

Prefer using the following offline copies of upstream information relating to
the VESC spec, refloat package inner implementations, VBMS workings or generally
other platform related information.

Look in the `vendor/` directory, here there should be the upstream firmware and
package sources, for example:

- <https://github.com/vedderb/bldc>
- <https://github.com/lukash/refloat>
- <https://github.com/vedderb/vesc_tool>
- <https://github.com/vedderb/vesc_bms_fw>
- <https://github.com/vedderb/vesc_express>
- <https://github.com/acheronfail/float-view>

## Contract and ownership

`frontend/src/lib/backend/contract.ts` is the wire-contract source of truth.
Frontend code follows the ownership-first layout documented in
`frontend/src/lib/README.md`.
The UI uses only its authenticated request/result/error envelopes and complete,
monotonic `state.changed` snapshots. The Rust core implements that contract by
hand for every native host. Contract changes must update the frontend
client/mock, Rust core mapping, and canonical `conformance/` suite together.

The process-owned Rust runtime under `backends/core/` owns application state,
VESC/Refloat/BMS protocols, persistence, imports/exports, contract dispatch,
and the authenticated loopback server. Swift and Kotlin are thin hosts for OS
Bluetooth, location, lifecycle/background execution, permissions, resource and
document access, settings/export presentation, WebViews, and diagnostics.
WebViews are disposable.

Dependencies point downward from the C ABI/Android JNI and loopback boundaries
through the application actor to platform-port models, storage, and protocol
crates. Protocol code does not depend on Bluetooth implementations, SQLite,
HTTP, host interoperability, or views. Apple and Android each run one dedicated
host-command pump outside the UI thread; hosts copy bounded events and bytes
across the private bridge and never interpret protocol or frontend-contract
semantics.

Cargo and `backends/core/Cargo.lock` own portable dependencies. Direct
dependency responsibilities, versions, target evidence, licenses, and update
policy are recorded in `backends/core/DEPENDENCIES.md`; toolchains and generated
header provenance are recorded beside it. Normal builds use locked, offline
Cargo inputs and the checked-in header. Bind only to loopback, validate origin
and launch token, and bound messages, queues, collections, retries, and files.
Never log addresses, identifiers, coordinates, tokens, full telemetry, SQL
rows, or unbounded bridge payloads by default.

## Frontend interaction

Every interactive control must work through the same frontend on desktop and
touch clients. Prefer Pointer Events (`onpointerdown`, `onpointerup`, and
related cancellation handling) over separate mouse and touch handlers. In
frequently updating WebView UI such as menus, do not rely only on a synthesized
`click`; handle primary-pointer activation directly and retain a guarded
keyboard/assistive-technology click path without double activation. Keep native
button semantics, use `type="button"` outside form submission, apply an
appropriate `touch-action`, and test primary, non-primary, and keyboard
activation behavior.

## Retained paths

- `frontend/`
- `backends/apple/`
- `backends/android/`
- `backends/core/`
- `backends/sql/`
- `backends/fixtures/`
- `conformance/`
- `tools/vesc-test-device/`
- `openspec/`

The ESP32 tool is pure ESP-IDF firmware. Scenario logic, serial orchestration,
and VESC interpretation belong only to `conformance/`.

Use `just verify` as the software gate. Real Android/iOS devices and the flashed
fixture are required only for the explicit physical BLE checkpoint.
