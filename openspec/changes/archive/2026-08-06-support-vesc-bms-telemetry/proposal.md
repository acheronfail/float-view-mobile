## Why

Float View can identify and poll a motor controller but cannot discover a VESC BMS (VBMS) as a distinct direct or CAN-routed node, establish that it is currently attached, or show its cell-level telemetry. Adding bounded live VBMS support gives riders reliable battery-health visibility without coupling BMS availability to controller readiness or expanding durable session data.

## What Changes

- Classify the local VESC endpoint and bounded CAN inventory by an exact audited BMS compatibility catalog, retaining stock `HW_TYPE_VESC_BMS` 6.00 and VESC Express `VBMS32` `HW_TYPE_CUSTOM_MODULE` 6.06 nodes independently from the selected motor controller.
- Add an auxiliary direct-or-CAN BMS route, authoritative `preferredBmsCanId` Board Profile setting, and deterministic handling for absent, unique, configured, timed-out, malformed, and multiple VBMS nodes.
- Query a selected VBMS directly with `COMM_BMS_GET_VALUES`, decode bounded cell, balancing, temperature, charge, health, environmental, lifetime, version, and status values, and poll it on the serialized VESC request stream at a slow cadence.
- Publish complete live-only BMS state and freshness through the shared frontend contract without persisting BMS samples into sessions, diagnostics, or exports.
- Add a BMS menu item only when a VBMS identity has been detected; activating it opens the shared sheet/popover with route, availability, freshness, and live BMS information.
- Extend the conformance simulator and interactive fixture with source-faithful direct and CAN-routed VBMS identities and values, multiple-node ambiguity, configured routing, timeouts, malformed bounds, reconnect, and disconnect cases.
- Keep controller discovery, Refloat telemetry, session creation, riding continuity, and connection `ready` independent of BMS discovery or failure.

Non-goals are historical BMS storage or export, BMS configuration or mutation commands, support for non-VESC BMS protocols or arbitrary custom modules, automatic selection among multiple unconfigured VBMS nodes, accepting unaudited VESC Express 7.00 identities, and using a controller-cached BMS response as proof of attachment.

## Capabilities

### New Capabilities

- `vesc-bms-telemetry`: Discover, select, directly query, validate, and represent a live VESC BMS independently from the motor-controller route and lifecycle.

### Modified Capabilities

- `backend-contract`: Add bounded live BMS state and `preferredBmsCanId` Board Profile behavior to the mandatory cross-platform contract without changing session persistence.
- `live-board-dashboard`: Add a conditional BMS menu action and accessible sheet/popover for live BMS availability and telemetry.
- `backend-conformance`: Cover source-faithful VBMS topology, routing, values, bounds, ambiguity, failure, and lifecycle behavior on both native backends.
- `interactive-fixture-development`: Allow the composable development fixture to include deterministic VBMS nodes and live values without moving scenario logic into ESP32 firmware.
- `platform-hosts`: Add mirrored native ownership for auxiliary-node discovery, serialized polling, firmware classification, and bounded BMS decoding.

## Impact

The TypeScript contract and mock, frontend state and live-board UI, Apple and Android application/protocol owners, Board Profile validation and current disposable schemas, conformance simulator and fixtures, interactive fixture controls, and canonical tests change together. The implementation adds no external dependency and no database migration; BMS values remain live-only, and superseded controller-only routing or parsing assumptions are removed rather than duplicated.
