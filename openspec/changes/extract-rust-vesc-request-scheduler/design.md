## Context

Both production backends contain the same connection-wide scheduler above BLE writes and below controller discovery, telemetry polling, BMS polling, controls, and configuration. It owns generation replacement, a bounded priority queue, FIFO order within a priority, one response-bearing request in flight, typed response attribution, deadlines, one-way completion, cancellation, and deferred pumping. The differences are host timer and byte-container mechanics rather than policy.

The scheduler is a better first shared application-behavior boundary than a complete application coordinator: it is already isolated and exhaustively mirrored, has no persistence model, and is the common execution point for several Rust protocol candidates. Selecting it through another long-lived FFI/JNI object now would increase migration scaffolding. This change therefore extracts an unselected safe Rust state machine that can later be composed directly inside a unified Rust runtime.

## Goals / Non-Goals

**Goals:**

- Define one safe Rust scheduler state machine with bounded platform-neutral requests, contexts, generations, completions, and ordered effects.
- Match every selected Swift/Kotlin arbitration, attribution, cancellation, timeout, defer/resume, and queue-bound behavior with deterministic tests.
- Keep clock reads, timer arming/cancellation, BLE writes, diagnostics, and completion delivery outside the candidate while making their required ordering explicit.
- Create the connection-wide composition point for later Rust telemetry, discovery, BMS, control, and configuration services.
- Prove that the isolated candidate changes no production package, runtime path, symbol, or physical behavior.

**Non-Goals:**

- Selecting the candidate in Apple or Android production or deleting either native scheduler.
- Adding C/JNI exports, callbacks, opaque handles, generated bindings, or platform adapter crates.
- Moving BLE, timer execution, protocol framing/decoding, application publication, SQLite, loopback transport, frontend models, files, location, lifecycle, or background services.
- Changing request priorities, queue limits, response matching, timeout values, or any frontend/persistence contract.

## Decisions

### Use a pure transition owner with explicit ordered effects

`vesc-request-scheduler` will accept commands such as begin generation, enqueue, receive, expire deadline, cancel matching, and resume. Each command receives the current monotonic time where needed and returns an ordered bounded list of effects: write bytes, arm/cancel a deadline token, complete a request, or emit a stable diagnostic. The crate retains scheduler state but no host object or executable callback.

Alternative: define clock, timer, writer, and completion traits. Rejected because host calls from inside the candidate obscure ordering, complicate deterministic testing, and become an unnecessary foreign-runtime contract during cutover.

### Use opaque typed contexts rather than protocol-specific dependencies

The candidate will define the closed request-context variants and bounded route/index/operation facts needed for matching and cancellation, but it will not depend on any protocol codec or construct payloads. Payload bytes and expected command identifiers enter as validated request facts. This preserves the current application-layer attribution contract without coupling the scheduler downward to every protocol crate.

Alternative: make contexts untyped numeric tags. Rejected because the selected implementations rely on exact route and configuration-operation identity to prevent misattribution and cancel only matching work.

### Represent deadline identity explicitly

Every dispatched response-bearing request will receive a monotonically changing deadline token. Expiry is accepted only when the token still identifies the current in-flight request and generation. Hosts will eventually map arm/cancel effects to native timers, but this candidate's tests can inject stale, repeated, or out-of-order expiry events safely.

Alternative: expire by request sequence alone. Rejected because a replaced generation or cancelled/reused request must not let a late host timer advance the new queue.

### Preserve priority with FIFO sequence order

The queue selects highest priority first and earliest wrapping sequence within equal priority, while the currently dispatched request is never preempted. Sequence and generation counters use explicit wrapping transitions so the state machine remains total at integer boundaries. Queue capacity excludes the in-flight request exactly as the native implementations do.

Alternative: use a heap. Rejected because the queue is capped at 64 and a bounded vector makes stable tie-breaking and source parity clearer without dependencies.

### Prove parity before any host integration

Tests will replay vectors derived from both native suites and add adversarial sequences for every context, maximum payload/queue bounds, stale responses and deadlines, cancellation/defer/resume ordering, one-way chains, and counter rollover. Repository audits will prove that neither host package, bridge catalogue, exported-symbol allowlist, nor application call path selects the candidate.

## Risks / Trade-offs

- **The candidate and native schedulers coexist temporarily** → Keep it unselected, audit production isolation, and require separate host/runtime cutovers before native deletion.
- **Effect application order can be mishandled by a future adapter** → Make transition effects ordered and cover cancellation-before-completion, completion-before-next-write, and deadline replacement explicitly in tests/specs.
- **Protocol contexts may evolve while hosts remain selected** → Keep one closed, bounded Rust context catalogue and require later changes to update candidate and both native owners until cutover.
- **Physical runs do not execute the candidate** → Treat them only as regression/isolation evidence; production-execution evidence belongs to later cutovers.

## Migration Plan

1. Add and verify the isolated safe scheduler candidate and ownership audits.
2. Extract additional deterministic connection/application coordination above it, composing existing Rust protocol and configuration services without new feature bridges.
3. Introduce one unified Rust runtime boundary per host, select scheduler/application behavior together, and delete each host's superseded native scheduler and decision logic after complete gates pass.
4. Move loopback service and SQLite behind that runtime in separately bounded changes.
5. Once hosts call the unified runtime and retain only OS adapters, delete all `*-apple-ffi`, `*-android-jni`, canonical bridge records, and bridge build catalogue entries.

Rollback for this extraction is deletion of the unselected crate and its audits; production binaries and stored data are unchanged.

## Open Questions

None. Host selection and the final unified runtime ABI remain intentionally deferred.
