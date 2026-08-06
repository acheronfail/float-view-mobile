# Native backend architecture

Apple and Android implement the authenticated loopback contract by hand. The
frontend is disposable; native application services own Bluetooth, protocol
requests, lifecycle, persistence, background work, and authoritative state.

## VESC request ownership

Each active Bluetooth connection has one bounded `VescRequestScheduler`. It is
the only production owner allowed to write framed VESC requests after UART setup,
keeps at most one request in flight, attributes responses to an exact route and
expected command, and rejects late work from older connection generations.
Controller identification has the highest priority, riding telemetry precedes
ordinary controller work, and auxiliary BMS traffic has the lowest priority.

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
4. Update strict Swift and Kotlin decoders together, including identical valid
   and malformed tables, bounds, error classification, and route checks.
5. Update the frontend contract/mock and canonical `conformance/` scenarios if
   public semantics change; do not add a generated or platform-specific API.
6. Run `just verify`, then the explicit physical BLE checkpoint without
   weakening software coverage for unavailable hardware.

Never broaden an existing entry by major version, hardware type, prefix match,
or successful command-96 probing. Add a new exact compatibility entry only
after all six audit steps pass.
