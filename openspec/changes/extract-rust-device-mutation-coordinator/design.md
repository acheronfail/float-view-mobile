## Context

The selected Swift and Kotlin application services each own a near-equivalent connection-scoped `DeviceMutationCoordinator`. It is the sole safety boundary immediately above exact Refloat/control payload codecs and below scheduler submission. It combines current generation/readiness, firmware and package compatibility, authoritative telemetry and lighting observations, semantic-control confirmation, controller shutdown, and a shared configuration-write exclusion. The safe Rust workspace already owns every protocol/policy model needed by this state machine, but the cross-feature authorization and pending-work decisions remain duplicated native application behavior.

The existing Rust `refloat-configuration-service` cannot be safely composed into the connection coordinator until the shared mutation gate is also available in Rust: configuration writes must compete with live controls and shutdown through one owner. This change therefore extracts that prerequisite as an unselected, bridge-free safe candidate while both native coordinators remain authoritative.

## Goals / Non-Goals

**Goals:**

- Own the complete current native mutation state machine in safe platform-neutral Rust, including generation isolation, readiness, authoritative observations, availability, one pending operation, idempotence, confirmation, failure, cancellation, and reset.
- Directly compose existing safe VESC/Refloat compatibility, control-codec, normalized-telemetry, firmware-identity, and request-fact models without bridge records or host callbacks.
- Preserve the exact safety matrix for package, flywheel, lighting, fatal clear, configuration exclusivity, Tune apply, and controller shutdown.
- Emit exact bounded scheduler request facts for live controls and shutdown, and an exact generation/sequence-qualified lease for configuration work, without executing either.
- Prove source parity, adversarial totality, production isolation, and unchanged all-host behavior.

**Non-Goals:**

- Selecting the candidate from Swift/Kotlin, deleting native mutation coordinators, or adding a mutation FFI/JNI bridge.
- Integrating configuration or mutation behavior into `connection-application-coordinator` in this slice.
- Owning the request scheduler, BLE/framing, clocks/timers, persistence, SQLite, sessions, local serving, frontend contracts, or OS capabilities.
- Changing wire commands, user-visible behavior, stored data, or the final unified-runtime ABI.

## Decisions

### Directly depend on the six safe models that define mutation facts

`device-mutation-coordinator` will use `refloat-control` for exact payloads and lighting decoding, `refloat-compatibility` for package control capabilities, `refloat-telemetry-service` for normalized authoritative samples, `vesc-compatibility` plus `vesc-firmware-identity` for exact read/write and shutdown admission, and `vesc-request-scheduler` for route/generation/request facts. These are the complete production dependencies and add no third-party crate.

Alternative: copy small enums and payload construction into the new crate. Rejected because it would create a second Rust source of truth. Alternative: exchange canonical FFI/JNI records. Rejected because the final in-process Rust runtime must call safe models directly and delete those bridge records.

### Configure one generation from validated identities and retain authoritative state

One validated configure command supplies the current scheduler generation, selected route, structurally decoded VESC firmware identity, exact admitted Refloat compatibility profile, and connection readiness. The coordinator derives writable and shutdown capability through Rust compatibility policy, initializes bounded loading control state, and clears prior pending work atomically. Current normalized telemetry and exact lighting responses update the retained authoritative package, mode, footpad, fatal, and lighting state before later authorization decisions.

Alternative: pass writable/shutdown booleans and current state with every authorization call, matching the native call shape. Rejected because a future unified coordinator already owns those facts; retaining them once prevents stale caller snapshots from becoming an authorization seam.

### Use one closed pending mutation and exact token

The coordinator permits at most one of live control, fatal clear, controller shutdown, or configuration lease. Every accepted non-idempotent operation receives a generation- and sequence-qualified `MutationToken`. Failure, configuration finish, shutdown clearing, and cancellation must match the exact current token; stale, repeated, mismatched, or prior-generation completions are inert. Sequence rollover is explicit wrapping while the generation remains part of identity.

Configuration and Tune apply share the same `Configuration` lease kind because the configuration service owns their wire/readback lifecycle. The lease must be acquired immediately before its first scheduler submission and held until verified completion or failure. No second lock or direct write path is introduced.

### Return exact request facts without running the scheduler

Package, flywheel, lighting, fatal-clear, and shutdown authorization returns either an idempotent snapshot or a mutation token plus one exact `RequestFacts`. The request uses the selected checked route, existing priority/context, expected command/refloat command, timeout, generation, and one-way semantics. Configuration authorization returns only a token; its later composition supplies the configuration service request facts while the mutation lease guarantees exclusivity.

The candidate owns no scheduler instance, physical time, deadline, write callback, transport, frame, or host reference. Alternative: emit only payload bytes as the native coordinators do. Rejected because priority/context/one-way attribution is part of the duplicated application decision and the safe scheduler facts already provide the final internal boundary.

### Preserve telemetry-first confirmation and flywheel exception

An authoritative sample is applied to pending confirmation before flywheel exclusion is evaluated. Flywheel enable confirms on the first flywheel-mode sample. Flywheel disable remains pending through repeated flywheel samples and confirms only after normal mode. Every other pending device mutation becomes failed when newly authoritative flywheel mode prohibits it. The only mutation authorized while flywheel mode is active is exact flywheel disable; it remains eligible even when normalized ride state is riding.

Lighting confirms only from an exact profile-matched decoded lighting response. Package/flywheel/fatal work confirms only from authoritative telemetry. Idempotent requests emit no request and create no pending token.

### Keep production isolation until direct coordinator composition

The crate adds no C/JNI symbol, bridge catalogue entry, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, callback, or host fallback. Repository audits require both native coordinators and application services to remain selected. A later change will directly compose this candidate and `refloat-configuration-service` inside `connection-application-coordinator`; no mutation-specific bridge will ever be created.

## Risks / Trade-offs

- **[Safety behavior could diverge while three copies coexist]** → Derive shared vectors from both native suites, exhaustively cover every state combination and ordering seam, and keep the candidate unselected until direct Rust composition deletes the native paths.
- **[Retaining authoritative state could admit stale observations]** → Require current generation, checked monotonic sample sequence, and exact lighting profile; reject non-increasing, stale, mismatched, malformed, or repeated observations atomically.
- **[A configuration lease could outlive failed scheduler work]** → Make finish/failure token-qualified, cover every scheduler completion class in later composition, and reset all pending work on connection replacement.
- **[Request-fact construction might accidentally execute or own timing]** → Return only existing safe scheduler value types and audit out scheduler instances, clocks, timers, callbacks, async runtimes, and transport dependencies.
- **[Physical runs do not execute the candidate yet]** → Treat them as strict regression/isolation evidence; later direct composition and host cutovers must separately prove selected production execution.

## Migration Plan

1. Add and exhaustively verify the unselected safe mutation coordinator while native hosts remain unchanged.
2. In the next change, directly compose this owner and the existing Refloat configuration service into the Rust connection application coordinator, including exact scheduler attribution and cache effects.
3. Extend that coordinator with remaining controller/current-limit application behavior, then place it behind the platform-neutral runtime shell with Rust persistence and local serving in separate changes.
4. Cut over Apple and Android only after complete runtime software and physical gates; delete native application orchestration and every superseded `*-apple-ffi`/`*-android-jni` crate rather than retaining compatibility layers.

Rollback is deletion of the unselected crate, workspace/audit entries, and this change's canonical requirements; selected host behavior is unaffected.

## Open Questions

None. Direct connection-coordinator composition, persistence/runtime execution, host cutover, and native/bridge deletion remain explicit later OpenSpec changes.
