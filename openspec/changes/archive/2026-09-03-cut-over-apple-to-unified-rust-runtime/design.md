## Context

Apple currently selects ten safe Rust protocol components through ten stateless C ABIs, but Swift still owns connection discovery, request scheduling, telemetry cadence/normalization, BMS discovery, mutation/configuration coordination, active-session persistence, history reduction, and pruning. The proven `application-runtime` composes these Rust owners and provides direct session reader/pruner entry points. The host cutover must cross one stateful boundary, preserve the hand-written frontend contract, retain OS responsibilities in Swift, and delete the superseded paths in the same change.

The pre-cutover runtime baseline is 34,070 Rust and 17,924 Apple production-source lines, 44 workspace members, 20 temporary protocol bridge crates, ten Apple bridge families, and 80 Android target artifacts. The Apple change is acceptable only if the combined Rust-plus-Apple production count falls below 51,994 and the Apple protocol bridge/artifact counts fall from ten to zero/one respectively.

## Goals / Non-Goals

**Goals:**

- Select one process-owned Rust application runtime for both iOS and macOS.
- Preserve ordered host execution, generation safety, persistence-before-publication, reconnect/reset behavior, frontend envelopes, canonical SQLite data, and disposable-WebView semantics.
- Replace ten stateless Apple protocol boundaries with one bounded stateful application boundary.
- Delete superseded Swift decisions and Apple migration scaffolding and demonstrate a net source/build reduction.
- Pass software gates plus connected iOS/ESP32 physical conformance before ownership is claimed.

**Non-Goals:**

- Moving CoreBluetooth, clocks/timers, CoreLocation acquisition, permissions, lifecycle/background execution, native file presentation, theme/UI, loopback transport/authentication, product catalogue metadata, or export/import presentation into Rust.
- Changing the frontend contract, SQL schema, persistence compatibility policy, Android production selection, or running Android physical conformance.
- Adding another protocol bridge, generated binding layer, callback from Rust, or production fallback.

## Decisions

### One opaque handle and one versioned envelope ABI

`application-runtime-apple-ffi` will be the only Apple Rust boundary. It owns an `ApplicationRuntime` behind a checked opaque handle and exposes construction/destruction, bounded command submission, and bounded result-copy operations. Commands and returned effects use a documented versioned UTF-8 JSON envelope with strict byte/depth/collection bounds, fixed numeric discriminants, and complete validation before mutation. Rust never calls Swift and never retains caller buffers.

This adds `serde`/`serde_json` only to the unsafe host-boundary crate. The dependency replaces ten handwritten C record ABIs, ten headers/modules, and their Swift validation code. Hand-writing a large evolving C union would retain most of that maintenance burden; generated bindings conflict with repository policy; callbacks would violate ownership and make ordering harder to audit.

### Tokens remain Rust-owned

Deadline, timer, mutation, prune-plan, reader, and interruption identities remain inside the coarse boundary. The bridge maps them to bounded generation-qualified scalar handles and rejects stale or foreign handles. Swift schedules/cancels OS work using those scalar handles and reports firings back as commands; it cannot reconstruct private Rust tokens.

### Swift remains an OS-effect executor and contract adapter

A small `RustApplicationRuntime` Swift adapter validates ABI/version/result extents and maps envelope records into existing public domain snapshots. `ApplicationService` retains discovery scanning, BLE connection/reconnect execution, timer objects, saved-device/profile/catalogue operations, location acquisition, exports/imports, and event publication. Once CoreBluetooth reports a prepared transport, all VESC notification bytes and application operations pass through Rust; Swift does not run alternate protocol or application decisions.

### Active session and history access use the Rust SQLite owner

The native product catalogue allocates session identity/path and reconciles returned commit markers, while Rust exclusively creates/writes the active canonical session database and performs bounded published reads and prune preview/commit. Native export/import may perform packaging and catalogue work but obtains session data through the coarse boundary. The duplicated Swift write/reduction/prune SQL and `SessionHistoryReducer` are deleted. Existing disposable incompatible-database behavior remains unchanged.

### Deletion is part of selection

The implementation removes these owners rather than leaving a fallback:

- Swift application algorithms: `AdaptiveLogging`, `ControllerDiscovery`, `AuxiliaryNodeDiscovery`, `VescRequestScheduler`, `TelemetryPolling`, `RefloatMutationCoordinator`, `RefloatConfigurationService`, and `SessionHistoryReducer`.
- Selected Swift protocol implementation/adapters under `protocol/vesc` and `protocol/refloat`, retaining only fixture/software-peripheral facts that do not make production decisions.
- The ten `*-apple-ffi` crates, `apple-ffi-support`, ten `CVesc*` modules, per-family Package/Xcode link/build entries, and Apple ABI tests/audits.
- Active-writer, history-reduction, and prune-decision portions of `SQLiteLogStore`; retained catalogue/export/import code must not decode VESC or reproduce Rust history algorithms.

Tests that address deleted private types are deleted or replaced with coarse runtime bridge/service vectors. Canonical black-box conformance remains the behavioral authority.

### Apple verification includes real BLE

Focused Rust boundary tests cover handle, envelope, bounds, panic containment, token mapping, and exact effect order. Shared Apple tests cover effect execution, state projection, persistence/catalogue reconciliation, reconnect/reset, and absence of fallbacks. Packaging proves one archive on macOS, simulator, and device targets. After `just verify`, physical conformance runs on the connected iOS device against the ESP32 fixture and its result is recorded before deletion ownership is finalized.

## Risks / Trade-offs

- **[Risk]** A JSON host boundary can allocate or accept ambiguous values. → **Mitigation:** hard byte/depth/count bounds, closed tagged records, finite-number validation, two-pass caller-owned output, no permissive defaults, fuzz/adversarial vectors, and no JSON in the safe runtime crate.
- **[Risk]** One opaque handle can be misused concurrently or after free. → **Mitigation:** one serial Swift owner under the service lock, registry-backed handle IDs rather than dereferenced caller pointers, generation-qualified tokens, idempotent destroy, and hostile-handle tests.
- **[Risk]** Moving active writes while retaining catalogue/export responsibilities can split transaction visibility. → **Mitigation:** Rust commit markers are published before Swift events; catalogue reconciliation uses the marker and the same canonical file; reads require a published boundary.
- **[Risk]** A large deletion makes rollback tempting. → **Mitigation:** the planning commit and implementation commit remain separately revertible in git, but no runtime fallback ships; failure before physical verification blocks the change rather than retaining two owners.
- **[Risk]** macOS and iOS host behavior can diverge. → **Mitigation:** both use the same Swift adapter and static archive, shared unit tests, both software conformance targets, and an additional real-iOS BLE checkpoint.

## Migration Plan

1. Add and exhaustively test the single Apple runtime boundary without selecting it.
2. Add the Swift adapter/effect executor and focused parity vectors.
3. Select it in `ApplicationService` and native packaging, then remove the ten old Apple bridge paths.
4. Move active session/history/prune calls and delete duplicated Swift algorithms/tests.
5. Run focused audits, report convergence deltas, and run `just verify`.
6. Run connected iOS physical conformance with the ESP32 fixture. Only after success commit the deletion-bearing cutover, sync specs, and archive.

Rollback before completion is `git revert` of the implementation commit. There is no in-product fallback or feature flag.

## Open Questions

None. Android remains deliberately unchanged until the next independent cutover.
