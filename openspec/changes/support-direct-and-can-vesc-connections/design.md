## Context

Both native backends currently perform the first half of direct-device detection correctly: they request `COMM_FW_VERSION` from the BLE endpoint, recognize a motor-controller hardware type, and request Refloat INFO without CAN forwarding. The flow then loses that topology distinction. Telemetry-layout discovery and polling are guarded by a non-null firmware `canId`, so a direct connection cannot schedule either operation. On Apple, the direct-layout retry path also returns before advancing its attempt count, producing an effectively endless no-op retry.

CAN discovery is similarly coupled to nullable firmware fields rather than an explicit route. It assumes a useful CAN ping response and one motor controller, while some VESC Express installations require a known CAN ID and other installations contain multiple VESC controllers. Firmware parsing also discards the controller UUID bytes that VESC Tool uses to verify a remembered controller.

VESC Tool is the behavioral reference for the Bluetooth and early protocol layers. Its relevant properties are:

- Nordic UART service discovery and notification setup are bounded and validated before the connection is usable.
- BLE writes form a byte stream and are segmented to the available payload size; a default ATT MTU remains valid.
- Direct transport and CAN forwarding are separate route choices applied to commands, not inferred again in each polling call.
- CAN discovery pings while direct, probes firmware for each returned ID, and filters by VESC hardware type.
- A remembered transport-to-controller choice includes the remote CAN ID and controller UUID and is revalidated on reconnect.

Float View must implement the same robustness without exposing stable device identifiers, introducing frontend platform branches, or moving protocol behavior into the ESP32 byte bridge. The hand-written frontend contract, both native implementations, mock, persistence, and conformance must change together.

## Goals / Non-Goals

**Goals:**

- Make direct and CAN-routed VESC connections equally capable of reaching `ready`, polling telemetry, reconnecting, and recording sessions.
- Deterministically select zero, one, configured, or multiple CAN-connected motor controllers.
- Give the user enough display-safe information to choose among multiple controllers and persist that choice.
- Support audited stable VESC 6.02/6.05/6.06/7.00 and Refloat 1.0.0/1.1.0/1.2.3 through explicit, maintainable compatibility families and capability evidence.
- Harden BLE setup, stream fragmentation, timeouts, and saved-device discovery on Apple and Android.
- Give Swift and Kotlin matching ownership boundaries that make later compatibility additions easy to locate and review.
- Cover topology, version, lifecycle, and failure behavior in canonical conformance.

**Non-Goals:**

- Selecting or polling non-VESC CAN devices.
- Automatically overriding an explicit user CAN preference.
- Exposing firmware UUIDs or Bluetooth addresses beyond the existing platform-identifier contract.
- Supporting arbitrary historical VESC or Refloat versions not listed by this change.
- Generating cross-platform protocol models or sharing Swift/Kotlin source.
- Adding database migrations while persisted data remains disposable test data.

## Decisions

### 1. Make the selected controller route an explicit native value

Each backend will use one internal value for all controller-bound work:

```text
ControllerRoute = direct | can(id: 0...253)
```

Firmware probes, Refloat INFO, layout negotiation, telemetry polling, and reconnect validation accept this route. The VESC command layer is the only owner of `COMM_FORWARD_CAN` wrapping. A direct command is sent unchanged; a CAN command is wrapped once with the selected ID.

This replaces nullable `canId` checks spread through application services and protocol code. Inferring the route at every call site was rejected because it caused the current direct-polling defect and makes reconnect races difficult to reason about.

The public `ActiveConnection` exposes a nullable `controllerConnection` discriminated union: `{ kind: "direct" }` or `{ kind: "can", canId }`. It is null only before a route has been established.

### 2. Use one bounded discovery state machine

A mirrored `ControllerDiscovery` component on each platform owns one connection generation and these conceptual stages:

```text
connecting -> connected -> discovering_services -> identifying
                                              ├-> selecting_controller -> identifying
                                              └-> ready
reconnecting ---------------------------------------> identifying -> ready
any identifying stage ------------------------------> failed(issue)
```

The Bluetooth adapter's internal UART-ready signal is not application `ready`. After UART setup, the backend requests local firmware while public state is `identifying`. Once a valid local `COMM_FW_VERSION` response establishes that the peer speaks the VESC protocol, the product store resolves or creates its saved-device record by the platform Bluetooth identifier, publishes `savedDeviceId`, and loads its Board Profile. Arbitrary BLE peers that never establish valid local VESC identity are not saved. This separates saved identity from session creation: no durable session is created until application `ready`.

