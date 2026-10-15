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
native adapters limited to closed OS facts, effects and completions. Product
CRUD, catalog/archive policy, reconnect decisions, waiters and public snapshot
construction belong in the shared Rust service.

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

## Verification

Run checks for changed code and affected consumers, following dependencies and
observable behavior rather than just the edited paths. Start with focused tests
and relevant type, lint, build, or integration checks; broaden only when the
impact or a failure warrants it. See [CONTRIBUTING.md](CONTRIBUTING.md#verification) for
commands and examples. Documentation-only changes need documentation validation,
not application builds or runtime suites.

A frontend-only change needs relevant frontend checks, including browser or mock
coverage when the affected interaction warrants it. It does not require Rust,
native packaging, simulator/emulator suites, or physical devices. Shared Rust
changes need affected crate and consumer tests; public contract changes also need
client/mock and canonical conformance coverage for affected host integrations.
Native-only changes need checks for the affected host and shared boundaries.

`just verify` remains an optional comprehensive software check for broad changes,
uncertain cross-cutting impact, or an explicitly requested full regression. It is
not a prerequisite for every change, commit, spec sync, or archive.

Require a physical checkpoint only for affected behavior that software cannot
establish, such as OS Bluetooth integration, permission presentation, background
execution, radio timing, or new firmware/wire interoperability. Scope it to the
relevant platforms, routes and scenarios; do not require unaffected devices or
the entire physical matrix. A UI change alone does not require physical BLE
verification. Keep fixture safety and restore checks when running hardware tests.

Briefly report checks run and their results. Explain omitted relevant coverage
or unavailable required hardware; do not claim unrun or software-only checks as
physical validation. No separate hardware-skip justification is needed for an
ordinary web, product-logic, persistence, or documentation change.

This policy governs ongoing work after the completed Rust migration. Full-suite,
all-platform and physical acceptance gates in historical migration/cutover specs
or archived changes apply only to those original changes, not subsequent edits.
Preserve historical evidence without treating it as a recurring checklist.
