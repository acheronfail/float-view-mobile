# Tasks

## 1. Planning and private platform integration

- [x] 1.1 Confirm the proposal, design and three delta specs are committed before implementation; verify strict OpenSpec validation and a planning-only commit.
- [x] 1.2 Add Linux to private server/platform and permission observations with port `11013`, preserving existing host values; add the public `linux` bootstrap type and desktop Save labeling with frontend tests; test occupied-port behavior, Linux state mapping and existing Apple/Android mappings in affected shared crates.
- [x] 1.3 Project Linux Bluetooth unavailable/off/denied/service-loss states and unavailable optional location through the existing public contract; add shared permission/projection tests proving no unsupported actions, no location gate on Bluetooth, and unchanged mobile behavior.

## 2. Linux desktop composition

- [x] 2.1 Create the single `backends/linux` package/workspace and lockfile with compatible GTK4, WebKitGTK and BlueR dependencies; document exact prerequisites and verify a locked build with the pinned Rust toolchain on the selected x86_64 Linux baseline.
- [x] 2.2 Compose one shared process/server using the private XDG data root and existing exclusive lock; test a second owner, inaccessible/current-corrupt storage, fixed-port collision and bounded startup failure without data deletion.
- [ ] 2.3 Implement ordered OS ingress, bounded effect execution, clocks/timers and cancellation outside service/UI locks; add focused tests for saturation, stale lifetimes, duplicate completions and shutdown contention, documenting Linux-specific bounds.
- [x] 2.4 Embed the shared frontend artifact and show it in the native WebView with constrained navigation and visible startup errors; verify missing-artifact failure, current token/Origin checks, remote navigation rejection and WebView recreation preserving the same service.
- [ ] 2.5 Apply shared theme effects and matching native canvas colors; document desktop close/minimize behavior and verify system/light/dark startup plus a saved preference opposite to system appearance.

## 3. Production Bluetooth adapter

- [ ] 3.1 Implement deterministic adapter selection, capability observation, discovery and saved-association lookup with scoped cleanup; test no adapter, multiple adapters, daemon restart, denied access, adapter removal and rediscovery without name-based identity guessing.
- [ ] 3.2 Implement connection/GATT discovery and notification setup as shared effect execution; test missing characteristics, unsupported capabilities, setup timeout, notification-before-ready ordering and stale generation events.
- [ ] 3.3 Implement response/no-response write execution using actual BlueR progress semantics and conservative payload bounds; test multi-chunk ordering, delayed/failed submission, ten-second stall handling, cancellation and no uncertain-write replay, documenting any unavailable mode.
- [ ] 3.4 Route notifications, disconnects and restored availability into shared policy; test guest/saved telemetry, restart association recovery, reconnect and radio loss with the controlled OS adapter seam, keeping protocol simulation in conformance.

## 4. Desktop OS effects and lifecycle

- [ ] 4.1 Implement file destination presentation/copy and existing import chooser integration, retaining service leases until completion; test cancellation, copy failure, duplicate completion and quit during presentation, and verify a canonical archive round trip.
- [ ] 4.2 Implement native permission observations and unsupported location effects without prompts or fabricated samples; verify existing frontend Settings behavior and imported location review, documenting OS prerequisites and location limitations.
- [ ] 4.3 Wire window visibility, WebView recovery, close/quit and system suspend/resume into ordered OS facts and cleanup; test expired timers and late callbacks across interruptions, and verify recording continuity on minimize plus durable recovery after forced exit.

## 5. Tooling, ownership and canonical integration

- [x] 5.1 Add `just build-linux`, `just run linux` and `just test linux`, reusing frontend build inputs without mobile toolchain dependencies; document commands in root/Linux READMEs and execute them on the supported Linux host.
- [ ] 5.2 Extend the existing dev launcher and frontend proxy configuration for `just dev linux` on port `11013`; add target-routing/readiness/cleanup tests and verify HMR, browser reload, exact upstream Origin, native file presentation and occupied-port failures.
- [x] 5.3 Add a minimal Linux software conformance target with isolated storage and explicit controlled OS inputs; reuse canonical assertions to verify authentication, product operations, recording/history, archive leases, interrupted recovery and monotonic snapshots through the Linux composition.
- [x] 5.4 Add Linux physical target launch/readiness/cleanup to the existing fixture runner; verify command routing and bounded failure cleanup without duplicating fixture scenarios or changing firmware ownership.
- [x] 5.5 Update ownership audits, AGENTS, backend documentation and OpenSpec context for Linux, including explicit source accounting and budget while retaining the core member/two-binding constraints; run audits and inspect dependencies for duplicate portable policy or unused integration probes.

## 6. Integration acceptance

- [x] 6.1 Run Linux locked formatting, warnings-denied lint, unit tests and native build plus affected shared crate/consumer tests, direct canonical scenarios and Linux software conformance; record actual commands/results and run affected frontend/tooling checks where changed.
- [x] 6.2 Verify Apple/Android consumers of changed private shared enums and permissions retain their existing behavior using focused binding/consumer tests; run additional native builds only where changed boundaries warrant them, recording unavailable relevant coverage.
- [ ] 6.3 Run the scoped Linux fixture checkpoint through real BlueZ for guest/saved discovery, notifications, available write modes, segmentation, telemetry/recording, reconnect, adapter/radio loss and suspend/resume; retain fixture safety/restore checks and record machine, BlueZ, adapter and fixture versions plus unavailable required scenarios.
- [ ] 6.4 Record real desktop observations for native WebView recreation, minimize/close/quit, appearance and file presentation; distinguish these results from software and physical BLE evidence and leave missing required coverage incomplete.
- [ ] 6.5 Reconcile task evidence, run strict OpenSpec validation and documentation/diff checks, then sync delta specs and archive together only after required acceptance is complete; verify the final archive commit preserves the planning and coherent implementation history.

Evidence and remaining gaps: [verification.md](verification.md).
