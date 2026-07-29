# Native backend structure

Apple and Android deliberately mirror ownership even though they implement the
wire contract by hand:

| Owner | Apple | Android |
| --- | --- | --- |
| Connection lifecycle and controller discovery | `apple/.../application/` | `android/.../backend/` |
| VESC frames, firmware identity, compatibility, and CAN routing | `apple/.../protocol/vesc/` | `android/.../protocol/vesc/` |
| Refloat compatibility and route-aware polling | `apple/.../protocol/refloat/` | `android/.../protocol/refloat/` |
| OS Bluetooth/GATT transport | `apple/.../bluetooth/` | `android/.../bluetooth/` |
| Durable products, private hints, and sessions | `apple/.../storage/` | `android/.../storage/` |

Dependencies point from application discovery to Bluetooth, VESC protocol,
Refloat protocol, and persistence. Protocol and Bluetooth owners emit typed
events and do not know about the loopback server or frontend.

`ControllerDiscovery.swift` and `ControllerDiscovery.kt` own the deterministic
connection generation, direct/CAN probe decisions, candidate bounds and
selection, compatibility negotiation, and stable discovery failures.
`TelemetryPolling.swift` and `TelemetryPolling.kt` own layout retries,
route-aware requests, sample sequencing, and adaptive polling cadence.
`ApplicationService` executes their effects, coordinates persistence and
sessions, and publishes authoritative contract state.

## Telemetry and sessions

Both hosts normalize the same catalog of motion, raw electrical load,
temperatures, rider input, control contributions, accumulated values, charging
values, semantic states, and bounded negotiated numeric extensions. Currents
remain distinct raw ampere values; neither host derives utilization. Dynamic
Refloat sources use command 31/32 while legacy 1.0.0 uses exact command 1.
Accumulated and charging values come from command 10 detail 4 once per second in
the same serialized polling stream. Command 35 and commands 41–43 are deferred.

The latest compact values are cached with their own observation time, merged
into realtime samples, and cleared whenever the selected route generation
changes. Device counters are trusted even when they decrease. Persistence keeps
first-class summary/query columns plus bounded detail and extension JSON.
Discrete state changes are appended atomically in stable catalog order and
session APIs return at most 2,000 numeric points and the newest 2,000 typed
events. CSV and archives omit Bluetooth and private controller identifiers.

Both discovery implementations are exercised by the same dependency-free
vectors in `fixtures/connection/controller-discovery.tsv`. Add lifecycle cases
there when a route-selection rule changes, then retain focused native tests only
for platform-specific Bluetooth, scheduling, and persistence behavior.

## Connection identity and privacy

A BLE peripheral is published during discovery only when it advertises the
VESC UART service or exactly matches a previously saved platform identity. Both
paths must still validate the complete UART GATT profile after connecting.

The public connection identifies the selected route as direct Bluetooth or CAN
with an ID. A meaningful 12-byte controller firmware UUID can be retained
privately to accelerate automatic rediscovery and prove reconnect continuity.
It must not be serialized into the API, errors, exports, or default logs.

The public Board Profile preference wins over the private learned hint. Changing
or clearing `preferredCanId` invalidates the UUID associated with the old route.
Selecting an ambiguous controller atomically merges only that preference into
the latest profile.

## Compatibility additions

Exact compatibility catalogs live beside their protocol:

- VESC controller firmware: 6.02, 6.05, and 6.06.
- Refloat source packages: 1.0.0 (legacy wire identity `1.0`), 1.1.0, and 1.2.3.

Add a catalog entry when a new release uses an already-covered wire family.
Add a new codec in the relevant protocol directory only when its framing,
identity, layout, or telemetry shape actually differs. Every addition needs the
same immutable fixture and focused tests on Swift and Kotlin.
