# Change: Support direct and CAN-routed VESC connections

## Why

Float View currently identifies a directly connected VESC but then requires a CAN ID for layout negotiation and telemetry polling. That makes direct BLE connections stall even though firmware detection succeeded. CAN-routed connections also assume discovery will yield one usable controller, cannot target installations where CAN ping is unavailable or incomplete, and cannot ask the user to disambiguate multiple motor controllers.

The connection flow should be topology-aware, deterministic, and compatible with the VESC and Refloat firmware combinations represented by `fixture-dev`. It should follow the discovery and routing principles used by VESC Tool while preserving Float View's authenticated contract, bounded state, privacy rules, and shared frontend.

## What Changes

- Add one native controller-discovery and negotiation flow on Apple and Android that:
  - distinguishes a VESC connected directly over BLE from a VESC reached through a VESC Express or other CAN gateway;
  - represents every protocol request with an explicit direct-or-CAN controller route;
  - discovers CAN devices with bounded ping and firmware probes, filters for VESC motor controllers, and never silently chooses the first of several candidates;
  - resolves or creates saved-device identity after a valid local VESC firmware response, loads its Board Profile before controller routing, and creates a durable session only after controller and telemetry negotiation reach application `ready`;
  - validates a configured or remembered CAN controller before declaring the session ready;
  - renegotiates the route and compatibility profile after reconnects, rejects stale callbacks, and prevents a different controller from being appended to an existing session.
- Add an interactive multi-controller state. When discovery finds more than one VESC motor controller, the backend publishes at most 16 CAN-ID-sorted candidates containing a CAN ID, canonical controller UUID when meaningful, identifying firmware information, and whether that VESC firmware version is supported. The shared frontend prompts the user to choose, disables unsupported candidates, validates the selection against the current discovery generation, atomically merges the chosen CAN ID into the latest Board Profile, and resumes connection. Candidate overflow fails without publishing a misleading truncated list. Controller UUIDs may also be used in bounded logs and other application surfaces.
- **BREAKING** Extend the backend contract and complete state snapshot with:
  - explicit identifying and controller-selection connection states;
  - detected controller transport information (`direct` or `can` with its CAN ID);
  - bounded controller candidates while selection is required;
  - a bounded connection-issue value with stable asynchronous failure codes;
  - a controller-selection request;
  - an optional `preferredCanId` board-profile setting, validated in the VESC CAN range and applied on the next connection, reconnect, or same-device retry through a gateway.
- Keep any public `preferredCanId` authoritative, whether entered in Board settings or produced by controller selection. Probe that route directly even when CAN ping is unavailable; fail with actionable state when it is not a compatible VESC instead of falling back to a different controller. Use automatic learned CAN-ID/UUID hints while the public preference is null, and fall back from a stale hint to automatic discovery. Ignore both CAN mechanisms when the BLE endpoint is itself a direct motor controller.
- Retain failed active state with its saved identity and stable issue until the user retries or disconnects. Reusing `device.connect` for the same failed `deviceId` starts a new bounded connection generation using the latest Board Profile; a different active device remains invalid.
- Correct telemetry setup so direct sessions poll directly and CAN sessions wrap every applicable firmware, Refloat, layout, telemetry, and adaptive-logging command for the selected CAN ID.
- Support the fixture catalog's audited VESC 6.02, 6.05, 6.06, and 7.00 firmware variants and Refloat source packages 1.0.0, 1.1.0, and 1.2.3 through explicit compatibility families. Strictly decode the complete VESC firmware identity baseline, allow nonzero test markers only for audited versions in read-only mode, tolerate only bounded source-defined optional tails, and prove the route and Refloat capability before reaching `ready`. Future writes must pass a capability gate and are denied for test versions. The legacy 1.0.0 source package exposes only wire identity `1.0`, which is what Float View reports; Refloat 1.0 uses the fixed packet set while Refloat 1.1 and 1.2 use negotiated dynamic layouts.
- Treat a non-motor VESC-protocol BLE endpoint as a potential CAN gateway and prove the capability through a configured forwarded firmware probe or bounded CAN discovery. The VESC Express 6.05 fixture is the canonical tested gateway identity without restricting compatible forwarding to its display name alone.
- Harden native BLE behavior by mirroring VESC Tool's discovery rule: scan BLE broadly, publish ordinary devices only when their advertisement contains the Nordic UART service, and let only an exact saved Apple peripheral UUID or Android address bypass that advertisement check. Direct connection by that saved identifier still must validate the actual UART service, characteristics, and notifications within the setup deadline. Segment writes to the negotiated/default BLE payload size, accept default ATT MTU operation, and bound all queues and retries.
- Restructure the native connection and protocol sources into mirrored ownership areas for controller discovery, telemetry polling, VESC framing/firmware parsing/routing, Refloat protocol families, and compatibility mapping. Document the ownership and dependency rules so adding a firmware version normally means adding a compatibility entry, not another copy of connection code.
- Expand software and physical conformance coverage for direct, discovered CAN, configured CAN, multiple-controller selection, reconnect persistence, failure paths, BLE fragmentation/setup behavior, stable-versus-test firmware identity, and the supported VESC/Refloat compatibility matrix.
- Remove the documented direct-telemetry limitation, the implicit `canId != null` polling prerequisite, broad version predicates, and superseded discovery/polling branches left in monolithic application-service or protocol files.

