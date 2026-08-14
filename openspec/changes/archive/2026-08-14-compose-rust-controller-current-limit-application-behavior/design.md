## Context

The selected Swift and Kotlin application services perform one remaining connection-scoped read that the Rust connection coordinator does not yet own. Immediately after a supported controller becomes ready, each native service submits route-correct `COMM_GET_MCCONF` command 14 through the connection-wide scheduler, attributes the response to the selected direct/CAN route, decodes four current limits plus an optional battery-series-cell count, publishes them in live connection state, and clears them on reset.

The safe Rust workspace already contains every required lower layer: `vesc-request-scheduler` has the exact `CurrentLimits(Route)` context, and `vesc-controller-data` owns bounded command-14 decoding. Leaving their composition in Swift/Kotlin would require the future Rust runtime to recreate native request ordering, response attribution, and live-state ownership. This change completes that composition while the coordinator remains unselected and bridge-free.

## Goals / Non-Goals

**Goals:**

- Make the Rust connection application coordinator the complete safe owner of the current-limit request, attribution, decode, publication, and reset lifecycle.
- Directly consume `vesc-controller-data::CurrentLimits` and its decoder without bridge records, copied auto-float/signature logic, or a second application model.
- Preserve exact direct/CAN routing, scheduler priority, timeout, activation order, standalone-BMS exclusion, soft failure behavior, and generation isolation from both native services.
- Retain one optional bounded current-limit value only for the current ready controller and expose it through the coordinator snapshot with redacted diagnostics/debug output.
- Prove source parity, adversarial totality, production isolation, complete software verification, and unchanged all-host behavior.

**Non-Goals:**

- Selecting the Rust coordinator from Swift/Kotlin, deleting native current-limit behavior, or adding a coordinator/controller-data FFI or JNI boundary.
- Moving durable telemetry writes, sessions, SQLite, local serving, reconnect execution, BLE/framing, clocks/timers, or OS capabilities.
- Using the standard command-4 realtime decoder, changing the frontend contract, changing current-limit semantics, or adding periodic current-limit polling.
- Removing the existing protocol bridges before the complete Rust runtime is selected by each host.

## Decisions

### Depend directly on the existing safe controller-data owner

`connection-application-coordinator` will add the existing dependency `vesc-controller-data` and retain its `CurrentLimits` value directly. The coordinator will not introduce a duplicate current-limit struct, translate through Apple/JNI records, or copy command-14 decoding. This dependency replaces the only missing safe composition seam and adds no third-party code.

Alternative: define a coordinator-local four-float record. Rejected because that would create another mapping layer and preserve bridge-shaped data inside the future in-process runtime. Alternative: defer decoding to a future host adapter. Rejected because both request attribution and semantic decoding belong below runtime execution and already have safe Rust owners.

### Enqueue one current-limit read in native activation order

After configuring mutation and Refloat configuration children, activation will enqueue one command-14 request before starting telemetry and auxiliary BMS behavior. The request uses controller-work priority, the existing CAN-probe timeout, exact scheduler generation, `CurrentLimits(route)` context, expected command 14, and either `[14]` or one checked `[34, can_id, 14]` payload.

This ordering preserves the native behavior: configuration may enqueue its first controller-work request first, current limits then enter the same FIFO, and higher-priority telemetry remains governed by the existing scheduler. Standalone BMS never reaches controller activation and therefore emits no command-14 work.

Alternative: add a new controller-data child service. Rejected because this is a one-shot application read with no independent cadence or state machine; the connection coordinator already owns its activation, scheduler, and reset lifetime.

### Attribute and publish only one current ready result

The existing scheduler remains the sole response gate. A response is decoded only after the exact current request completes under `CurrentLimits(route)`. A valid value replaces the coordinator's optional current-limit state and causes one deterministic state publication while the coordinator is still ready. Direct/CAN route and generation mismatches, stale or repeated responses, and unrelated command families never reach this transition.

The public snapshot will expose `Option<vesc_controller_data::CurrentLimits>`. Its custom `Debug` implementation will report only presence, never numeric values. No additional publication effect is needed because the existing bounded `PublishState` effect already represents application state changes.

Alternative: emit a separate current-limit effect for a future runtime to merge. Rejected because that would recreate the live-state composition this change is meant to move into Rust.

### Treat read failure as unavailable, not connection failure

Malformed command-14 payloads produce one stable payload-free current-limit diagnostic and leave the optional value absent. Timeout, queue-full, cancellation, stale generation, and reset likewise publish no stale value and do not fail the controller connection. This matches selected native behavior where current limits are optional UI enrichment rather than readiness criteria.

Begin/reset clears the retained value before any new generation can publish. The existing scheduler cancellation and generation rules invalidate in-flight work atomically. Maximum payload and numeric validation remain owned by `vesc-controller-data`; coordinator diagnostics never render request or decoded values.

### Keep production selection and runtime execution unchanged

The candidate adds no exported symbol, bridge crate, build-catalogue entry, SwiftPM/Xcode/Gradle input, callback, generated binding, persistence/network dependency, or fallback. Repository audits require both native application services and their selected controller-data bridges to remain unchanged until a later complete runtime cutover proves production Rust execution.

## Risks / Trade-offs

- **[Adding the request can change scheduler ordering]** → Assert exact activation effects and request order for cache hit/miss, direct/CAN, telemetry, configuration, and BMS vectors against both native sources.
- **[A malformed response could leak prior-generation limits]** → Clear on begin/reset, decode only scheduler-attributed current-generation work, and test stale/repeated/mismatched completions after reconnect.
- **[Current-limit floats could appear in diagnostics]** → Reuse the decoder's redacted `Debug`, customize coordinator snapshot/debug output to expose only presence, and add adversarial formatting tests.
- **[Physical tests still execute native owners]** → Treat them as regression/isolation evidence only; later runtime selection and host cutovers must prove production coordinator execution before deleting native code.

## Migration Plan

1. Add the direct dependency, retained optional state, activation request, completion dispatch, and source-derived/adversarial tests to the unselected Rust coordinator.
2. Keep Apple and Android production paths unchanged and run focused audits, `just verify`, strict OpenSpec validation, and the complete macOS/Android/iOS ESP32 matrix.
3. In later changes, introduce the platform-neutral Rust runtime/persistence boundary, move SQLite/session and loopback serving into Rust, cut over each host, then delete superseded native application owners and all unused `*-apple-ffi`/`*-android-jni` crates.

Rollback is removal of the controller-data dependency, optional snapshot field, activation/completion logic, tests, and this change's requirements. No selected code or durable format changes in this slice.

## Open Questions

None. Runtime execution, SQLite/session ownership, loopback serving, host selection, and bridge deletion remain separate increments.
