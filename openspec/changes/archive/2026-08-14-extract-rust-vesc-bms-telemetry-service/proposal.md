## Why

Apple and Android still duplicate the BMS polling/application lifecycle even though auxiliary discovery, command-96 decoding, and connection-wide request scheduling now have safe Rust counterparts. Extracting the composition layer next turns those isolated candidates into one directly usable Rust service for the future unified runtime without adding another temporary host bridge or changing production behavior.

## What Changes

- Add one safe bounded `backends/core/vesc-bms-telemetry-service` candidate that directly composes Rust auxiliary discovery, BMS values decoding, and request-scheduler facts.
- Move exact direct/CAN discovery request construction, selected-route command-96 polling, one-second completion-driven cadence, current-generation attribution, concrete owned BMS observations, three-consecutive-failure unavailability, stale-observation retention, valid-response recovery, reset, and ordered timer/publication/diagnostic effects into a deterministic host-independent service.
- Prove source-derived parity and adversarial behavior for controller-attached and standalone BMS lifecycles, direct/preferred/reused/enumerated routes, malformed responses, scheduler outcomes, stale work, timer/generation rollover, request bytes, privacy, and read-only command safety.
- Keep Swift and Kotlin selected and unchanged. Add no Apple FFI, Android JNI, callback, executor, BLE owner, persistence port, local server, generated binding, or packaged artifact; the unified Rust runtime will call this safe API directly and later delete both native lifecycles and temporary protocol bridges.
- Extend workspace tests and isolation audits, then run `just verify` and the connected Android/iOS/macOS ESP32 matrix, deferring only bounded location acquisition to the agreed manual follow-up.

Non-goals: production host cutover; deleting native BMS discovery/polling or existing command-96 bridges; controller telemetry changes; session or SQLite ownership; BLE, background, location, lifecycle, files, theme, loopback, WebView, or frontend-contract changes; BMS mutation commands; or the final unified runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the safe compositional VESC BMS telemetry-service candidate, exact lifecycle parity, and strict boundary below runtime execution.
- `platform-hosts`: Record candidate versus selected native ownership and the direct-composition/no-new-bridge path.
- `backend-conformance`: Require source-derived, adversarial, isolation, software, and all-host physical evidence before later runtime selection.

## Impact

The locked Rust workspace gains one unselected crate depending only on `auxiliary-node-discovery`, `vesc-bms-values`, and `vesc-request-scheduler`. Root verification gains focused tests and ownership/dependency audits. Native packages, source selection, runtime behavior, backend contracts, SQLite, and OS capability ownership remain unchanged, so no existing bridge can yet be removed.
