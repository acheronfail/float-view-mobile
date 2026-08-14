## Context

`TelemetryPolling.swift` and `TelemetryPolling.kt` are line-for-line policy counterparts above native clocks/timers and the connection-wide VESC scheduler. They negotiate dynamic layouts, select legacy/realtime/compact requests, hold one semantic telemetry request pending, recover timeouts, change polling cadence around riding, cache the latest compact record, and map accepted records into sequenced application samples. The safe Rust workspace already contains the legacy and dynamic decoded models plus the request scheduler's route, priority, generation, response, and context vocabulary, but no component composes them.

This change is an unselected extraction. Production Swift/Kotlin behavior and every platform-owned capability remain unchanged until a later unified-runtime cutover. Its purpose is to establish one directly composable safe Rust application-service boundary rather than another protocol-specific host ABI.

## Goals / Non-Goals

**Goals:**

- Represent the complete duplicated telemetry coordination policy as a deterministic bounded Rust state machine.
- Compose existing safe Rust decoded telemetry types and scheduler facts without copying their wire decoders or arbitration rules.
- Produce exact route-correct request bytes/facts and ordered host effects while retaining no executor, callback, or foreign reference.
- Normalize both legacy and dynamic records, including active-fault semantics and bounded extensions, with exact cache/sequence/time behavior.
- Prove source parity, total behavior at public bounds, production isolation, and unchanged all-host behavior.

**Non-Goals:**

- Selecting Rust in Apple or Android, deleting native telemetry services, or adding any FFI/JNI bridge.
- Owning VESC frame encoding, BLE writes, clocks, timers, callback dispatch, persistence, SQLite, the local server, or an OS capability.
- Changing telemetry decoding, compatibility admission, frontend/backend contracts, persistence cadence, session lifecycle, or visible behavior.
- Building the final unified runtime in this step.

## Decisions

### Use one safe compositional crate

`refloat-telemetry-service` will depend only on `refloat-dynamic-telemetry`, `refloat-legacy-telemetry`, and `vesc-request-scheduler`. It will accept those crates' decoded samples directly and emit scheduler `RequestFacts`, so a later in-process Rust runtime does not need canonical bridge records between Rust components. Alternatives were a dependency-free duplicate model, which would create a second mapping layer, or a bridge-selected service, which would expand temporary ABI code before the unified runtime exists.

### Model host execution as explicit input and ordered effect

The service will accept monotonic milliseconds and observation epoch milliseconds on commands. It will emit bounded effects for timer replacement/cancellation, scheduler enqueue, normalized sample publication, stable failure, and redacted diagnostic. Timer ticks, scheduler completion, decoded responses, layout establishment, settings updates, stop, and reset return new ordered effects. No effect executes host work and no callback or trait object is retained.

Timer identities carry a monotonically wrapping nonce plus service generation so stale layout/poll ticks are ignored. Scheduler requests retain the selected scheduler generation. Arithmetic that represents physical time uses checked operations and rejects invalid input atomically; only source-equivalent sequence/token counters wrap explicitly.

### Preserve exact request and cadence policy

The component will produce `[36,101,32]` for dynamic layout, `[36,101,31]` for dynamic realtime, and `[36,101,10,4]` for legacy or compact detail-level-4 telemetry, wrapping once as `[34,can_id,...]` for a CAN route. Layout uses controller-work priority, command-32 response attribution, a 900 ms request deadline, immediate first attempt, 1,000 ms repeating retry, and five total attempts. Polling immediately requests a sample, uses 100 ms while riding or within the inclusive post-ride tail, otherwise the configured idle interval, and makes dynamic compact data due every 1,000 ms without an immediate timeout retry or parallel lane.

The service itself retains at most one semantic telemetry request pending. Its emitted request enters the independently bounded scheduler; completion input clears only the matching pending kind/generation. Mismatched, late, stale-generation, or repeated completion/response/timer input cannot advance unrelated state.

### Normalize from native-equivalent decoded semantics

A common bounded `NormalizedSample` owns the complete fields currently copied in both native services. Legacy fixed samples publish immediately and use themselves as slow data. Dynamic compact samples replace the entire latest compact cache and publish nothing; the next accepted realtime record merges only the established slow fields and carries the compact observation time. Decreasing device totals replace earlier values without clamping. Dynamic extensions preserve decoder order and remain within its 32-entry bound.

Package `Running` maps to riding and all other sample states to idle. Active legacy firmware faults preserve the current unknown-ride/fault-only semantics. Accepted publishable samples start or reuse the monotonic session origin, increment the source-equivalent sequence, clamp negative elapsed time to zero, update the inclusive post-ride deadline, and emit publication before any cadence restart effects, matching native callback order.

### Keep the candidate unselected until unified-runtime composition

Repository audits will forbid Apple/Android source references, bridge/catalog/build integration, ABI symbols, generated bindings, packaged artifacts, callbacks, threads, async runtimes, sockets, filesystem, SQLite, and OS dependencies. Swift and Kotlin remain the selected services and the physical gate exercises them unchanged. A later unified-runtime change will compose this state machine in Rust and delete native policy rather than create service-specific bridge crates.

## Risks / Trade-offs

- **[Decoded enums differ between legacy and dynamic crates]** → Perform explicit exhaustive conversion in the service and test every semantic enum, optional field, active fault, and extension boundary.
- **[A pure effect API can accidentally change callback/timer ordering]** → Derive vectors from both native implementations and assert exact ordered effects for starts, receives, cadence changes, timeout, stop, reset, and stale input.
- **[Request encoding could duplicate routing policy]** → Limit encoding to the four telemetry commands and exact single CAN-forward wrapper; test direct/CAN bytes and document it as telemetry request construction for later consolidation into the unified protocol runtime.
- **[The candidate initially adds code without deleting native code]** → Keep it unselected and bridge-free; the value is a directly composable application slice that a later runtime cutover can select and then delete both native copies.
- **[Physical background checks can be delayed by location authorization]** → Require every non-location checkpoint and record bounded location acquisition for the agreed later manual verification.
