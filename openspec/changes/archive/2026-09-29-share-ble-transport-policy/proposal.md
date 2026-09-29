## Why

The complete Rust backend still leaves duplicate VESC BLE policy and write queues in Swift and Kotlin. Linux would otherwise need a third implementation of discovery filtering, profile decisions, bounded chunking and write ordering.

## What Changes

- Move common BLE transport policy into one bounded Rust state machine owned by the existing process service, below application reconnect policy.
- Share advertisement eligibility, required GATT profile decisions, write-mode preference, payload splitting, queue accounting and generation/correlation checks.
- Keep native object ownership, permissions, scanning/GATT calls, negotiated capabilities, notification setup and OS flow-control mechanisms in native adapters.
- Preserve Android's conservative 20-byte payload and 20 ms no-response pacing and Apple's negotiated payload capped at 244 bytes; do not normalize platform behavior without evidence.
- Remove native `VescBluetoothPolicy` replicas and superseded native logical write queues after both hosts select the shared policy. Preserve only bounded OS execution state.
- Audit setup deadlines against the existing shared connection deadline so extraction does not add another reconnect or timeout owner.

## Capabilities

### New Capabilities

- `shared-ble-transport-policy`: Portable, bounded BLE decisions and ordered writes driven by normalized OS capabilities and correlated events.

### Modified Capabilities

- `vesc-device-connection`: Require the common BLE policy on both native hosts while retaining platform-specific GATT execution.

## Impact

Touches `backend-service`, its closed OS exchange through existing C/JNI bindings, native Bluetooth adapters/policy tests, conformance and ownership audits. No new crate, external dependency, binding artifact, frontend API, persistent format or data migration. Stricter internal accounting includes in-flight writes within the existing 128-write/65,536-byte limits; oversize work is rejected before allocation or transmission.

Non-goals: Linux/BlueZ implementation, moving OS Bluetooth libraries into Rust, removing supported firmware interoperability, increasing Android MTU, unifying native permission flows, general host scheduling, or implementing the separate shared simulation change. Neither proposal depends on the other landing first.
