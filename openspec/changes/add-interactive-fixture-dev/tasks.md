## 1. Shared Refloat Simulation

- [x] 1.1 Add immutable source-provenanced controller firmware fixtures for VESC 6.02, 6.05, and 6.06, including their version-specific `COMM_FW_VERSION` tails, and verify both native decoders read the expected stable THOR300 identity.
- [x] 1.2 Add immutable source-provenanced Refloat fixtures for exact identities 1.0.0 legacy INFO/realtime/all-data, 1.1.0 INFO v2/dynamic IDs/data, and 1.2.3 INFO v2/dynamic IDs/data with alert metadata.
- [x] 1.3 Implement independently typed THOR300 identity, three firmware variants, three Refloat codecs with exact read-only request sets, and direct/Express topology strategies; compose them at runtime without an enumerated nine- or eighteen-profile registry.
- [x] 1.4 Refactor the conformance VESC simulator to accept the independently composed controller identity, firmware variant, Refloat codec, direct or Express topology, optional CAN identity, and telemetry source while preserving Express/CAN-ID-1 as the deterministic default and proving canonical behavior is unchanged.
- [x] 1.5 Implement and unit-test bounded Refloat 1.0.0 legacy and Refloat 1.1.0/1.2.3 dynamic telemetry encoders from one normalized idle/riding sample, including version-specific masks and optional tails.
- [x] 1.6 Implement and unit-test the seeded interactive ride model, including correlated speed/ERPM/load, voltage sag, temperature drift, smooth adjacent samples, immediate idle output after stop, and equivalent decoded core values across package formats.
- [x] 1.7 Generate table-driven Cartesian tests for direct identity/package requests and Express 6.05 identity, single-ID CAN discovery, command-34 forwarding, wrong-ID silence, and read-only safety across all eighteen effective configurations, while retaining golden fixtures only for distinct upstream wire formats.

## 2. Server-Side Fixture Coordinator

- [x] 2.1 Implement the revisioned bounded coordinator state model under the documented `/conformance/fixture-dev` ownership boundary, including selected topology, retained canonical decimal CAN ID, selected composable variants, and a serialized event/action queue around one injectable `FixtureControl`.
- [x] 2.2 Implement explicit serial enumeration, attach/detach, bounded `RESET`/`READY`, optional initial-port attachment, and active-session serial-loss recovery that closes ownership, powers state off, preserves configuration, and requires explicit reattachment, with fake-control tests.
- [x] 2.3 Implement idempotent power and powered-on-only ride actions, prevent repeated start from reseeding an active ride, continuously answer allowlisted requests, re-advertise while powered on, and retain bounded counters/diagnostics, with state-machine tests.
- [x] 2.4 Implement one powered-off-only atomic configuration action for topology, retained CAN ID, firmware, and Refloat selections that resets simulated protocol/telemetry state and rejects active changes, with state-machine tests.
- [x] 2.5 Add one process-wide coordinator resolver and bounded signal cleanup, with tests proving UI reload/HMR and repeated route resolution cannot duplicate serial ownership; document and test that coordinator/server source changes require process restart.
- [x] 2.6 Preserve canonical conformance fail-fast request handling while interactive mode silently drops unsafe, unknown, and wrong-CAN-target requests with bounded diagnostics; detach only for malformed framing, bounded serial-protocol violations, or serial failure.

## 3. Development-Only SvelteKit Server

- [x] 3.1 Add aligned SvelteKit, Svelte, Vite, and checking dependencies to the existing conformance package and configure `/conformance/fixture-dev` for development/checking only on strict fixed address `127.0.0.1:11004`, with no second package or lockfile.
- [x] 3.2 Add thin SvelteKit routes for complete state, bounded port enumeration, attach/detach, one atomic configuration action, power, and ride operations, including exact Origin, launch-token, method, enum, canonical CAN-integer, and body validation.
- [x] 3.3 Add focused server-route tests covering atomic configuration/actions, unauthorized or oversized requests, invalid topology/profile/CAN values, active configuration rejection, monotonic complete snapshots, occupied port 11004, initial-port handling, and state after serial loss.

## 4. Fixture Control Surface

- [x] 4.1 Build an ownership-first `control` feature with a thin route and an atomic saved configuration form containing direct/Express topology, retained Express CAN ID, controller firmware, and Refloat selectors, plus serial attachment, power, BLE connection, ride controls, synthetic telemetry preview, request counters, and bounded diagnostics driven by complete snapshot polling.
- [x] 4.2 Implement disabled/error/loading states and native primary-pointer plus guarded keyboard activation without double activation.
- [x] 4.3 Add Svelte checks and focused UI tests for atomic form save, topology/profile rendering, CAN-ID validation and retention, powered-off-only configuration, powered-on-only ride controls, duplicate-start prevention, direct-mode telemetry limitation, snapshot replacement, polling cleanup, error presentation, and exactly-once pointer/keyboard activation.

## 5. Developer Workflow

- [x] 5.1 Add the conformance development script and `just fixture dev [port]` dispatch while preserving the existing firmware build, flash, monitor, check, and physical-conformance meanings.
- [x] 5.2 Add `/conformance/fixture-dev/README.md` ownership/import conventions and update repository and fixture/conformance documentation with the parallel `just fixture dev` plus `just dev <platform>` workflow, direct/Express topology and CAN-ID controls, composed variant matrix, current direct/older-Refloat compatibility behavior, upstream protocol provenance, physical prerequisites, fixed `127.0.0.1:11004` address, server-change restart rule, and development-only non-goals.
- [x] 5.3 Update repository audits as needed to enforce that VESC simulation remains outside the ESP-IDF project and no fixture UI build, adapter, deployment, or generated production artifact is introduced.

## 6. Verification

- [x] 6.1 Run focused conformance TypeScript, Svelte, fixture-decoder, topology/profile matrix, CAN-forwarding, simulator, coordinator, server-route, and control-surface checks/tests and resolve all failures.
- [x] 6.2 Run `just verify` and confirm the complete software gate passes without an ESP32 or native device and without building a fixture web artifact.