The active state uses `identifying` during automatic work and `selecting_controller` while user input is required. Entering a new connection or reconnect generation clears candidates, the previously verified public route, and any old connection issue. `ready` is emitted only after the selected route, controller firmware, compatible Refloat family, and telemetry plan are established. It then creates or resumes the one durable session according to the existing session-lifecycle rules.

If the local BLE endpoint reports VESC motor-controller hardware, discovery selects `direct` and does not use a stored CAN preference. Otherwise:

1. If `preferredCanId` exists, probe that CAN route directly.
2. Otherwise, if a private automatic routing hint exists, probe its CAN ID and confirm its meaningful UUID. A match selects it. Without an active session, timeout or mismatch invalidates only the hint and continues normally. With an active session, discovery continues in continuity-recovery mode and may select only the same meaningful UUID at another CAN ID.
3. Otherwise send a bounded CAN ping, normalize at most 64 unique IDs in `0...253`, sort them numerically, and sequentially request firmware for each ID.
4. Keep devices whose parseable firmware reports `HW_TYPE_VESC`, retaining whether their exact VESC version is supported.
5. Select the sole motor controller automatically, fail on none, or pause for user selection on multiple. An unsupported sole controller continues only far enough to publish its identity and then fails as unsupported.

Any valid VESC-protocol local firmware identity that is not `HW_TYPE_VESC` is treated as a potential gateway. Gateway capability is proven by a successful configured forwarded firmware probe or CAN discovery plus a forwarded firmware probe; it is not inferred from the advertised or hardware display name. The VESC Express 6.05 fixture remains the canonical tested transport.

Sequential probing preserves the one-request-in-flight VESC stream invariant. All callbacks carry a connection generation; disconnects, reconnects, retry, and a new connection invalidate older callbacks. Deadlines, response bytes, retries, diagnostics, 64 discovered CAN IDs, and 16 motor-controller candidates are bounded. More than 64 normalized CAN IDs fails with `too_many_can_devices`; finding a seventeenth motor controller fails with `too_many_controllers` and publishes no partial candidate list. Both failures direct the user to configure `preferredCanId`.

Treating the first ping result as the controller was rejected because CAN buses may contain BMS, Express, display, and multiple motor-controller nodes. Parallel firmware probes were rejected because the VESC byte stream has no correlation ID and would make response attribution unsafe.

### 3. Make ambiguity visible and resolve it through a contract command

When more than one motor controller is found, authoritative state contains a CAN-ID-sorted `controllerCandidates` array of at most 16 items and state `selecting_controller`. Each candidate contains:

- CAN ID;
- hardware/model name when supplied;
- firmware name when supplied;
- VESC version;
- `vescSupported`, derived from the exact controller compatibility catalog.

The firmware UUID is retained only in native discovery/persistence data. It is neither a candidate field nor logged.

Unsupported motor controllers remain visible so discovery does not conceal a physical controller, but the frontend disables them and the backend does not accept them as a route. The mandatory `device.selectController` request carries the active `deviceId` and chosen `canId`. Malformed or out-of-range parameters fail with `invalid_request`. A different device, unsupported or non-candidate ID, stale generation, duplicate/late selection, or connection not awaiting selection fails with `invalid_state`. These outcomes do not change the route or persistence.

A valid choice performs a product-store transaction that reads the latest Board Profile, merges only `preferredCanId`, preserves any concurrent edits to other fields, advances the revision once, records the private learned UUID when meaningful, clears the candidate list, and resumes negotiation. The command is distinct from `boardProfile.update`: discovery can validate current candidates and resume atomically without making the frontend coordinate profile revisions. A concurrently open editor whose expected revision is now stale receives the normal conflict and reloads authoritative profile state.

The shared frontend owns one accessible modal prompt for this state. It lists all display-safe fields, keeps disconnect available, handles primary pointer and keyboard activation, and reconstructs itself from `state.get` after WebView recreation.

### 4. Treat an explicit CAN preference as authoritative

`BoardProfile.values.preferredCanId` is absent or null when automatic discovery is desired and otherwise an integer from 0 through 253. It does not contribute to `missingFields` or profile completeness. It is used only when the connected BLE endpoint is a gateway and takes effect on the next connection, reconnect, or explicit retry of the same failed device. A Board Profile edit that changes or clears the preference invalidates any controller UUID associated with the old public route; the next successful preferred-route probe stores its own meaningful UUID. Interactive selection stores its chosen ID and already-probed UUID together.

