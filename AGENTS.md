# Float View agent guide

Float View contains one SvelteKit frontend and native backends for Apple
(iOS/macOS) and Android. Keep the repository small and delete superseded paths;
there is no generated API layer or platform-specific frontend code.

## Pre-release compatibility policy

Float View has not shipped a release and has no users yet. Until the first
release is cut, persisted data and archives are disposable development data.
Do not add or retain database migrations, legacy identity aliases, old-format
readers, compatibility shims, or fallback paths solely to preserve earlier
development versions. Replace superseded code and reset incompatible owned
storage with an explicit format/schema marker when needed. Preserve current
data on ordinary corruption or operational failure; those are not version
mismatches. This policy does not remove support for intentionally supported
VESC/Refloat firmware wire families. Revisit compatibility guarantees when the
first release is cut, not when a development version number changes.

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
monotonic `state.changed` snapshots. The shared server implements that contract
by hand for Apple and Android. Contract changes must update the frontend client/mock, both backends,
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

`backends/core/` contains the selected shared application runtime and loopback
server, composed through the existing C/JNI binding families. Native hosts own
OS Bluetooth, location, lifecycle, permissions, native file presentation, theme,
and UI integration. Transport workers must reuse the existing application owner;
read-only session workers use bounded independent readers without creating a
second runtime. Keep public dispatch and projection in the shared server, and
native adapters limited to typed operations and complete domain facts.

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
