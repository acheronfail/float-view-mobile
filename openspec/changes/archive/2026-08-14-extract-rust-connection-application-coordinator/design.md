## Context

The selected Swift and Kotlin application services own the same connection spine: start a scheduler generation, identify the direct transport, recover or enumerate a controller route, negotiate Refloat, establish telemetry, transition through selecting/ready/failed, start auxiliary BMS discovery, attribute request completions, and reset every connection-scoped owner together. The safe Rust workspace now contains the complete lower-level decision owners for controller discovery, request arbitration, Refloat telemetry, and VESC BMS telemetry, but a future runtime would still have to recreate the duplicated native glue between them.

This change extracts that glue as one unselected safe Rust coordinator. It is intentionally the first application-composition layer, not a platform backend: native services continue to execute BLE, clocks, timers, persistence, background work, and publication while the candidate proves the deterministic state/effect model needed by a later unified runtime.

## Goals / Non-Goals

**Goals:**

- Directly compose the existing safe discovery, scheduler, Refloat telemetry, and BMS telemetry owners without bridge records or callbacks.
- Own one connection/application generation, scheduler response attribution, discovery-to-readiness sequencing, child-service startup/reset, and ordered host effects.
- Decode only the response families required to connect those safe owners by calling the existing Rust firmware, Refloat compatibility, legacy telemetry, and dynamic telemetry APIs directly.
- Preserve native-equivalent direct, gateway, explicit-selection, reconnect-continuity, standalone-BMS, layout, telemetry, timeout, cancellation, and reset behavior.
- Leave an explicit ready-activation handshake so persistence/session work remains outside the coordinator until the Rust persistence/runtime changes.
- Prove source parity, adversarial totality, production isolation, and unchanged all-host behavior.

**Non-Goals:**

- Selecting the candidate from Swift/Kotlin, deleting native orchestration, or adding a coordinator FFI/JNI bridge.
- Owning VESC frame/stream transport, BLE, clocks, timer execution, SQLite, sessions, files, loopback serving, frontend contracts, threads, or OS capabilities.
- Integrating Refloat configuration, device mutations, controller current-limit reads, durable telemetry writes, reconnect backoff, or local-webserver behavior in this slice.
- Completing the unified Rust runtime or removing existing protocol bridges yet.

## Decisions

### Directly own the four stateful candidates and required safe decoders

`connection-application-coordinator` will own `ControllerDiscovery`, `VescRequestScheduler`, `RefloatTelemetryService`, and `VescBmsTelemetryService`. It will call the existing firmware-identity, Refloat compatibility, dynamic-telemetry, and legacy-telemetry APIs for scheduler-attributed responses. Those eight safe crates are the complete production dependency set; no host support or bridge crate is allowed.

Alternative: exchange canonical C/JNI records or host-decoded events. Rejected because it would preserve the temporary bridge boundary inside the final Rust runtime and leave response attribution duplicated. Alternative: merge the existing crates into one large crate. Rejected because their independently tested downward ownership remains useful and the coordinator should contain composition rather than copies.

### Use one closed command/effect boundary with no execution

The coordinator accepts bounded commands for begin, select-controller, response bytes, deadline expiry, child timer firing, activation, settings/session facts, refresh, and reset. Commands carry current generation, monotonic time, positive observation epoch time where needed, and exact timer/deadline identity. Effects contain only owned work: arm/cancel scheduler deadlines, write one unframed VESC payload, replace/cancel one child timer, clear/store a routing hint, publish selecting/ready/failed state, publish normalized telemetry or BMS state/observation, and emit stable redacted diagnostics.

The host remains responsible for VESC framing, BLE writes, physical clocks/timers, persistence, and frontend publication. No effect invokes a callback or retains a host reference. Alternative: an async trait-based runtime inside this crate. Rejected because it would prematurely choose executors, lifetimes, and OS adapters before the local server and persistence are migrated.

### Drain child and scheduler effects synchronously in source order

Discovery effects are converted to exact scheduler `RequestFacts`. Child `Enqueue` effects are submitted immediately to the owned scheduler. Scheduler effects are then mapped to outward deadline/write work or recursively dispatched completions before `resume` pumps the next request. This preserves one in-flight response request, priority/FIFO arbitration, completion-before-next-write ordering, and atomic transition output without a parallel queue.