The backend probes a preferred route without requiring CAN ping to return it. A timeout or non-VESC response produces `preferred_controller_unavailable`; a parseable VESC with an unsupported controller version produces `unsupported_controller`; and mismatch against a meaningful UUID stored for that same public route produces `controller_changed`. It does not silently switch to another CAN ID. Clearing the setting restores automatic discovery.

This favors predictable installations over opportunistic fallback. Silent fallback was rejected because it can record the wrong motor controller on a multi-controller bus.

### 5. Retain a private learned controller identity

The VESC firmware parser will require the complete source-defined identity baseline: command and major/minor bytes, a terminated bounded hardware name, 12-byte controller UUID, pairing marker, firmware test-version marker, hardware type, custom-configuration count, phase-filter/QML/NRF flags, and a terminated bounded firmware name. It preserves the UUID, exposes only display-safe names, and tolerates a bounded optional four-byte hardware-CRC suffix. The CRC is build and hardware provenance rather than a compatibility key. A truncated baseline, unterminated name, or unexpected remainder is malformed rather than a supported identity.

Native persistence associates a meaningful non-zero UUID and CAN ID with the saved BLE transport. When public `preferredCanId` is null and exactly one supported controller was automatically discovered, that pair is a private automatic routing hint. A controller selected by the user instead writes the authoritative public preference; it is not treated as a fallback hint.

On a connection without an active session, a private-hint timeout or UUID mismatch invalidates the hint and returns to discovery. If discovery finds the same meaningful UUID at a new CAN ID, the backend updates the private hint and uses it. During active-session reconnect, the same search is allowed only to recover that exact UUID; finding it at a new ID updates the hint and safely resumes the session. Firmware versions that omit a meaningful UUID continue to work using the selected CAN ID and normal firmware validation.

An existing durable session has a stronger continuity rule. Reconnect may append new samples only after confirming the same controller UUID. When a meaningful UUID is unavailable, it requires the same route plus matching supported controller and Refloat identities. A different meaningful UUID, or an ambiguity that cannot establish continuity, closes the existing session as interrupted and enters `failed` with `controller_changed`; it never prompts or silently appends a replacement controller to that session. A new explicit connection may then discover or select another controller and create a new session.

UUIDs are intentionally excluded from frontend state, errors, exports, and default logs. Persisting them in the public Board Profile was rejected because they are stable hardware identifiers and the user only needs the CAN preference.

### 6. Represent asynchronous failure and retry in authoritative state

`ActiveConnection.connectionIssue` is null outside an unresolved failure and otherwise contains a stable code plus a bounded display message. The initial codes are:

```text
transport_setup_failed
identification_timeout
no_controller_found
too_many_can_devices
too_many_controllers
preferred_controller_unavailable
controller_changed
unsupported_controller
unsupported_refloat
```

The failed active connection retains its `deviceId`, saved identity when established, firmware and route information already verified in that generation, and latest persisted sample. It does not poll or create a session. `device.connect` with that same failed `deviceId` is the retry operation: it clears the issue, starts a new generation, and uses the latest Board Profile. A request for a different device while one remains active returns `invalid_state`. Disconnect remains available in every non-terminal identifying, selection, and failed state.

A separate retry method was rejected because the existing connect intent already carries the correct device identity and can have one unambiguous same-device failed-state meaning.

### 7. Separate firmware versions from wire-protocol families

Compatibility data is declarative and maps exact audited stable versions to the smallest set of consumed wire implementations:

| Identity | Compatibility family | Notes |
|---|---|---|
| VESC 6.02 | `vescCoreReadOnly` | Complete stable identity baseline; no hardware-CRC suffix in the audited source |
| VESC 6.05 | `vescCoreReadOnly` | Same consumed commands; source-defined hardware-CRC suffix |
| VESC 6.06 | `vescCoreReadOnly` | Same consumed commands and identity shape |
| VESC 7.00 | `vescCoreReadOnly` | Same consumed commands and identity shape as 6.06 |
| Refloat source package 1.0.0 | `refloatLegacyV1` | Wire identity and reported version `1.0`; fixed legacy realtime/all-data packets |
| Refloat 1.1.0 | `refloatDynamicV1` | Dynamic IDs and float16 data |
| Refloat 1.2.3 | `refloatDynamicV1WithAlerts` | Dynamic data with bounded alert/fault tail |