Non-goals:

- Supporting non-VESC CAN devices as telemetry targets.
- Supporting VESC test builds outside the audited catalog, prereleases, unknown releases, or firmware older than the audited 6.02 baseline; audited test builds are read-only.
- Exposing controller UUIDs, BLE addresses, launch tokens, or full telemetry in state or default logs.
- Moving scenario interpretation into the ESP32 fixture firmware.
- Adding a generated contract layer or platform-specific frontend.

## Capabilities

### New Capabilities

- `vesc-device-connection`: Defines BLE setup, direct-versus-CAN controller discovery, explicit route semantics, multi-controller selection, firmware and Refloat compatibility negotiation, and route-correct telemetry polling.

### Modified Capabilities

- `backend-contract`: Adds connection-identification and selection state, detected route data, controller candidates and selection request, optional board-profile CAN preference, and stricter ready-session semantics.
- `backend-conformance`: Adds canonical scenarios and assertions for direct, discovered CAN, configured CAN, ambiguous CAN, reconnect, failure, BLE transport, and firmware-package compatibility behavior.
- `live-board-dashboard`: Displays the detected direct/CAN route, prompts for a controller when discovery is ambiguous, and exposes the optional preferred CAN ID in board settings.
- `app-navigation`: Extends Boards configuration and live connection flows to include the optional CAN preference and controller-selection experience.
- `interactive-fixture-development`: Extends the composed VESC simulator and fixture-development catalog to exercise all declared VESC/Refloat versions and bounded multi-controller gateway configurations without retaining the current direct-telemetry exception.
- `platform-hosts`: Establishes mirrored native source ownership for discovery, polling, routing, protocol families, and compatibility data while retaining the required dependency direction.

## Impact

- Frontend contract, client, mock backend, authoritative state store, connection UI, live firmware details, and Board Profile editor.
- Apple and Android application services, Bluetooth adapters, VESC framing/firmware parsing, Refloat codecs, persistence schemas, logging, and lifecycle handling.
- Canonical conformance protocol, simulator configurations, scenarios, reports, and representative physical BLE checkpoints.
- Fixture-development scenario catalog and documentation; the ESP32 remains a byte bridge.
- Native source layout and backend documentation, with old superseded paths deleted after callers move.
- Persisted board profiles gain a nullable CAN preference and native persistence gains a private learned-controller routing hint; existing profiles remain valid without migration tooling.
- Valid VESC transports become saved boards before controller selection, while session creation moves to application `ready`; unsuccessful arbitrary BLE peers are not saved.
- No new runtime dependency is planned.
