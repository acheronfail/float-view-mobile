## Context

The selected Swift and Kotlin application services perform the same lifecycle above `AuxiliaryNodeDiscovery`: translate discovery work into scheduler submissions, request command-96 values approximately once per second after each completion, decode and own a live snapshot, count malformed/timeout/queue-full outcomes, become unavailable after three consecutive failures while retaining the snapshot, and recover on a valid current response. They also implement the standalone direct-BMS path with no ride session or persistence.

The safe Rust workspace now has all three lower-level owners required by that lifecycle: `auxiliary-node-discovery`, `vesc-bms-values`, and `vesc-request-scheduler`. Leaving them disconnected would force the future runtime to recreate native coordination. This change instead extracts one unselected, directly composable Rust application service while native production behavior remains unchanged.

## Goals / Non-Goals

**Goals:**

- Compose the existing safe discovery, decoder, and scheduler models without bridge records or duplicated compatibility/decoding/arbitration policy.
- Own the complete controller-attached and standalone BMS read lifecycle as a deterministic bounded command/state/effect machine.
- Emit exact direct/CAN request bytes and scheduler facts for scan, firmware probe, and values work while retaining no executor or callback.
- Own a complete bounded observation with exact route and observation-time attribution, failure threshold, stale retention, recovery, timer identity, generation isolation, and read-only safety.
- Prove source parity, adversarial totality, production isolation, and unchanged all-host behavior.

**Non-Goals:**

- Selecting Rust from Swift or Kotlin, deleting native lifecycle code, or adding a BMS service FFI/JNI bridge.
- Changing lower-level identity admission, command-96 wire decoding, scheduler arbitration, frontend contracts, persistence, sessions, or visible BMS behavior.
- Owning a clock, timer executor, BLE writer, callback, SQLite, local server, transport, frame codec, thread, async runtime, or OS capability.
- Building or cutting over the final unified runtime.

## Decisions

### Directly compose the three safe Rust owners

`vesc-bms-telemetry-service` will depend only on `auxiliary-node-discovery`, `vesc-bms-values`, and `vesc-request-scheduler`. It will contain `AuxiliaryNodeDiscovery<BmsObservation>`, accept already decoded routed identities and CAN inventories for discovery, decode command-96 bytes through the existing decoder, and emit scheduler `RequestFacts`. A duplicate dependency-free service was rejected because it would fork route, status, bounds, and observation policy; a host-selected service was rejected because it would add two more temporary bridges.

### Use explicit commands and ordered effects

Public commands carry service generation, auxiliary generation, scheduler generation, positive observation epoch time, route, request kind, completion, decoded discovery facts, or payload bytes. Timer execution remains host-owned, so the candidate accepts no physical monotonic clock value. Transitions return ordered effects for scheduler enqueue, one-shot timer replacement/cancellation, public-state publication, observation publication, stable failure, and redacted diagnostic. No effect executes work and the service retains no host reference.

Timer tokens contain service generation, auxiliary generation, route, and a wrapping nonce. Public validation rejects CAN routes above 253, non-positive observation epoch time, mismatched routes, oversized discovery input through the composed owner, and malformed decoded observations without partial mutation. The full positive `i64` epoch range is accepted without arithmetic. Stale, repeated, mismatched, or prior-generation commands cannot advance current work.

### Preserve exact scheduler and request behavior

CAN discovery emits command 62 with no route, auxiliary firmware probing emits command 0 wrapped once as `[34,can_id,0]`, and values polling emits `[96]` directly or `[34,can_id,96]`. All use auxiliary priority, their exact request contexts, the selected scheduler generation, response-bearing facts, and native-equivalent 10,000 ms scan or 1,000 ms probe/values deadlines.

Scheduler response completion and decoded payload delivery remain separate inputs, matching current host order. A values response/sent completion schedules the next one-shot 1,000 ms poll; timeout or queue-full also records one failure and schedules the next poll; cancelled or stale-generation completion schedules nothing. Scan/probe timeouts and scheduling failures advance the composed discovery failure/next-probe policy. Exact response context and route attribution prevent late work from affecting another request.

### Own concrete observations and bounded recovery

`BmsObservation` will copy every bounded decoded command-96 field, cells, balancing flags, temperatures, optional data version/status, expected route, and observation epoch milliseconds into owned storage. Debug output redacts all values and status text. A valid current values response resets consecutive failures, attaches/replaces the observation, and recovers an unavailable route. Malformed response, timeout, queue-full, or immediate enqueue rejection increments a counter capped at three; the third marks the selected route unavailable while preserving its last observation and later polls continue so recovery remains possible.

The service publishes no observation on identity alone. Reset or replacement generation cancels current timer state, clears live discovery/observation/failure state, and invalidates old scheduler/timer/response work. Standalone direct BMS uses the same service path but produces no session, location, or persistence effect.

### Keep production isolation until unified-runtime selection

Repository audits will forbid Apple/Android references, ABI symbols, bridge/catalog/build integration, generated bindings, callbacks, executors, filesystem, sockets, SQLite, and packaged artifacts. Native services remain selected and physical tests exercise them unchanged. A later unified runtime will call this service in-process and delete the native BMS lifecycle plus command-96 bridges when no native protocol boundary remains.

## Risks / Trade-offs

- **[Scheduler completion can arrive before decoded payload delivery]** → Model them as separate generation-qualified commands and test both orders, repeats, malformed payloads, and late responses.
- **[Failure accounting could double-count one malformed response]** → Treat response completion as cadence-only; only decoding failure contributes for that response, while timeout/queue-full contribute at completion.
- **[Owned observations duplicate decoded borrowed fields]** → Keep one exhaustive conversion at the application boundary, preserve decoder bounds, and test every field plus maximum cells, temperatures, and status length.
- **[Request construction repeats a small forward-CAN wrapper]** → Limit it to the three exact read-only service commands, test every byte, and leave consolidation to the unified protocol runtime.
- **[The candidate temporarily adds code beside native copies]** → Keep it unselected and bridge-free; later runtime selection deletes both native lifecycles rather than maintaining three production paths.
- **[iOS lifecycle tooling may finish XCTest without exiting `xcodebuild`]** → Record the passed XCTest and continuous background fixture evidence explicitly while requiring all other non-location checkpoints.

## Migration Plan

1. Add and verify the unselected safe service with source-derived and adversarial tests; leave host packages unchanged.
2. Compose it with the Rust connection/application runtime in a later OpenSpec change alongside the existing controller discovery, request scheduler, and Refloat telemetry service.
3. Cut over each host only after the unified runtime owns the complete production path, then delete native BMS discovery/polling and temporary command-96 bridges.

Rollback is removal of the unselected crate, workspace/audit entries, and this change's canonical requirements; selected host behavior is unaffected.

## Open Questions

None. Host runtime selection, the final runtime boundary, SQLite ownership, and bridge deletion remain deliberate later changes.
