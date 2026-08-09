## Context

The direct-and-CAN controller change establishes an explicit selected controller route, bounded sequential CAN probing, strict motor-controller firmware identity parsing, route-aware Refloat polling, and complete authoritative connection snapshots. That discovery currently ends as soon as it can select a motor controller: a direct controller skips CAN ping, a configured or privately hinted gateway probes only the selected controller, and automatic enumeration discards every non-motor identity. `TelemetryPolling` serializes only its own Refloat requests rather than arbitrating every request written to the uncorrelated VESC stream.

Upstream assigns hardware types `HW_TYPE_VESC = 0`, `HW_TYPE_VESC_BMS = 1`, and `HW_TYPE_CUSTOM_MODULE = 2`, but hardware type is not a complete capability taxonomy. Stock `vesc_bms_fw` 6.00 reports type 1. The official VESC Express VBMS32 target reports hardware name `VBMS32`, version 6.06, and type 2 from both `COMM_FW_VERSION` and CAN pong while routing `COMM_BMS_GET_VALUES` to its BMS implementation. A supported BMS can be the local BLE endpoint or a distinct CAN node. Command 96 is sent directly as `[96]` or routed as `[COMM_FORWARD_CAN, canId, 96]`. A direct BMS response establishes current liveness; a motor controller can answer the same command from a cache whose age is not exposed, so Float View will not use the cached controller response as attachment evidence or automatic disambiguation.

The stock VESC BMS identity ends after the common firmware prefix, UUID, pairing marker, test version, hardware type, and custom-configuration count. The VBMS32 identity uses the VESC Express custom-module tail with four capability bytes, a terminated firmware name, and a hardware CRC. The strict decoder parses both structures, while auxiliary discovery decides BMS capability from the exact audited identity tuple rather than the enum alone. The values response contains variable-length cell and temperature arrays and has a legacy form ending after pressure plus an extended form with data version and a bounded status string.

This change follows the controller-routing change and must be reconciled against its delta specs before archive. The frontend contract remains the source of truth; Apple and Android implement it by hand; conformance owns simulation and scenario behavior; and the ESP32 remains a byte bridge.

## Goals / Non-Goals

**Goals:**

- Discover a local or CAN-routed stock VESC BMS 6.00 or VESC Express VBMS32 6.06 independently from the selected motor controller.
- Let a unique VBMS be selected automatically and let `preferredBmsCanId` select one exact CAN route without relying on CAN ping.
- Report multiple unconfigured VBMS nodes as ambiguous and poll none of them.
- Query the selected VBMS directly, enforce source-derived bounds, and expose complete live values and freshness.
- Keep controller readiness, Refloat telemetry, session creation, reconnect continuity, and riding operation independent of BMS availability.
- Serialize BMS discovery and polling with all other VESC requests and keep its cadence subordinate to controller telemetry.
- Present detected BMS information through one conditional shared-frontend menu action and sheet/popover.
- Provide matching Swift/Kotlin ownership and deterministic software and physical conformance coverage.

**Non-Goals:**

- Persisting BMS values into session samples, diagnostics, summaries, databases, or exports.
- Supporting generic BLE battery services, smart-BMS protocols, or non-VESC BMS commands.
- Sending BMS configuration, balancing, reset, charging, bootloader, or firmware-write commands.
- Automatically selecting the lowest CAN ID or using controller-cached BMS values to choose among multiple VBMS nodes.
- Making BMS discovery or health a prerequisite for application `ready`.
- Exposing stable BMS UUIDs in frontend state, logs, errors, or exports.

## Decisions

### 1. Keep controller selection and auxiliary BMS discovery as separate lifecycles

Both backends will introduce an `AuxiliaryNodeDiscovery` owner for one connection generation. It consumes the already classified local firmware identity and any complete CAN inventory produced by controller discovery, and can perform its own bounded ping and sequential firmware probes when that inventory does not exist.

```text
UART ready
   │
   ├─ local firmware classification ───────────────┐
   │                                               │
   ├─ controller discovery ─▶ Refloat ─▶ ready     │
   │                                               ▼
   └──────────────────────────────▶ auxiliary BMS discovery
                                                   │
                  ┌──────────────┬─────────────────┼──────────────┐
                  ▼              ▼                 ▼              ▼
               not_found      attached         ambiguous      unavailable
```

