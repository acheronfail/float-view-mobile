## 1. Shared host boundary

- [x] 1.1 Extract platform-neutral envelope, registry, token, interruption, pending-result, status, and bound behavior into one safe `application-runtime-host` crate without copying runtime algorithms.
- [x] 1.2 Reduce the Apple C ABI to thin raw-pointer validation over the shared host API while preserving its exact header, symbols, statuses, limits, and adversarial tests.
- [x] 1.3 Add focused shared-host tests for every command/effect family, malformed/bounded input, stale/foreign handles/tokens, pending results, concurrency, interruption, panic containment, and redacted failures.

## 2. Android coarse JNI

- [x] 2.1 Add one thin callback-free `application-runtime-android-jni` crate and Kotlin adapter with versioned commands/results, checked caller-owned primitive arrays, nonzero handles, fixed statuses, and no retained JVM references.
- [x] 2.2 Add exact/adversarial host-JNI and JVM tests for lifecycle, result sizing/copy, malformed/oversized input, handles, aliasing, concurrency, interruption, status mapping, and every command/effect family.
- [x] 2.3 Source-build one host-test and four ABI libraries from one Gradle definition and package exactly one runtime library in clean Debug and Release outputs.

## 3. Android selection and persistence

- [x] 3.1 Refactor Android `ApplicationService` to own one runtime, submit connection/notification/timer/activation/control/configuration/session/location/history/prune facts, execute ordered effects, and project authoritative snapshots.
- [x] 3.2 Route controls, shutdown, Refloat configuration/Tunes, BMS, saved/guest sessions, commit-marker reconciliation, reads/reductions, interruption, pruning, reconnect/reset, and errors through Rust while retaining Android OS/catalogue/presentation responsibilities.
- [x] 3.3 Replace tests of private Kotlin algorithms with focused runtime-adapter/effect/persistence vectors while preserving canonical contract/conformance behavior.

## 4. Deletion and build simplification

- [x] 4.1 Delete the superseded Kotlin application and VESC/Refloat protocol decision owners and production/test references with no fallback.
- [x] 4.2 Delete all ten Kotlin `Rust*Bridge` adapters, ten `*-android-jni` crates, `android-jni-support`, their tests, tasks, properties, source sets, and generated protocol artifact paths.
- [x] 4.3 Delete the bridge catalog and per-protocol Android helper/Just recipes; retain only one coarse runtime build path for each host.
- [x] 4.4 Update ownership docs and audits so retired paths/imports/symbols/artifacts fail and the frontend contract, SQL schema, Apple production selection, and Android OS-owned adapters remain unchanged.

## 5. Final convergence and software gates

- [x] 5.1 Record final Rust/Apple/Android production source, workspace-member, binding, bridge, and target-artifact counts; require combined source below 59,280 lines, members below 34, and temporary protocol bridges/artifacts equal zero.
- [x] 5.2 Run formatting, focused locked tests, warnings-denied workspace linting, JVM/Swift tests, clean Apple and Android host/device/simulator/ABI packaging, audits, and `git diff --check`.
- [x] 5.3 Run strict OpenSpec validation and complete `just verify`, including macOS/iOS/Android software conformance.

## 6. Requested physical completion

- [x] 6.1 Discover the connected iOS device and ESP32 serial port and rerun the canonical iOS physical matrix after shared-boundary changes, allowing only an explicit bounded location deferral.
- [x] 6.2 Record that Android physical testing was skipped, confirm no production fallback or unrelated contract/schema change, and commit the deletion-bearing final cutover.
- [ ] 6.3 Sync delta specs, archive the completed change, commit the archive, and leave only pre-existing user changes in the worktree.
