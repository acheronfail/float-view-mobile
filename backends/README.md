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
the current route and generation, exact audited VESC firmware versions, and exact
Refloat 1.0.x, 1.1.0–1.1.2, 1.2.0–1.2.3, or the 1.3.0 selectable wire profile. Test builds of audited VESC versions allow the same guarded writes as release builds;
unrecognised VESC and Refloat versions remain blocked by compatibility checks. One action may be pending for at most three seconds;
the request result and published state change only after matching telemetry or
lighting confirms it. Disconnect, reconnect, and generation changes cancel the
owner and its waiter.

Audited stable Refloat INFO-v2 versions also accept the upstream build artifact
whose suffix exactly repeats the patch number (for example, `1.3.0-0` or
`1.2.3-3`). The upstream `src/Makefile` suffix extraction passes an undelimited
stable patch through `cut`. Admission uses the canonical stable version; other
suffixes, including betas and release candidates, remain unsupported, as do
versions outside the audited catalog.

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

Tune application on supported Refloat 1.1.0 and later releases also permits
normal-mode ready/running boards with a known idle/riding state, including
engaged footpads. Refloat 1.0.0/1.0.1 keep the idle, disengaged-footpad rule:
their configuration handler reinitializes balancing. General configuration
writes retain the idle restriction on every version. Tune application retains
the same identity, revision, mutation lease, and verified-readback requirements.

Tunes are bounded application-wide records in the catalog database, keyed and
filtered by exact package version, schema fingerprint, and wire signature with
optimistic revisions. The schema cache lives in shared product storage and is keyed
by private connection identity, custom index, and a nonzero hardware CRC; every
hit is digest-checked, bounded, decoded, and audited again. Cache identity is
never public and never authorizes a write by itself. Both hosts select the shared
store; its canonical storage marker is defined in `core/product-store/src/lib.rs`.
Incompatible development databases currently follow the pre-release reset policy.

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
6. Run affected decoder, consumer and canonical conformance checks, then the
   relevant physical BLE interoperability checkpoint for the new wire version.
   Record unavailable required hardware; `just verify` is optional broader coverage.

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

## Ride timing diagnostics

Every newly recorded ride carries bounded diagnostic JSON in its SQLite
`session_diagnostic(sequence, record_json)` table. The normal `.floatview`
archive export includes it; no debugger or separate log export is required.
When a slowdown occurs, add a ride marker, finish the ride, and export it.
Markers capture recent request, BLE write, sample-sequence, and recording
progress traces, capped at 256 events and 30 seconds. Automatic captures are
limited to once per ten seconds; markers, reconnects, and ride close also capture.

Each record has version `v`, wall-clock `at_ms`, monotonic time since diagnostic
start `elapsed_ms`, an `event`, and structured `data`. Sample/marker elapsed
times inside data use the ride clock. Capture records precede their earlier
trace records in insertion order, so sort by timestamps when reconstructing
a timeline. Metadata includes the build's Git revision, firmware/package
versions, polling and persistence intervals. Native summaries supply platform
and OS version.

Ten-second summaries measure request queue and response time, BLE write progress,
notification sizes/counts, timer lateness, sample intervals, SQLite append and
catalog time, runtime work, and snapshot projection. Values named `_ms` use
milliseconds; `_us` use microseconds. `count_p50_p95_max` contains the full-window
count and maximum, with approximate percentiles of the most recent 256
observations. Request traces distinguish fast, slow, fault, BMS, configuration,
and other work. Native records count callbacks, queue depth/delay, admission
overflows, and exchange failures. UI reports measure snapshot count, maximum
delivery gap, sample age, and synchronous subscriber handling time; they do not
measure completed paint or animation latency. Background suspension and wall-clock
changes can affect these UI maxima.

Recording traces include cumulative accepted/stored counts and committed sequence
and elapsed time. Telemetry gaps, protocol diagnostics, clock/sequence
discontinuities, foreground changes, link transitions, and disconnect reasons
provide context. Normal idle polling can produce intervals above 500 ms; automatic
gap capture uses riding samples.

Persistence runs on a separate worker with eight pending batches of at most 32
records. It yields on database contention and drops diagnostics on overload or
failure, reporting cumulative `dropped_records` in subsequent summaries when
persistence recovers. The table keeps its first record and the newest 4,999
records, each at most 1,024 bytes. Older evidence is evicted on long rides.
Abrupt process termination can lose pending records. Diagnostics contain timing,
counts, closed status codes and versions, never raw BLE payloads, telemetry
values, coordinates, addresses, device identities, or authentication tokens.

For an exported archive, inspect the records with:

```sh
sqlite3 'file:ride.floatview?immutable=1' \
  'SELECT record_json FROM session_diagnostic ORDER BY sequence;'
```

The archive format is defined in `core/product-store/src/archive.rs` and the
owned product-storage marker in `core/product-store/src/lib.rs`. Under the current
pre-release policy, incompatible development storage is reset and unsupported
archive formats are rejected rather than migrated. The public-release plan
introduces baseline 100 and preservation guarantees before external testing.

Foreground auto-connect is owned by the shared Rust service. Each native
foreground visit can search for one locally selected saved board for 15 seconds
after readiness. This demand shares ordinary discovery without extending its
automatic eligibility. Manual connection/disconnect suppresses the opportunity
until the next visit; WebView/listener replacement does not rearm it.
