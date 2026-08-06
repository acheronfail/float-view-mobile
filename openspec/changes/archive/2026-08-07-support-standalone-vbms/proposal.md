## Why

VBMS32 devices expose the same VESC UART Bluetooth profile as motor controllers, but Float View currently fails their connection when no motor controller is available. Exact directly connected VBMS devices should instead become useful read-only BMS connections with their existing live values presented as a dedicated device experience.

## What Changes

- Classify an exact supported directly connected VBMS identity as a BMS-only device immediately after firmware identification.
- **BREAKING** Stop treating an exact direct VBMS as a transparent gateway for discovering and operating a CAN motor controller; users must connect through a controller or controller-oriented gateway for the board experience.
- Add an authoritative connection kind to the frontend/backend contract so navigation and controls never infer device capability from names or partial telemetry.
- Reuse the existing BMS presentation in a full-page route while retaining it in the live-board sheet for auxiliary BMS devices.
- Keep standalone BMS connections read-only and live-only: no Refloat negotiation, motor polling or controls, location recording, ride session, or durable BMS telemetry history.
- Add canonical mock, Apple, Android, and frontend coverage for initial classification, direct polling, navigation, reconnect, cancellation, and privacy bounds.
- Non-goals: BMS mutations, arbitrary custom-module support, BLE-name classification, BMS history, or a user-selectable direct-VBMS gateway mode.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `backend-contract`: Expose the authoritative kind of an active connection without weakening complete state snapshots.
- `vesc-device-connection`: Complete exact direct VBMS identities as BMS-only connections instead of searching CAN for a motor controller.
- `vesc-bms-telemetry`: Poll direct standalone VBMS values through the existing bounded read-only lifecycle.
- `live-board-dashboard`: Reuse the BMS view in a dedicated full-page route and route active devices by authoritative kind.
- `backend-conformance`: Replace direct-VBMS gateway expectations with canonical standalone-VBMS behavior across both native backends.

## Impact

The hand-written frontend contract, mock backend, shared SvelteKit navigation, Apple and Android application services, controller/BMS discovery policy, and canonical conformance fixtures and assertions change together. Saved and guest connection envelopes remain compatible apart from the new required active-connection field; standalone BMS connections reuse existing persistence association but never create a session or historical BMS data. No dependency or database-schema change is expected, and the superseded direct-VBMS-plus-CAN-controller conformance path is removed.
