## Why

The safe Rust connection coordinator, configuration service, and device-mutation coordinator still stop at separate value boundaries, so a future runtime would have to recreate the duplicated native glue that serializes configuration against controls and shutdown, converts configuration work into scheduler requests, attributes responses, and publishes/cache-effects results. Composing those existing owners now is the smallest application-behavior increment that makes the final in-process Rust runtime more complete without introducing another temporary Apple FFI or Android JNI surface.

## What Changes

- Extend `connection-application-coordinator` to directly own the safe Rust device-mutation coordinator and Refloat configuration service alongside its existing scheduler, discovery, telemetry, and BMS owners.
- Configure both children from the exact selected controller generation and activate configuration only after the existing persistence/session handshake.
- Route semantic controls, fatal clear, controller shutdown, configuration reads/writes, Tune leases, telemetry/lighting confirmation, failures, cancellation, and reset through one generation-scoped coordinator command/effect boundary.
- Encode configuration commands 92–95 into exact scheduler request facts, attribute every completion back to its operation/token, preserve one mutation lease across write acknowledgement and verified readback, and emit bounded cache/publication effects for a future Rust runtime to execute.
- Add source-derived composition vectors, exhaustive adversarial tests, dependency/production-isolation audits, the complete software gate, and unchanged Android/iOS/macOS ESP32 regression evidence.
- Remove no selected native code yet; this unselected composition is a prerequisite for later runtime and host-cutover changes that delete the native application owners and temporary protocol bridge crates.

Non-goals: selecting the Rust coordinator from Swift or Kotlin; adding coordinator/configuration/mutation FFI or JNI; deleting native configuration or mutation owners; executing BLE, VESC framing, clocks, timers, persistence, SQLite, sessions, cache storage, loopback serving, background/location/files/theme/UI behavior; changing the frontend contract, database schema, physical behavior, or completing the unified runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define direct, safe composition of configuration and mutation application behavior into the Rust connection coordinator, including scheduler attribution and generation-safe lifecycle behavior.
- `backend-conformance`: Require source-derived and adversarial composition evidence, production-isolation audits, complete software verification, and unchanged all-host physical behavior.
- `platform-hosts`: Record the expanded unselected Rust ownership boundary while retaining selected native application and OS-capability ownership until later host cutovers.

## Impact

- Affected Rust code: `backends/core/connection-application-coordinator`, its direct dependencies/tests, shared connection/configuration fixtures, workspace documentation, and audit recipes.
- Affected specifications: `native-core-migration`, `backend-conformance`, and `platform-hosts`.
- No cross-platform wire-contract, host package, native API, persistence format, dependency download, or production runtime selection changes.