Controller discovery continues to own controller candidates, compatibility, preferences, failure, readiness, and session continuity. Auxiliary discovery owns only BMS candidates, route resolution, BMS liveness, and BMS status. It can begin during controller identification when a reusable inventory exists, but it never delays controller effects. If the controller reaches `ready` first, auxiliary work continues in the background.

A generic internal `NodeRoute = direct | can(id)` will own command forwarding semantics. `ControllerRoute` may become an alias/wrapper or remain the controller-domain projection, but the public `controllerConnection` and BMS connection fields remain distinct so one route can never overwrite the other.

Extending `ControllerDiscovery` to select both devices was rejected because direct, preferred, hinted, ambiguous, reconnect, and failure rules differ, and a missing BMS must not turn a valid controller connection into a failed one.

### 2. Build a bounded typed inventory and perform an auxiliary pass only when necessary

Firmware classification retains the local identity and at most 64 normalized unique CAN IDs in `0...253`, probed sequentially. It records at most 16 VBMS candidates independently from the existing 16-controller bound. Overflow produces an unavailable BMS state with a bounded diagnostic and no partial automatic selection; it does not fail the controller connection.

The local endpoint is handled first:

- A local identity matching the audited BMS compatibility catalog becomes the direct BMS candidate and is selected without CAN ambiguity. Controller discovery still treats that endpoint as a gateway and discovers the motor controller behind it.
- A local motor controller, generic Express target, or arbitrary custom module is not a BMS candidate. VBMS32 is the narrow exception keyed by exact hardware name, type, and wire version.

For CAN candidates:

- If `preferredBmsCanId` is set and the local endpoint is not itself a VBMS, probe that route directly without requiring ping to advertise it. A timeout, malformed identity, or non-BMS identity produces `unavailable`; Float View does not fall back to another BMS.
- Without a preference, reuse a complete controller-discovery inventory when available. Otherwise perform a low-priority auxiliary ping and sequential firmware pass after the controller route is established.
- Zero VBMS identities produces `not_found`; one selects that route; more than one produces `ambiguous` and sends no values request.

The selected controller's cached `COMM_BMS_GET_VALUES` response is deliberately not queried to disambiguate multiple nodes. It cannot expose cache age, and the user chose an explicit fail-ambiguous rule.

Changing `preferredBmsCanId` on the active board cancels only the current auxiliary generation and immediately probes the new exact route or resumes automatic discovery when cleared. It does not reconnect the controller, interrupt its session, or modify `preferredCanId`. The preference applies on subsequent connect and reconnect generations as well. A local directly connected VBMS remains authoritative because the CAN preference does not describe a direct route.

### 3. Split common firmware parsing from hardware-specific identity tails

The VESC firmware decoder parses a common bounded prefix containing command, version, terminated hardware name, 12-byte UUID, pairing marker, test version, hardware type, and custom-configuration count. It then delegates by wire layout:

- `HW_TYPE_VESC` requires the audited motor-controller flags, terminated firmware name, and allowed optional CRC tail exactly as today.
- `HW_TYPE_VESC_BMS` 6.00 requires the source-defined identity to end after custom-configuration count.
- Custom-module identities require the audited capability bytes, terminated firmware name, and optional CRC tail. Only the exact `VBMS32`, type 2, version 6.06 tuple is entered in the BMS compatibility catalog.
- Other structurally valid hardware identities remain available for gateway classification but are never accepted as a controller or BMS.

This preserves fail-closed controller compatibility while allowing source-faithful stock VBMS, VBMS32, and gateway identities. A permissive parser or generic "custom module with command 96" probe was rejected because malformed or mutable modules could be misclassified as trustworthy battery routes.

Meaningful BMS UUIDs use the canonical lowercase-hex representation and may be exposed or persisted wherever application identity is useful. `preferredBmsCanId` remains authoritative for route selection.

### 4. Decode exactly bounded legacy and extended BMS value variants

