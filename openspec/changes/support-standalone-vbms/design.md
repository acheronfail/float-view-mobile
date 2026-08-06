## Context

Apple and Android scan the VESC Nordic UART service, then identify the connected endpoint with `COMM_FW_VERSION`. The exact supported VBMS catalog and direct `COMM_BMS_GET_VALUES` decoder already exist, but controller discovery owns the primary connection lifecycle and auxiliary BMS discovery starts only after a motor controller becomes ready. The frontend already renders BMS state through a presentation-only `BmsView`, while navigation assumes every active connection belongs on `/live`.

The vendored VESC Express source establishes that VBMS32 uses the same UART GATT UUIDs, reports `VBMS32`, `HW_TYPE_CUSTOM_MODULE`, version 6.06, and implements command 96. Its configurable advertised name is not identity evidence.

## Goals / Non-Goals

**Goals:**

- Classify an exact direct stock VBMS 6.00 or VBMS32 6.06 identity before generic gateway/controller discovery.
- Complete a bounded BMS-only connection without CAN enumeration, Refloat/controller work, or session creation.
- Publish enough authoritative state for the shared frontend to select a dedicated BMS route.
- Reuse the existing bounded read-only BMS polling and presentation on Apple, Android, mock, and conformance targets.
- Preserve auxiliary CAN-BMS behavior for connections whose direct endpoint is a motor controller or non-BMS gateway.

**Non-Goals:**

- Using a direct VBMS as a controller gateway, or offering a mode switch to restore that path.
- Inferring device kind from an advertised name, address, or platform identifier.
- BMS writes, configuration, firmware operations, ride sessions, location capture, or historical BMS storage.
- Supporting unaudited BMS identities or broad version ranges.

## Decisions

### Exact direct identity selects the connection kind

After the first structurally valid direct firmware response, the application service applies one shared exact BMS compatibility policy. A supported BMS identity sets `deviceKind` to `bms`, transitions the existing connection to `ready`, selects the direct BMS route, and begins values polling. It sends no CAN ping or controller/Refloat request.

Motor identities and non-BMS custom modules retain the existing controller discovery path and set `deviceKind` to `board` only after a controller route is established. This deliberately replaces the direct-VBMS gateway behavior. Name matching was rejected because the firmware permits arbitrary BLE names, including numeric names.

### Add a nullable authoritative device kind

`ActiveConnection` gains required `deviceKind: 'board' | 'bms' | null`. Null means identification has not yet established the experience. The field is complete and generation-scoped in every state snapshot; the frontend does not inspect firmware hardware names or BMS status to decide navigation.

A separate active-connection union was rejected because most lifecycle, display-name, disconnect, saved/guest intent, and failure fields are shared, and controller-only fields are already nullable or safely unavailable before readiness.

### BMS-only readiness is live-only

The BMS-only completion path cancels identification deadlines, clears scan results, reaches `ready`, and invokes existing auxiliary discovery with the exact local identity so it selects `.direct` and schedules command-96 polling. It does not call controller readiness, create/resume a session, start telemetry or location, request motor configuration, or configure Refloat/controller controls. Existing disconnect and BLE reconnect generation cancellation remain authoritative.

### Reuse BMS content in two shells

`BmsView` remains the single content component. The live-board route continues wrapping it in `BottomSheet`; a new `/bms` route wraps it in full-page layout with BMS connection status and disconnect only. App-shell and route guards use `deviceKind` to replace-route BMS connections to `/bms` and board connections to `/live`, including frontend recreation.

### Replace the direct-gateway fixture

Canonical conformance removes `direct-vbms-plus-can-controller` and introduces an exact standalone direct VBMS scenario. It asserts ready BMS kind, direct values, no controller/Refloat/session state, no CAN ping or forwarded request, bounded reconnect, and parity across Apple and Android. Existing controller-plus-CAN-BMS scenarios retain auxiliary coverage.

## Risks / Trade-offs

- [Existing users relied on VBMS Bluetooth as a CAN controller gateway] → This is an intentional breaking choice; connect through the controller or controller-oriented gateway instead, and remove the conflicting conformance promise.
- [Shared `ready` state could accidentally start board work] → Keep distinct completion methods and add negative packet/session assertions in native and black-box tests.
- [Navigation briefly enters `/live` because `device.connect` is asynchronous] → Route guards replace history as soon as authoritative `deviceKind` arrives; keep the accepted-request contract asynchronous.
- [A spoofed name appears to be a VBMS] → Ignore names and require the existing structurally decoded exact hardware type/name/version catalog.
- [BMS polling loss is mistaken for full disconnection] → Retain current attached-to-unavailable BMS state while BLE remains ready; only transport events control connection lifecycle.

## Migration Plan

No database migration or dependency change is required. Add the contract field to the frontend, mock, both hand-written backends, and conformance in one implementation slice. Remove the superseded direct-gateway fixtures and documentation. Rollback is the inverse code/spec change; stored test data is unaffected.

## Open Questions

None. Direct exact VBMS identity is intentionally authoritative for BMS-only mode.
