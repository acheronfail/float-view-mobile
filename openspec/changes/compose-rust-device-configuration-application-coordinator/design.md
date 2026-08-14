## Context

The unselected safe Rust `connection-application-coordinator` already owns discovery, request scheduling, Refloat telemetry, and BMS telemetry, but it does not yet own application mutations or configuration. Those behaviors exist as two independently tested safe candidates: `device-mutation-coordinator` is the sole authorization/serialization gate, while `refloat-configuration-service` owns schema, values, write, verification, and cache decisions. Swift and Kotlin currently provide the missing composition: they initialize both owners from the selected controller, acquire one mutation lease before configuration writes, turn configuration effects into commands 92–95, attribute scheduler completions, and release or fail the lease.

Leaving that glue outside Rust would preserve duplicated application behavior in the final runtime. This change composes the existing safe owners directly inside the Rust connection coordinator while keeping the entire coordinator unselected by production hosts. Native services continue to own every execution and OS boundary.

## Goals / Non-Goals

**Goals:**

- Directly own `DeviceMutationCoordinator` and `ConfigurationService` inside `ConnectionApplicationCoordinator` with the existing scheduler and telemetry owners.
- Configure and reset all connection-scoped children atomically from one selected controller generation.
- Expose bounded commands for semantic controls, shutdown, configuration reads/writes, Tune leases, scheduler completion, and cache-backed activation.
- Convert all child work into exact safe scheduler facts and decode attributed commands 92–95 through the existing custom-configuration codec.
- Maintain one exact mutation token across an entire configuration write/verified-readback transaction and release it on every success, failure, cancellation, flywheel transition, reconnect, and reset path.
- Return deterministic state, cache, persistence-intent, request, failure, and diagnostic effects without executing them.
- Prove parity, bounds, production isolation, complete software verification, and unchanged Android/iOS/macOS behavior.

**Non-Goals:**

- Selecting this coordinator in Swift or Kotlin, adding an FFI/JNI wrapper, or deleting selected native application owners.
- Executing BLE/framing, clocks/timers, cache storage, SQLite, sessions, persistence, loopback/web serving, filesystem, network, async runtime, or OS capabilities.
- Migrating durable telemetry writes, current-limit application behavior, local-server behavior, or the frontend contract.
- Changing wire bytes, scheduler priority, database schema, or user-visible behavior.

## Decisions

### Extend the existing connection coordinator instead of adding another composition crate

`ConnectionApplicationCoordinator` will directly own the mutation and configuration candidates. Its public snapshot will include their bounded snapshots, and its command/effect enums will gain the application operations and returned cache/publication work required by a future runtime. This keeps one connection generation, scheduler, response dispatcher, and reset order.

Alternative: add a peer `device-configuration-application-coordinator` above the current connection coordinator. Rejected because it would require proxying every response, telemetry sample, generation, and scheduler effect across another layer and would preserve the glue this change is intended to remove.

### Activate application children only after the existing persistence handshake

Discovery continues to stop at `AwaitingActivation`. The generation-qualified activation command will carry host-loaded bytes for the exact derived configuration cache identity, when available. After activation, the coordinator configures the mutation owner from selected route, firmware identity, admitted Refloat profile, and readiness; begins the configuration service from firmware/package/cache facts; then starts telemetry and BMS in source-equivalent order. Standalone BMS never starts mutation or configuration behavior.

The coordinator emits cache store/remove effects but never reads or writes storage. Alternative: pass a cache trait into the coordinator. Rejected because callbacks and persistence ports would prematurely create the runtime boundary and complicate later SQLite ownership.

### Use safe internal types and explicit adapters, never bridge records

The coordinator adds direct safe dependencies on the mutation coordinator, configuration service, configuration value model, and VESC custom-configuration codec. Small private adapters convert the already validated discovery route/generation/identity/profile into each child’s types. There is no serialization between safe Rust crates and no new C/JNI symbol, generated binding, callback, or opaque handle.

