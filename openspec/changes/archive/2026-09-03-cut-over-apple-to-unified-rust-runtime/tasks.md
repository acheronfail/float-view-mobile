## 1. Coarse Apple boundary

- [x] 1.1 Add one `application-runtime-apple-ffi` workspace crate with a registry-backed runtime handle, fixed ABI statuses, bounded versioned command/effect envelopes, two-pass caller-owned results, panic containment, and no callbacks.
- [x] 1.2 Map connection, notification, deadline/timer, activation, control, configuration, session/location, reader, interruption, and prune operations to existing safe Rust owners without copying algorithms.
- [x] 1.3 Add exact and adversarial ABI tests for lifecycle, ordering, tokens, malformed/oversized envelopes, UTF-8/numeric/path bounds, stale/foreign handles, insufficient/overlapping buffers, concurrency rejection, panic containment, and redacted diagnostics.

## 2. Shared Apple selection

- [x] 2.1 Add the single C header/module and Swift `RustApplicationRuntime` adapter with strict ABI/result validation and complete mapping to retained public domain records.
- [x] 2.2 Refactor `ApplicationService` to own one runtime, translate CoreBluetooth/clock/timer/cache/location facts into commands, execute ordered effects, and project authoritative snapshots without native protocol/application fallbacks.
- [x] 2.3 Route controls, shutdown, Refloat configuration read/write/Tune apply, BMS state, saved/guest session behavior, reset/reconnect, and errors through Rust while retaining product catalogue and OS responsibilities.
- [x] 2.4 Route active session writes, published history reads/reductions, interruptions, locations, and prune preview/commit through the Rust session owner and reconcile commit markers with the native catalogue/export/import shell.
- [x] 2.5 Replace deleted private Swift tests with focused coarse-adapter/service/persistence parity and adversarial vectors, keeping canonical contract/conformance behavior unchanged.

## 3. Deletion and packaging

- [x] 3.1 Delete the eight superseded Swift application algorithm files and remove their production/test references.
- [x] 3.2 Delete the selected Swift VESC/Refloat protocol decision/adapters, preserving only software-fixture facts required outside production.
- [x] 3.3 Delete all ten `*-apple-ffi` crates, `apple-ffi-support`, ten `CVesc*` modules, ABI tests, Package linker/system-library entries, Xcode build/link phases, and per-family Apple recipes/audits.
- [x] 3.4 Package and link exactly one coarse Apple runtime archive for macOS, iOS simulator, and iOS device while leaving Android packages and all OS-owned Swift adapters unchanged.
- [x] 3.5 Update root/core audits and ownership documentation so forbidden old files/imports/symbols/artifacts fail and the candidate/bridge/workspace inventories match Apple-selected state.

## 4. Convergence and verification

- [x] 4.1 Record before/after tracked non-test Rust and Apple source, workspace-member, Apple bridge/module/artifact, and Android bridge/artifact counts; require combined Rust-plus-Apple source below 51,994 lines.
- [x] 4.2 Run Rust formatting, locked focused boundary tests, warnings-denied workspace linting, Swift formatting/tests, Apple ABI/packaging audits, and `git diff --check`.
- [x] 4.3 Run strict OpenSpec validation and the complete `just verify` software gate, including macOS/iOS/Android software conformance.
- [x] 4.4 Discover the connected iOS device and ESP32 serial port, then run the canonical iOS physical conformance matrix against the fixture and record the successful checkpoint; do not run Android physical conformance.
- [x] 4.5 Confirm the frontend contract and SQL schema are unchanged, Android production source is unchanged, no fallback remains, and commit the deletion-bearing Apple cutover.
