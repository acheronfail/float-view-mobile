## Context

Apple `ControllerDiscovery.swift` and Android `ControllerDiscovery.kt` are parallel pure state machines that own the same direct/CAN identification behavior. They consume decoded firmware and Refloat facts, retain bounded connection-generation state, and return effects that each native `ApplicationService` executes through its scheduler, persistence, and publication owners. The repository now has safe Rust firmware identity, VESC compatibility, Refloat compatibility, and request-scheduler components, but discovery still duplicates compatibility coordination and route policy in Swift and Kotlin.

This change extracts the decision layer only. It does not select Rust in either host, so production packages and physical behavior continue through the existing native state machines.

## Goals / Non-Goals

**Goals:**

- Provide one unsafe-free, dependency-minimal Rust controller-discovery candidate with total typed transitions.
- Directly reuse `FirmwareIdentity`, VESC compatibility, and Refloat compatibility models instead of copying their catalogues or inventing bridge records.
- Cover direct selection; configured and learned CAN probing; hint invalidation; bounded normalized enumeration; sequential probe/timeout behavior; complete classified inventory; candidate limits and selection; active-session continuity; Refloat family/layout readiness; reset and stale-generation isolation; stable payload-free failures; and diagnostic privacy.
- Preserve exact canonical limits: CAN IDs `0...253`, at most 64 normalized IDs, at most 16 motor-controller candidates, and bounded display identities.
- Make the candidate ready for direct composition by a later unified Rust runtime without adding an Apple FFI or Android JNI package.

**Non-Goals:**

- Selecting the candidate in Apple or Android, deleting native discovery, or changing the frontend contract.
- Parsing Board Profile JSON, reading/writing routing hints, persisting a selected CAN ID, or owning guest/saved policy.
- BLE, clocks, timers, request scheduling/execution, VESC/Refloat payload decoding, CAN forwarding bytes, auxiliary BMS discovery, telemetry, sessions, SQLite, loopback serving, files, theme, lifecycle, permissions, or UI behavior.
- Adding code generation, callbacks, opaque handles, foreign records, or another bridge catalogue entry.

## Decisions

### Use a safe Rust state machine with typed facts and effects

The crate will expose platform-neutral routes, continuity/hint facts, stable issue codes, bounded candidates/inventory, snapshots, and transition effects. A host effect may request a direct or forwarded firmware probe, CAN ping, routing-hint clear, Refloat INFO, telemetry layout, readiness, candidate selection publication, or failure; it never executes that work or retains a host reference.

Alternative: move request construction and scheduler execution at the same time. Rejected because it would combine discovery policy with host integration and create a much larger regression surface.

### Compose existing safe domain models directly

Firmware inputs use `float_view_vesc_firmware_identity::FirmwareIdentity`; admission calls `float_view_vesc_compatibility`; Refloat readiness consumes `float_view_refloat_compatibility::CompatibilityProfile`. The crate introduces no duplicate compatibility tables and no canonical foreign record.

Alternative: define a generated or serialized cross-component model. Rejected because all participants are safe Rust candidates intended to run in one future process.

### Keep persistence and profile representation above discovery

The candidate accepts an already validated optional preferred CAN ID and optional private routing hint. It emits `ClearRoutingHint` and `RememberController` facts but does not parse `valuesJSON` or call storage. This preserves the dependency direction and lets a later Rust application runtime own SQLite/profile behavior without coupling discovery to the current native representation.

Alternative: parse Board Profile JSON in this crate. Rejected because JSON/storage shape is not part of route selection and would block reuse when persistence moves to Rust.

### Bound all retained state and make public transitions total

CAN normalization uses the finite `0...253` domain, rejects more than 64 unique valid IDs before probing, stores at most 64 classified nodes and 16 motor candidates, and never retains raw ping input. Invalid selections, mismatched responses, stale generations, repeated starts, and counter rollover return deterministic results rather than panicking. Generation increments use explicit wrapping solely to invalidate stale work.

Alternative: mirror native precondition failures exactly. Rejected because the future unified runtime needs a panic-free public boundary; native callers already translate invalid frontend selection into stable errors before invoking discovery.

### Treat native code and physical behavior as the parity oracle for this extraction

Rust tests replay the committed shared discovery TSV and add exhaustive/adversarial cases derived from both native implementations and canonical connection requirements. Audits prove native files remain selected, the ten bridge catalogues are unchanged, and no host/build source mentions the candidate. `just verify` plus the complete Android/iOS/macOS ESP32 matrix catches production drift.

## Risks / Trade-offs

- **The repository temporarily has three discovery implementations.** → Keep Rust unselected, require source-derived parity and production-isolation audits, and make unified runtime composition—not a per-platform discovery bridge—the follow-up path.
- **Native Swift/Kotlin models are not identical at their language edges.** → Use only structurally decoded bounded Rust identities and canonical typed facts; test every externally meaningful route/effect/issue rather than copying language-specific containers.
- **Private controller UUIDs are required for continuity.** → Retain them only in safe in-memory identities/hints, redact all `Debug` output, and expose no UUID in candidate display candidates or diagnostics.
- **Candidate selection and inventory can accidentally become unbounded.** → Enforce finite-domain normalization and hard collection limits before publication; add overflow, duplicate, timeout, reset, and malformed-order tests.
- **Physical iOS lifecycle orchestration may outlive a passing XCTest.** → Require the application/BLE checkpoints, bound runner execution, record only location acquisition for manual follow-up, and report infrastructure timeout evidence separately from product failures.

## Migration Plan

1. Add and verify the isolated Rust candidate while native discovery remains selected.
2. In later increments, compose discovery with the Rust scheduler, telemetry/configuration services, mutation policy, persistence, and loopback runtime.
3. Cut over hosts only when the unified runtime can execute the complete production path; delete both native discovery implementations and temporary protocol bridges in those cutovers.

Rollback is deletion of the unselected crate, its focused recipes, and its documentation/spec additions; no production package or persisted data changes.

## Open Questions

None for this extraction. Host selection and unified-runtime effect execution remain explicit later changes.