For VESC, a major/minor pair identifies only a candidate catalog entry. A controller is supported only when its identity baseline is structurally valid, its test-version marker is zero, its hardware type is `HW_TYPE_VESC`, and the selected direct or forwarded route successfully returns compatible Refloat INFO and any required layout. `COMM_FW_VERSION`, `COMM_FORWARD_CAN`, `COMM_CUSTOM_APP_DATA`, and `COMM_PING_CAN` retain the consumed command identities across the audited releases. Hardware name, firmware name, UUID, and hardware CRC do not broaden version support. A nonzero test-version marker, unknown major/minor pair, malformed baseline, or unsupported Refloat response fails closed.

VESC 7.01 remains unsupported while upstream identifies it as a test build. Future stable releases enter the catalog only after their identity response and every command Float View consumes are audited and immutable fixtures are added. A changed consumed command or required response shape creates a new profile; an added bounded optional identity suffix does not.

The legacy INFO response contains only protocol byte `10`; it cannot prove a patch version or carry a package name. Float View recognizes that source-derived namespace as Refloat, reports package version `1.0`, and treats fixture/source release 1.0.0 as the tested representative of that wire family. It does not claim that a physical device reported patch `.0`.

The compatibility map selects a family using parsed identity; codecs contain wire behavior, not version policy. Adding a version that uses an existing family requires a table entry and fixtures. A new codec file is added only when the wire shape changes.

A file per firmware/package version was rejected because it would duplicate framing, routing, and polling logic and make compatibility drift between Apple and Android likely. A broad `major >= 1` predicate was rejected because it claims support without establishing a tested wire family.

### 8. Split stream transport, discovery, polling, and codecs by ownership

Apple and Android use these concrete backend-owned paths:

```text
backends/apple/Sources/FloatViewBackend/
  application/{ApplicationService,ControllerDiscovery,TelemetryPolling,AdaptiveLogging,Domain}.swift
  bluetooth/{CoreBluetoothAdapter,SoftwareBluetoothAdapter}.swift
  protocol/
    DeviceCompatibility.swift
    vesc/{VescFrame,VescFirmware,VescCommands}.swift
    refloat/{RefloatProtocol,RefloatLegacy,RefloatDynamic}.swift

backends/android/app/src/main/java/fail/acheron/floatview/
  backend/{ApplicationService,ControllerDiscovery,TelemetryPolling,AdaptiveLogging,Domain}.kt
  bluetooth/{AndroidBluetoothAdapter,SoftwareBluetoothAdapter,VescGattProfile}.kt
  protocol/
    DeviceCompatibility.kt
    vesc/{VescFrame,VescFirmware,VescCommands}.kt
    refloat/{RefloatProtocol,RefloatLegacy,RefloatDynamic}.kt
```

The existing Android `backend` package is the counterpart of Apple `application`; retaining that established package avoids a repository-wide namespace rename while the filenames and protocol subtrees remain mirrored.

- `ApplicationService` coordinates contract commands and authoritative state.
- `ControllerDiscovery` owns the generation-based state machine and selection validation.
- `TelemetryPolling` owns layout establishment, cadence, request serialization, and decoded samples.
- `VescFrame` owns byte-stream framing and bounds.
- `VescFirmware` owns tolerant identity parsing, including private UUIDs.
- `VescCommands` owns direct/CAN encoding.
- `DeviceCompatibility` is the exact version-to-family catalog.
- Refloat family files own version-specific INFO/layout/data decoding.

`backends/README.md` will document these owners, the downward dependency direction, Swift/Kotlin mirroring, and the rule for adding versions. Superseded code is deleted from the monolithic service/protocol files after callers move; no compatibility wrapper layer remains.

### 9. Mirror VESC Tool's saved-device BLE discovery and bounded stream behavior

Both adapters validate the UART service, RX/TX characteristics, and notification enablement within one setup deadline. Writes are queued as bounded frame bytes and segmented according to the platform's current maximum write length, with 20 bytes as the safe default. Android no longer requires an MTU of 64; an increased MTU is an optimization. Platform-supported write-without-response is preferred, with the adapter's supported response mode handled without changing protocol semantics.

