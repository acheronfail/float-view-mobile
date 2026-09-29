# Native backend architecture

Apple and Android compose one process-owned Rust backend service. Native hosts
execute bounded OS capabilities: Bluetooth, permissions, location, lifecycle,
background execution, scoped file presentation, theme and UI. Rust owns product
storage/policy and authenticated loopback dispatch/projection. The frontend and
listener are disposable and borrow the existing application owner.

See [`core/README.md`](core/README.md), [`apple/README.md`](apple/README.md) and
[`android/README.md`](android/README.md) for the ownership and build boundaries.

## Startup presentation

The launch surface, native window/container, WebView and early HTML canvas use
`#f8fafc` (light) and `#020617` (dark), matching `frontend/src/app.css`.
iOS uses the adaptive `Canvas` asset for its system launch screen and SwiftUI
background. Android keeps its platform splash behavior, with matching window
and Android 12+ splash backgrounds. macOS shows its normal application window
with an adaptive canvas and applies the service's theme preference to AppKit.
Do not add minimum splash durations or hide the WebView while awaiting backend
connectivity; the frontend must remain able to show connection errors.

The OS launch surface follows system appearance before the application can read
its saved theme. Native theme effects and the early HTML theme hint then apply
the user's preference. Keep these canvas values aligned when changing the palette.

Focused real-browser startup checks run with `cd conformance && npm run startup`
after `just build`; install their development-only engines with
`cd conformance && npx playwright install chromium webkit`. Native macOS appearance
checks run with `just test apple`. Android canvas/recreation instrumentation runs
on the configured emulator with `cd conformance && npm run startup:android`.
These checks are included in `just verify`; recordings remain a separate visual
acceptance requirement.

For visual startup checks, cold-launch in system light and dark appearances,
then repeat with a saved opposite app theme. Check iOS background/foreground,
Android activity recreation and WebView teardown, and macOS window close/reopen.
The transition should contain no default white flash in dark mode or temporary
loading message. A loopback startup failure on iOS must still show an error.

## VESC request ownership

Each active Bluetooth connection has one bounded Rust `VescRequestScheduler`.
It is the only production decision owner allowed to emit framed VESC writes
after UART setup, keeps at most one request in flight, attributes responses to
an exact route and expected command, and rejects late work from older connection
generations. Native Bluetooth adapters execute those ordered writes.
Controller identification has the highest priority, riding telemetry precedes
ordinary controller work, and auxiliary BMS traffic has the lowest priority.

## Refloat product-control boundary

`DeviceMutationCoordinator` is the sole connection-scoped owner of semantic
package, flywheel, LED, headlight, and fatal-latch mutations. It authorizes only
the current route and generation, exact audited VESC release firmware, and exact
Refloat 1.0.x, 1.1.0–1.1.2, 1.2.0–1.2.3, or the 1.3.0 selectable wire profile. Audited VESC test
firmware remains read-only. One action may be pending for at most three seconds;
the request result and published state change only after matching telemetry or
lighting confirms it. Disconnect, reconnect, and generation changes cancel the
owner and its waiter.

Package disable and flywheel enable retain the authoritative idle gate. Exact
flywheel disable is the sole motion-state exception: active flywheel state is
normalized as riding, and upstream command 22 performs its ordered stop before
leaving flywheel mode. Every other mutation remains prohibited in that mode.

All writes and the one-second best-effort lighting read use the shared request
scheduler. Package, flywheel, and fatal-clear commands are one-way writes whose
completion is confirmed by later telemetry. Lighting uses the exact release
family response. While flywheel mode is active every mutation is rejected except
an otherwise eligible flywheel disable. Tunes and profiles, reverse stop, simple
start, recording/history, Handtest, remote/RC move, arbitrary configuration, and
motor-control commands are outside this allowlist and fail closed.

Refloat controls are live-only normalized state. They are never persisted,
restored, exported, or inferred by the frontend, and bounded diagnostics contain
no raw payloads or device identity.

## Refloat configuration boundary

Commands 92–95 use the same `VescRequestScheduler` and the same
`DeviceMutationCoordinator` as semantic controls and shutdown; there is no
direct or parallel write path. A connection-scoped configuration service
assembles bounded command-92 chunks, decodes the Qt envelope and strict XML,
matches the full schema fingerprint and CRC32C wire signature to an audited
exact Refloat release, and decodes complete command-93/94 values. Private custom
configuration count, hardware CRC, raw schema, and raw values stay below the
application boundary and out of logs.

Writes require the latest normal-mode current revision, exact identity, current
route/generation, stable firmware, idle ride state, disengaged footpad, and no
competing mutation. Command 95 acknowledgement is not success: a mandatory
command-93 readback must match the complete expected normalized values. A
disconnect after acknowledgement is indeterminate, and a mismatch publishes
fresh current state without claiming success. Flywheel mode rejects every
configuration write/Tune apply and withholds current reads; exit schedules a
fresh normal-mode current read before editing resumes. Shared semantic fields
are refreshed after verified writes rather than projected optimistically.

