## Why

Apple and Android still duplicate the sole connection-scoped authorization and serialization owner for Refloat controls, controller shutdown, configuration writes, and Tune apply. Extracting that safety-critical state machine is the necessary next application-behavior step before the existing Rust configuration service can join the connection coordinator without retaining a native gate or creating a parallel write path.

## What Changes

- Add one safe bounded `backends/core/device-mutation-coordinator` Rust candidate that owns current-generation readiness, exact compatibility and writable admission, authoritative telemetry/control observations, one-pending-mutation serialization, idempotence, confirmation, failure, cancellation, and reset.
- Directly use the existing safe Rust Refloat control codec and compatibility models to authorize and construct package, flywheel, lighting, fatal-clear, and controller-shutdown work, while representing configuration-write exclusivity as a generation-qualified lease for later configuration composition.
- Preserve the exact safety matrix: idle/package/footpad/flywheel requirements, active-flywheel disable exception, test-firmware read-only behavior, shutdown support, fatal-latch versus active-fault rules, lighting confirmation, stale-generation rejection, and telemetry-before-exclusion confirmation order.
- Add shared source-derived vectors and exhaustive adversarial tests for every mutation family, competing work, idempotence, confirmation timing, malformed lighting responses, stale callbacks, reset, bounds, privacy, and counter rollover.
- Keep Swift and Kotlin selected and unchanged. Add no Apple FFI, Android JNI, callback, executor, scheduler owner, BLE owner, persistence port, local server, generated binding, or packaged artifact; the connection application coordinator will compose this safe owner in a later change.
- Extend workspace tests and isolation audits, then run `just verify` and the connected Android/iOS/macOS ESP32 matrix, deferring only bounded location acquisition to the agreed manual follow-up.

Non-goals: production host cutover; deleting either native mutation coordinator or any protocol bridge; integrating the Rust configuration service into the connection coordinator; owning scheduler execution, VESC framing, BLE, clocks, SQLite, sessions, loopback serving, background/location/files/theme/UI behavior; changing frontend contracts or physical behavior; or completing the unified runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the safe device-mutation coordinator candidate, its complete cross-feature safety parity, and its boundary below runtime execution.
- `platform-hosts`: Record candidate versus selected native mutation ownership and the direct-composition/no-new-bridge path.
- `backend-conformance`: Require source-derived, adversarial, isolation, software, and all-host physical evidence before later runtime composition.

## Impact

The locked Rust workspace gains one unselected crate depending only on existing safe Rust protocol/policy models, with focused fixtures and tests. Root verification gains mutation-safety and production-isolation audits. Native packages, source selection, application behavior, frontend contracts, SQLite, and OS capability ownership remain unchanged, so no native coordinator or bridge can yet be removed. The following migration slice can then compose this owner and `refloat-configuration-service` directly into `connection-application-coordinator` without duplicating authorization.