As VESC Tool does, the platform scan may observe BLE devices broadly and applies its allowlist before publishing results. A device is connectable when its advertisement includes the Nordic UART service UUID or its exact platform identifier is already saved: the CoreBluetooth peripheral UUID on Apple and Bluetooth address on Android. This lets an advertising saved board reappear after process restart even when its current advertisement omits the UART UUID. It does not publish or connect an arbitrary nonmatching scan result.

Connection uses the observed or stored identifier directly, but saved status bypasses only the advertisement check. Service discovery must still find the Nordic UART service and the required RX/TX characteristics and establish notification delivery inside the setup deadline. Android's narrowly scoped missing-CCCD Express compatibility remains, while other missing notification capabilities fail.

This follows VESC Tool's robust behavior while preserving Float View's stricter bounds and saved-identity scope.

### 10. Conformance uses exhaustive software coverage and representative physical coverage

The composed simulator remains the source of protocol behavior. Software tests cover the Cartesian direct/Express matrix for all four audited VESC and three representative Refloat versions, plus:

- complete 6.02 identity without CRC and 6.05/6.06/7.00 identity with bounded CRC suffix;
- malformed/truncated identity, unknown release, and nonzero test-version rejection;
- Express automatic discovery;
- configured CAN ID with absent/incomplete ping;
- wrong configured ID;
- zero, one, and multiple VESC candidates mixed with non-VESC nodes;
- selection validation, persistence, reconnect, and WebView recovery;
- default and enlarged BLE payloads, response modes, fragmentation, missing setup elements, timeouts, queue bounds, and stale callbacks.

Physical conformance keeps identity, ride, and reconnect on Android, iOS, and macOS. The complete direct-dynamic, direct-legacy, Express-discovered, Express-configured-without-ping, and Express-ambiguous-selection topology set runs once against the Android backend and once against the shared Apple backend on macOS. iOS runs direct dynamic, Express discovery, and the required background-continuity path; it does not repeat every shared-Apple topology case. The fixture firmware remains a byte bridge; conformance owns all identities, request classification, and responses.

## Risks / Trade-offs

- [Candidate firmware probing increases connection time on busy CAN buses] → Probe sequentially with per-request and total deadlines, cap normalized IDs/candidates, and publish meaningful identifying progress.
- [A large bus exceeds discovery bounds] → Fail without truncation, retain a stable issue, and direct the user to configure the exact CAN ID.
- [Two controllers can have identical display names and versions] → Always show CAN ID as the stable user-facing differentiator; retain UUID only for private reconnect validation.
- [A stored preference can become stale after rewiring] → Fail explicitly with guidance to clear/change the CAN ID; never record a different controller silently.
- [A reconnect reaches a different controller] → Interrupt the old session and require a new explicit connection rather than merging controller histories.
- [Controller selection races a Board Profile editor] → Merge only the selected CAN ID into the latest profile transaction and let stale editors use existing revision-conflict recovery.
- [Firmware response tails vary and the existing 6.05 fixture omitted its source-defined CRC] → Require the complete identity baseline, model the bounded CRC suffix explicitly in corrected immutable fixtures, and never use its value as a compatibility key.
- [Adding contract states can expose frontend race bugs] → Drive the prompt entirely from complete monotonic snapshots and validate selections against the native discovery generation.
- [Source extraction can create temporary duplicate paths] → Move one verified vertical slice at a time and delete superseded branches in the same implementation phase.
- [Physical matrix size can make the gate slow] → Keep exhaustive compatibility in software and reserve hardware for representative BLE/lifecycle paths.

## Migration Plan

1. Extend the TypeScript contract, mock, fixture schemas, and conformance client with the new state, issue, candidate, retry, and selection semantics while native implementations still compile against the same explicit shape.
2. Recreate disposable native databases at the new schema version; existing test data is intentionally not migrated.
3. Separate saved-device establishment from session creation, then extract shared concepts within each native backend and add route-aware discovery/polling and compatibility families.
4. Add the shared frontend selection prompt, failure recovery, route display, and Board Profile field, then delete old nullable-CAN and broad-version paths.
5. Enable exhaustive software scenarios, run the distributed representative physical scenarios, and run `just verify`.

Rollback is a source rollback plus recreation of disposable test databases. There is no production data migration.

## Open Questions

None. The saved-device/session boundary, failure and retry semantics, preference-versus-hint precedence, reconnect continuity, candidate fields and bounds, legacy version reporting, gateway classification, saved-device BLE discovery, request errors, profile merge behavior, concrete source locations, and physical coverage distribution are defined by this change.