Tunes are bounded application-wide records in the catalog database, keyed and
filtered by exact package version, schema fingerprint, and wire signature with
optimistic revisions. The schema cache lives in shared product storage and is keyed
by private connection identity, custom index, and a nonzero hardware CRC; every
hit is digest-checked, bounded, decoded, and audited again. Cache identity is
never public and never authorizes a write by itself. Both hosts select the shared store with
storage marker `12`; incompatible development databases are recreated.

## Controller shutdown boundary

`controller.shutdown` is a live-only, accepted-for-transmission action owned by
the same `DeviceMutationCoordinator`. Stable writable VESC 6.05, 6.06, and 7.00
controllers may receive exactly `[156, 0, 0]`, either directly or once inside
`COMM_FORWARD_CAN`; VESC 6.02 and test builds remain unavailable. Authorization
requires fresh authoritative idle telemetry, normal (non-flywheel) mode, the
current route and generation, and no other pending device mutation.

The write is one-way and is never retried. A controller or Bluetooth loss while
the bounded shutdown intent is pending is classified as user-initiated and does
not enter automatic reconnect. If the selected controller remains healthy, the
intent expires and normal connectivity continues; this is the deterministic
path for hardware that ignores the command. State reports only transmission
acceptance and never claims power-off. Force, restart, broadcast, direct-CAN,
arbitrary payload, and repeated-pending variants are outside the allowlist.

Controller discovery owns controller compatibility, route selection, readiness,
and ride-session continuity. It also publishes a bounded, structurally decoded
hardware inventory after a complete CAN enumeration. `AuxiliaryNodeDiscovery`
may reuse that inventory or run its own low-priority enumeration, but owns only
VBMS candidates, preference resolution, liveness, and BMS status. A local VBMS
uses the direct route while controller discovery continues through CAN.

## VBMS protocol boundary

BMS capability is fail-closed and keyed by exact audited identity, not hardware
type alone:

| Upstream family      | Firmware identity                                          | Support                                                                 |
| -------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------- |
| `vesc_bms_fw`        | version 6.00, `HW_TYPE_VESC_BMS` (`1`)                     | Supported                                                               |
| VESC Express VBMS32  | name `VBMS32`, version 6.06, `HW_TYPE_CUSTOM_MODULE` (`2`) | Supported                                                               |
| VESC Express VBMS32  | name `VBMS32`, version 7.00                                | Rejected pending a new source, decoder, conformance, and physical audit |
| Other custom modules | any other tuple                                            | Not a BMS candidate                                                     |

VESC Express intentionally emits type `2` from both `COMM_FW_VERSION` and CAN
pong for VBMS32 even though its command dispatcher implements BMS command 96.
Firmware identity parsing preserves the private 12-byte UUID for in-memory
generation deduplication, but UUIDs are never serialized or logged.
`COMM_BMS_GET_VALUES` (96) is read-only and is routed as exact direct `[96]` or
CAN `[34, id, 96]` payloads. Decoders accept only the source-defined exact
legacy body or the exact data-version plus bounded, NUL-terminated UTF-8 status
extension. Counts, flags, finite values, route ID, status length, and remainder
are validated before a complete snapshot is made.

BMS values are live-only. They appear in authoritative state and conflated
`state.changed` snapshots, but never enter telemetry samples, SQLite telemetry
tables, session counts or aggregates, diagnostics, review data, or exports.
Unavailable state may retain a stale snapshot with its original observation
time. Normal diagnostics contain only bounded route/status codes—not UUIDs,
raw packets, full cell arrays, tokens, or Bluetooth addresses.

## Auditing another VBMS wire version

1. Pin the relevant `vesc_bms_fw`, `vesc_express`, VESC firmware, and VESC Tool
   sources under `vendor/` and identify the exact source commit.
2. Audit the firmware common prefix, hardware-type tail, command 96 field order,
   fixed-point scales, auto-float behavior, and optional tail byte for byte.
3. Add immutable source-derived fixtures with provenance under
   `backends/fixtures/vesc-bms/`.
4. Update the shared Rust decoder and source-derived vectors, including valid
   and malformed tables, bounds, error classification, and route checks.
5. Update the frontend contract/mock and canonical `conformance/` scenarios if
   public semantics change; do not add a generated or platform-specific API.
6. Run `just verify`, then the explicit physical BLE checkpoint without
   weakening software coverage for unavailable hardware.

Never broaden an existing entry by major version, hardware type, prefix match,
or successful command-96 probing. Add a new exact compatibility entry only
after all six audit steps pass.

For connected-iPhone startup acceptance, the existing UI test target includes
`FloatViewLifecycleTests/testStartupAndForegroundPresentation`. It checks three
cold launches and foreground reconstructions with visible frontend navigation
and no stale native startup error. Run it against the connected device with
`xcodebuild test -project backends/apple/ios/float-view.xcodeproj -scheme float-view -destination 'platform=iOS,id=<device-id>' -allowProvisioningUpdates -only-testing:float-view-lifecycle-tests/FloatViewLifecycleTests/testStartupAndForegroundPresentation`.
Retain XCTest screen recordings by setting `SystemAttachmentLifetime` to
`keepAlways` in the generated `.xctestrun` when collecting visual evidence.
These physical UI observations are separate from the software gate and do not
require an ESP32/BLE fixture.
