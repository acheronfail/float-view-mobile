# Native backend architecture

Apple and Android implement the authenticated loopback contract by hand. The
frontend is disposable. Their native application services select the shared
Rust application runtime, execute its OS effects, and own Bluetooth, lifecycle,
background work, catalogue/file presentation, loopback transport, and
authoritative state publication.

Both native source trees use the capability names `application`, `bluetooth`,
`files`, `lifecycle`, `location`, `protocol`, `storage`, `theme`, `transport`,
and `ui` wherever they own that responsibility. See
[`apple/README.md`](apple/README.md) and [`android/README.md`](android/README.md)
for the build-system-specific source roots and exact mappings. Matching names
are navigation and ownership boundaries, not shared implementation layers.

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
optimistic revisions. The schema cache lives beside native storage and is keyed
by private connection identity, custom index, and a nonzero hardware CRC; every
hit is digest-checked, bounded, decoded, and audited again. Cache identity is
never public and never authorizes a write by itself. Apple and Android share
storage marker `11`; incompatible development databases are recreated.

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
