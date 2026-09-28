## Why

Software conformance currently has separate Swift, Kotlin and TypeScript VESC peripherals. A Linux host should reuse one simulator rather than implement a fourth; independent protocol expectations must remain outside the production Rust decoder.

## What Changes

- Make conformance-owned TypeScript simulation the sole software peripheral for direct Rust, macOS, iOS simulator and Android emulator runs.
- Add a bounded, authenticated test-only byte/event relay and thin native Bluetooth test adapters; keep protocol interpretation and scenarios in `conformance/`.
- Select simulation explicitly in conformance builds and launches, never from emulator detection alone. Release and physical builds cannot select it.
- Delete both native `SoftwareVescPeripheral` implementations, simulation-only protocol helpers and native fixture packaging once references are removed.
- Preserve the public frontend API, persistence schemas and physical ESP32 workflow.

## Capabilities

### New Capabilities

- `shared-software-peripheral`: One isolated, bounded software fixture session usable by every host without native protocol simulation.

### Modified Capabilities

- `backend-conformance`: Extend conformance-owned VESC simulation to software hosts and require explicit test selection and production exclusion.

## Impact

Touches `conformance/fixture`, target launchers/tests, native software Bluetooth adapters and build/fixture wiring. Reuse Node and native HTTP facilities; no new external dependencies, Rust workspace members or binding families. The related `share-ble-transport-policy` change is independent; implementing this first simplifies its software verification.

Non-goals: Linux implementation, moving simulation into production Rust, replacing the independent oracle with production codecs, removing focused codec vectors, changing physical scenario semantics, general host scheduling, or adding new production routes.