The custom-configuration codec remains the sole owner of commands 92–95 envelopes. The configuration service remains the sole owner of schema/value semantics. The mutation coordinator remains the sole owner of authorization and exclusivity. The connection coordinator owns only their ordered composition.

### Attribute configuration work through the existing scheduler

Each `RequestEffect` becomes one exact `RequestFacts` with controller-work priority, the existing configuration context, current scheduler generation, and native-equivalent timeout. Direct and CAN routes use the existing checked route wrapper exactly once. Scheduler completion first selects the typed context, then decodes a response through `vesc-custom-configuration`, converts it into the service response/context, and drains the next service transition before the scheduler resumes.

Timeout, queue-full, cancellation, stale-generation, malformed response, and enqueue failure notify the configuration service and mutation owner exactly once. Diagnostics remain stable and payload-free. Alternative: let the configuration service own a scheduler instance. Rejected because the connection-wide scheduler is the arbitration source of truth and configuration must compete with telemetry, discovery, BMS, controls, and shutdown.

### Hold one mutation lease for the complete write transaction

Configuration reads do not acquire the mutation lease. A configuration write or Tune apply first obtains an exact mutation token; only then may the configuration service create command-95 work. The token is retained through acknowledgement and command-93 verification, and released only after verified success. Any synchronous codec/service failure or asynchronous scheduler failure aborts the operation and fails/releases the exact token.

Control, fatal-clear, lighting, and shutdown requests are submitted from mutation effects through the same scheduler. Current normalized telemetry is delivered to the mutation owner before application publication so confirmation and flywheel prohibition preserve source order. Configuration receives the authoritative flywheel transition, aborting prohibited current/write work and releasing its token where required.

### Bound every transition and preserve atomic reset

The existing finite effect limit will cover nested mutation, configuration, scheduler, telemetry, and BMS effects. Every public byte/string/collection input remains bounded by its owning safe component; invalid data produces a stable error before partial public work. `Begin` and `Reset` cancel scheduler work, reset all children, invalidate tokens/operations, clear cached in-memory schema/value state, and publish one current-generation snapshot in deterministic order.

## Risks / Trade-offs

- **[The enlarged command/effect surface could duplicate child semantics]** → Re-export or wrap only composition-level operations; keep all authorization, protocol, and schema decisions inside their existing safe owners and audit for copied constants/logic.
- **[A scheduler failure could leave a configuration lease stuck]** → Test every completion class and synchronous encoding/enqueue failure, requiring exact service abort plus token-qualified release/failure before the next mutation is admitted.
- **[Telemetry confirmation and flywheel cancellation could reorder effects]** → Derive vectors from both native application services and assert mutation observation, configuration transition, scheduler cancellation, failure, and state publication order.
- **[Cache bytes supplied at activation could be stale or oversized]** → Derive the cache identity inside the configuration service, apply its existing bounded decode/catalogue validation, and emit removal for corrupt exact-key data without exposing bytes in diagnostics.
- **[Physical tests still execute native owners]** → Treat them as regression/isolation evidence only; later runtime selection and host-cutover changes must prove production Rust execution before deleting native code.

## Migration Plan

1. Extend the unselected Rust connection coordinator with direct mutation/configuration composition and exhaustive source-derived tests.
2. Keep both native hosts unchanged and run the complete software and physical gates.
3. In later increments, add remaining connection application behavior, then Rust persistence/session ownership and the loopback webserver/runtime shell.
4. Select the complete Rust runtime per host only after its full gates pass; retain thin OS adapters and delete the superseded native application owners plus all `*-apple-ffi` and `*-android-jni` protocol bridge crates.

Rollback is removal of the added coordinator dependencies, commands/effects/adapters/tests, and this change’s requirements. No selected host or durable format changes in this slice.

## Open Questions

None. Cache execution, SQLite/session migration, local-server migration, runtime selection, and bridge deletion remain explicit later changes.
