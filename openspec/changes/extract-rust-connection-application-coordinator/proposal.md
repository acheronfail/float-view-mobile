## Why

Apple and Android still duplicate the connection-generation orchestration above Rust controller discovery, request scheduling, Refloat telemetry, and BMS telemetry. Extracting that composition spine now turns the isolated candidates into one coherent in-process application owner for the future unified Rust runtime without adding another temporary host bridge or changing production behavior.

## What Changes

- Add one safe bounded `backends/core/connection-application-coordinator` candidate that directly owns and composes Rust controller discovery, the connection-wide request scheduler, Refloat telemetry service, and VESC BMS telemetry service.
- Move connection-generation start/reset, exact discovery-effect translation, request enqueue/completion attribution, scheduler pumping, child timer routing, readiness sequencing, and ordered publication/diagnostic effects into one deterministic host-independent coordinator.
- Define a narrow runtime port expressed only as owned Rust commands and effects: the host supplies monotonic/epoch facts and decoded protocol results, while the coordinator returns BLE writes, deadline/timer work, routing-hint work, ready/selecting/failure state, telemetry samples, and BMS state/observations without executing any effect.
- Prove source-derived lifecycle parity and adversarial behavior across direct, CAN gateway, explicit selection, reconnect continuity, Refloat layout/telemetry startup, attached and standalone BMS, scheduler priority, timeout/cancellation, stale generations, reset, and rollover.
- Keep Swift and Kotlin selected and unchanged. Add no Apple FFI, Android JNI, callback, async runtime, BLE owner, persistence port, local server, generated binding, or packaged artifact; a later unified Rust runtime will call this coordinator directly and delete native orchestration plus temporary protocol bridges.
- Extend workspace tests and isolation audits, then run `just verify` and the connected Android/iOS/macOS ESP32 matrix, deferring only bounded location acquisition to the agreed manual follow-up.

Non-goals: production host cutover; deleting native application services or existing protocol bridges; owning protocol framing or every decoder; Refloat configuration/mutation integration; session or SQLite ownership; BLE, background, location, lifecycle, files, theme, loopback, WebView, or frontend-contract changes; or completing the final unified runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the safe connection application coordinator, its composed lifecycle parity, and its strict boundary below runtime execution.
- `platform-hosts`: Record candidate versus selected native orchestration ownership and the direct-composition/no-new-bridge path.
- `backend-conformance`: Require source-derived, adversarial, isolation, software, and all-host physical evidence before later runtime selection.

## Impact

The locked Rust workspace gains one unselected crate depending only on the existing safe controller-discovery, request-scheduler, Refloat telemetry-service, and VESC BMS telemetry-service components plus their directly exchanged safe models. Root verification gains focused lifecycle tests and ownership/dependency audits. Native packages, source selection, runtime behavior, backend contracts, SQLite, and OS capability ownership remain unchanged, so no existing bridge can yet be removed.