The new command enum entry is `COMM_BMS_GET_VALUES = 96`. The response decoder requires command 96, six scaled 32-bit values, a cell count no greater than 50, exactly that many cell voltages and boolean balancing bytes, a temperature count no greater than 50, exactly that many temperatures, and the fixed remaining values through pressure.

For `c` cells and `t` temperature sensors, the complete legacy payload length including the command is `58 + 3c + 2t`. After the fixed pressure field, the decoder accepts either:

- no remaining bytes; or
- one data-version byte followed by one NUL-terminated UTF-8 status of at most 40 content bytes, with no remainder.

Counts are validated before allocation or iteration. Every scaled numeric value must be finite; balancing bytes must be `0` or `1`; the returned CAN ID must be `0...253`; and a CAN-routed response must report the selected CAN ID. A malformed response is discarded, records a bounded diagnostic, and contributes to the same bounded unavailability threshold as a timeout. No partial snapshot is published.

The public `BmsSnapshot` contains its observation time, total and charge voltage, input currents, session Ah/Wh counters, cell voltages and balancing flags, sensor temperatures, IC and humidity-sensor temperatures, humidity, highest cell temperature, state of charge and health, reported CAN ID, lifetime charge/discharge Ah/Wh, pressure, nullable data version, and nullable bounded status. Units and nullable behavior are fixed in the contract.

### 5. Use one connection-wide request scheduler

The native protocol/application boundary will gain one `VescRequestScheduler` (matching name per platform) as the sole owner of post-UART VESC writes, deadlines, and the one-request-in-flight invariant. Controller discovery, Refloat layout, Refloat telemetry, current-limit probes, auxiliary discovery, and BMS polling submit typed bounded work rather than writing independently.

Requests are prioritized as follows:

1. connection-generation controller identification and reconnect validation;
2. due riding Refloat telemetry;
3. Refloat layout and other controller work;
4. auxiliary discovery and BMS polling.

The scheduler records the expected response command and request context, including route and firmware-probe target, so responses cannot inherit a stale global `pendingFirmwareCanId`. Timeouts complete the matching work item and allow the queue to advance. Queues, retries, diagnostics, and consecutive failures are bounded.

The BMS poll owner targets approximately one successful sample per second but skips a due poll while higher-priority work is queued. A successful directly routed response sets `attached` and resets the failure count. Three consecutive selected-route timeouts or malformed responses set `unavailable` while retaining the last snapshot and its timestamp. A later valid response recovers `attached`. This cadence and threshold are product policy, not proof derived from controller cache freshness.

Keeping an independent BMS timer that writes beside `TelemetryPolling` was rejected because it would violate the documented serialized-stream invariant and could misattribute firmware responses.

### 6. Publish one bounded live-only BMS state

`ActiveConnection` gains a complete `bms` value with:

```text
status: discovering | not_found | attached | unavailable | ambiguous
connection: null | direct | can(id)
candidates: bounded display-safe BMS candidates
latestSnapshot: null | complete BmsSnapshot
```

Candidates contain route, canonical BMS UUID when meaningful, bounded hardware name, and firmware version. `connection` is non-null only after a unique or preferred identity is verified. `latestSnapshot` is replaced atomically after a complete successful decode. It is retained with its original timestamp across bounded poll failures and an active-connection reconnect, but cleared when the active device is removed or replaced.

BMS state changes increment the authoritative state revision and use the existing conflated `state.changed` delivery. Unlike controller samples, BMS samples are not gated on or appended to session persistence. They do not change active-session counts, aggregates, integrity, diagnostics, exports, or SQLite telemetry tables. A one-Hz complete snapshot with arrays bounded to 50 is within the existing state-message bounds; conformance will enforce the resulting encoded size.

Publishing BMS values in `LiveSample.extensions` was rejected because it supports only scalar numeric values, cannot represent arrays or balancing flags, and would falsely imply session persistence semantics.

### 7. Present BMS state through a conditional menu sheet

The shared live-board menu adds a `BMS` item once at least one VBMS identity is detected in the current generation, including attached, unavailable, and ambiguous states. It remains absent while discovery has found no identity and in `not_found`. Activating it opens the existing responsive sheet/popover pattern and reconstructs entirely from authoritative state after WebView recreation.