The coordinator owns the small exact read-only discovery payloads: firmware `[0]`, CAN ping `[62]`, Refloat INFO `[36,101,0,2,0]`, and one checked `[34,can_id]` forwarding prefix. Telemetry and BMS request bytes remain produced by their child services. Every response is matched by the scheduler before its context selects a decoder or child command.

### Model readiness as prepare then activate

Controller discovery reaching ready produces an `AwaitingActivation` snapshot/effect with selected identity, route, profile, and classified inventory. The host or future persistence owner must answer with a generation-qualified `Activate` command after any durable session/continuity work succeeds. Activation starts Refloat telemetry and attached BMS discovery using current settings and supplied BMS preference, then publishes ready. A failed or stale activation changes nothing.

An exact direct standalone BMS bypasses controller session creation, enters ready immediately, and starts only the BMS service. It emits no Refloat telemetry, session, location, persistence, configuration, mutation, or controller-control effect.

Alternative: let the candidate create sessions through a storage trait. Rejected because that would couple this first coordinator slice to the still-native SQLite model and make rollback/cutover larger.

### Keep one generation and reject all stale work

`Begin` advances both coordinator and scheduler generations, resets all children, cancels prior deadlines/timers through ordered effects, validates settings/routes/hints/continuity atomically, and starts direct identification. Request contexts, scheduled sequence, deadline tokens, child timer tokens, discovery generations, and coordinator generation remain exact attribution keys. Reset advances invalidation state, drains cancellations, clears layouts/observations/public state, and is idempotent.

Counter rollover is explicit wrapping where lower components already define it. Monotonic additions remain checked; invalid time, route, payload, or configuration input returns a stable error with no partial mutation.

### Keep production isolation until a runtime cutover

The candidate adds no bridge symbol, build-catalog entry, SwiftPM product, Gradle task, package input, callback, generated binding, or runtime fallback. Repository audits require native application services to remain selected and forbid platform, persistence, network, filesystem, and executor dependencies. Physical tests therefore exercise unchanged production paths.

A later change will add the platform-neutral runtime shell around this coordinator and move persistence/local-server application ports into Rust. Host cutovers then retain only OS adapters and delete superseded Swift/Kotlin application orchestration plus all protocol bridge crates in one proven path.

## Risks / Trade-offs

- **[One coordinator transition can emit nested effects from several children]** → Use one iterative drain with a finite bound derived from scheduler queue and child transition maxima; preserve exact insertion order and fail atomically before publication if the bound is exceeded.
- **[Response completion and decoding failure can double-count or reorder work]** → Scheduler attribution happens once; completion is delivered to the child before decode delivery only where the child contract requires it, and malformed payload handling is tested for each context.
- **[Dynamic telemetry depends on the current admitted layout]** → Store one generation-scoped safe `DynamicLayout`, clear it on reset/replacement, admit it through Refloat compatibility, and reject realtime bytes without the current layout.
- **[The activation handshake may diverge from native session ordering]** → Derive vectors from both selected services and require no telemetry/BMS start or ready publication before the exact current activation command.
- **[The candidate temporarily adds application code beside native copies]** → Keep it unselected and bridge-free; later runtime selection deletes both native orchestration paths rather than maintaining three production implementations.
- **[iOS lifecycle tooling may finish XCTest without exiting `xcodebuild`]** → Record passed XCTest and continuous background fixture evidence explicitly while requiring all other non-location checkpoints.

## Migration Plan

1. Add and exhaustively verify the unselected coordinator while native hosts remain unchanged.
2. Add a later platform-neutral Rust runtime shell that executes coordinator effects through narrow OS adapters and integrates Rust persistence/local serving in separate changes.
3. Cut over Apple and Android only after the complete runtime path passes their software and physical gates; delete native application orchestration and all superseded protocol bridge crates rather than retaining compatibility layers.

Rollback is removal of the unselected crate, workspace/audit entries, and this change's canonical requirements; selected host behavior is unaffected.

## Open Questions

None. Refloat configuration/mutation composition, SQLite/session migration, local-server migration, host runtime selection, and bridge deletion remain deliberate later OpenSpec changes.
