## 1. Inventory and independent evidence

- [x] 1.1 Inventory native/direct simulator call sites, build selection and packaged assets; deliver a deletion list and baseline source/package counts, including helpers used outside simulation.
- [x] 1.2 Reconcile Swift/Kotlin/TypeScript discovery, firmware, CAN/BMS, configuration and telemetry transcripts against immutable vectors and vendored source; record intentional differences and passing golden transcript tests.

## 2. Canonical simulator and bounded relay

- [x] 2.1 Consolidate software protocol behavior in conformance-owned models; verify fragmented, malformed, multiple-frame, configuration write/readback and reset cases without production codec imports.
- [x] 2.2 Implement the loopback HTTP relay with independent run authentication and isolated sessions; verify wrong credentials, non-loopback selection, foreign session and duplicate-operation rejection before mutation.
- [x] 2.3 Enforce message/payload, command/event/byte/session bounds, deadlines and reserved cleanup; verify exact-limit/+1, stalled polls, saturation and shutdown within five seconds.
- [x] 2.4 Add generation-qualified event ordering and explicit model reset; verify reconnect, late notifications, run isolation, frontend recreation and harness-loss behavior.
- [x] 2.5 Route direct Rust software conformance through the same session dispatcher; verify canonical direct conformance and retained independent protocol-vector tests pass.

## 3. Native software host selection and deletion

- [x] 3.1 Introduce an explicit Apple conformance build/launch seam and thin asynchronous relay adapter; verify macOS and iOS simulator canonical conformance and relay failure cleanup.
- [x] 3.2 Delete Apple's native peripheral, unused simulation-only protocol helpers and fixture bundle wiring; verify reference audits, focused Swift tests and ordinary/Release/device packaging.
- [x] 3.3 Introduce an explicit Android conformance source set/build/launch seam and loopback forwarding; verify emulator canonical conformance, Activity recreation and cancellation without emulator-only automatic selection.
- [x] 3.4 Delete Android's native peripheral, unused constants/framing helpers and fixture assets; verify linked tests and clean host/four-ABI Debug/Release packaging.
- [x] 3.5 Audit both native adapters for absence of protocol interpretation/product assertions and commit verified host slices with task updates; verify no new dependency, Rust member or binding family and a net reduction in duplicated simulator code.

## 4. Selection isolation and completion

- [x] 4.1 Test attempted simulation activation in ordinary, Release and physical builds; inspect packaged symbols/assets and verify production network policy and frontend routes remain unchanged.
- [x] 4.2 Run explicit macOS/iOS/Android ESP32 smoke checkpoints for discovery, saved telemetry and reconnect with no simulator process; record proof that real OS adapters are selected, leaving unavailable coverage incomplete unless explicitly deferred.
- [x] 4.3 Run focused simulator/relay/native regression tests, formatting, strict OpenSpec validation and `git diff --check`; verify every new scenario has passing evidence and document the Linux test-adapter contract.
- [ ] 4.4 Run `just verify` across all software targets, record results and commit implementation/task updates; once required checks pass, sync delta specs and archive together in the final commit.