The sheet displays route and status, observed-at age/staleness, summary voltage/current/SoC/SoH values, per-cell voltages and balancing state, temperatures, environmental values, counters, and optional status/version fields. Ambiguous state lists the bounded candidates and instructs the user to set `preferredBmsCanId` in Board Profile; unavailable state retains and visibly marks any stale snapshot. It sends no BMS mutation request.

The menu item and sheet controls use native button semantics, `type="button"`, primary Pointer Events with cancellation, a guarded keyboard/assistive click path, suitable `touch-action`, focus containment/restoration, and no double activation.

### 8. Extend Board Profile, fixtures, and conformance together

`preferredBmsCanId` is nullable/absent for automatic discovery and otherwise an integer `0...253`. It does not affect profile completeness. Board Profile writes remain revision checked and preserve unrelated fields. The current disposable Apple and Android schemas/default JSON change together without a migration.

Conformance-owned topology types gain zero, one, or multiple source-faithful VBMS nodes, local-direct VBMS transport, independently routed controllers and BMS nodes, legacy/extended values, omitted ping IDs, route timeouts, malformed counts and remainders, mismatched reported IDs, reconnect, and disconnect behavior. The interactive fixture exposes bounded configuration and plausible seeded BMS values, while ESP32 firmware continues to transport only bytes and events.

Physical coverage adds the supported direct-controller-plus-CAN-BMS, Express-plus-CAN-BMS, and direct-BMS-gateway-to-controller topologies where hardware permits. The retained Thor301 capture identifies a Thor301 controller at CAN 3 and VBMS32 6.06 custom module at persisted CAN 10; iOS and Android values polling are verified, while unavailable topologies remain explicitly recorded rather than inferred. Software conformance remains the required gate; physical BLE remains an explicit checkpoint.

## Risks / Trade-offs

- [Auxiliary discovery can take up to the bounded CAN-probe window] → Run it after controller routing is established, reuse complete inventories, serialize it at low priority, and expose progress without blocking `ready`.
- [A one-Hz array snapshot increases state-event size and frontend work] → Bound arrays to 50, replace rather than append snapshots, conflate events, and render stable keyed rows without animation churn.
- [Strict BMS identity/value variants may reject future firmware] → Tie accepted layouts to pinned upstream fixtures and add a compatibility entry only after auditing changed source. VESC Express 7.00 remains rejected until its identity, command-96 encoder, both native decoders, conformance, and physical hardware are audited together.
- [A preferred BMS can disappear while another remains present] → Treat the preference as authoritative and report unavailable; never silently switch battery identity.
- [Retaining stale values can be mistaken for current data] → Keep the observation timestamp, publish explicit unavailable status, and render freshness prominently.
- [The connection-wide scheduler touches mature controller polling] → Introduce typed scheduler tests first, preserve request priority/cadence assertions, and land the refactor as a verified slice before enabling BMS work.
- [This change overlaps an active controller-routing delta] → Implement after that change, reconcile its synced canonical specs before final validation, and archive changes in dependency order.

## Migration Plan

1. Complete and archive `support-direct-and-can-vesc-connections` or otherwise sync its delta specs before implementing this dependent change.
2. Add the contract/profile shapes, request scheduler, common firmware parser, BMS codec, auxiliary discovery, polling, frontend presentation, and simulator coverage in verified slices.
3. Recreate disposable test databases through the existing schema-version behavior; no user-data migration is introduced.
4. Run focused frontend/native/conformance tests, `just verify`, strict OpenSpec validation, and the explicit physical VBMS checkpoint.
5. Roll back by reverting this change; controller routing and Refloat session behavior remain independently functional, and disposable profile storage can be recreated.

## Resolved physical evidence

- The 2026-08-06 Thor301 observation records controller CAN 3 (`Thor301`, type 0, 6.06) and BMS CAN 10 (`VBMS32`, type 2, 6.06). iOS and Android discovered and polled the VBMS32 successfully after the exact compatibility entry was added. The CAN ID is device configuration, not product identity; upstream VBMS32 defaults to 3 and persists a unique configured controller ID.
