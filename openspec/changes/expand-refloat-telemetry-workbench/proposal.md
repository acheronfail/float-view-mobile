## Why

Float View and its interactive fixture currently reduce Refloat telemetry to a small motor-data subset, so developers cannot reproduce reported board states or build and verify detailed live and historical telemetry experiences. The supported Refloat packages already expose richer motion, rider-input, control, pushback, charging, alert, field-weakening, and accumulated-energy data that should flow source-faithfully through the fixture, both native backends, durable logs, and the shared frontend.

## What Changes

- Turn `just fixture dev` into an interactive scenario workbench, paired with `just dev <platform>`, for manually reproducing editable direct/CAN topologies, user-reported configurations, telemetry states, and connection events without becoming an automated test runner.
- Add bounded import and export of human-readable scenario documents using deterministic private controller identities, plus common starting templates and expert overrides.
- Add one version-independent semantic board-state model with coherent scenes and individual controls for motion, motor and battery load, IMU, footpads, ATR and tilt contributions, field weakening, charging, pushback, wheelslip, modes, stop conditions, warnings, and faults.
- Expand the source-faithful simulator and both native protocol implementations across the audited stable Refloat 1.0.x, 1.1.x, and 1.2.x wire profiles: 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3. Support complete legacy realtime and compact telemetry, complete negotiated dynamic telemetry, latest-reported accumulated values, charging data, and active firmware-fault state.
- Separate package-version recognition from wire compatibility: exact audited releases select a profile, dynamic command 32 must advertise the complete required semantic baseline before the controller becomes ready, and additional bounded numeric fields remain forward-compatible extensions.
- Expand the composed controller catalog from VESC 6.02/6.05/6.06 to audited stable 6.02/6.05/6.06/7.00. Model their shared read-only command family once, require the complete stable `COMM_FW_VERSION` identity baseline and zero test-version marker, correct the source-applicable 6.05 hardware-CRC fixture, and reject malformed, test, prerelease, or unknown controller identities.
- **BREAKING** Extend authenticated live samples and session detail with typed detailed telemetry, bounded discrete transition events, stable state fields, and a bounded extension field map; update the frontend client, mock, Apple and Android loopback serialization, persistence, exports, and conformance together.
- Persist the additional normalized telemetry on both native backends and expose it in complete `state.changed` snapshots, session series, diagnostics, and export without exposing private controller identifiers.
- Keep telemetry, durable logging, and session ownership strictly bound to one selected direct-or-CAN controller at a time. Discovery may expose multiple candidates, but changing the selected controller ends the current session before another begins.
- Add shared live and historical detailed-telemetry presentation for the new fields and simulated states.
- Report trustworthy raw requested, measured, filtered, directional, booster, and field-weakening currents without deriving torque or current-utilization percentages.
- Keep automated pass/fail scenarios in canonical conformance. Keep Refloat tuning, configuration writes, motor movement, and other actuator commands unsupported until a product workflow explicitly requires them.
- Remove the reduced eight-field simulator layout, discarded known dynamic fields, partial legacy encoder/decoder paths, and duplicated telemetry serialization that the complete normalized pipeline supersedes.

Non-goals:

- Turning fixture development into a physical-conformance runner or asserting the behavior of the separately running Float View application.
- Simulating BLE services, characteristics, CCCD availability, ATT negotiation, multiple physical advertisers, or operating-system suspension in the byte-response workbench.
- Adding Refloat tuning, configuration, lighting, flywheel, motor-control, or other write workflows.
- Deriving torque or current utilization, obtaining controller current-limit configuration, or simulating utilization metadata.
- Refloat command 35 alert/fault history and data-recorder commands 41–43.
- Simultaneous polling, display, or durable logging of more than one controller.
- Supporting prerelease, unknown, or stable Refloat package versions outside the audited 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 releases.
- Supporting VESC test builds, VESC 7.01 while it reports a nonzero test-version marker, unknown VESC releases, or firmware older than the audited 6.02 baseline.

## Capabilities

### New Capabilities

- `refloat-telemetry-pipeline`: Defines supported Refloat read commands, version-specific decoding and normalization, route-correct polling, detailed telemetry state, bounded extension handling, durable persistence, session retrieval, and export.

### Modified Capabilities

- `interactive-fixture-development`: Expands the single-controller fixture controls into an editable, shareable topology and semantic telemetry workbench with manual connection and board-state events.
- `backend-contract`: Adds typed detailed live and historical telemetry, state values, bounded transition events, bounded extensions, and consistent serialization across mock, Apple, and Android.
- `platform-hosts`: Requires both native hosts to decode, normalize, schedule, persist, retrieve, and export the same supported telemetry without platform-specific frontend behavior.
- `live-board-dashboard`: Displays detailed live board state, load, motion, control, warning, and fault information and exposes the same fields in historical session detail.
- `backend-conformance`: Covers version-specific telemetry commands, normalization, persistence, serialization, privacy, exports, frontend recovery, and representative physical workbench scenarios.

## Impact

- `conformance/fixture/` protocol encoders, semantic telemetry sources, topology/scenario documents, simulator request handling, and canonical scenario reuse.
- `conformance/fixture-dev/` contract, coordinator, validation, controls, diagnostics, import/export, and tests.
- `frontend/src/lib/backend/contract.ts`, backend client/mock/state, live dashboard, session detail, telemetry formatting, and interaction tests.
- Apple and Android Refloat codecs, VESC routing and polling, application telemetry models, loopback servers, durable SQLite schemas/stores, diagnostics, and exports.
- Canonical fixtures and conformance matrices for every audited stable Refloat release across the legacy 1.0.x, dynamic 1.1.x, and alert-capable dynamic 1.2.x profiles, composed with audited stable VESC 6.02/6.05/6.06/7.00 through direct and CAN routes.
- Persisted telemetry schema changes are cross-platform and test databases are recreated from the current schema; no migration framework or new runtime dependency is planned.
- Implementation starts only after `support-direct-and-can-vesc-connections` is completed and its route, discovery, fixture-topology, and controller-continuity contracts are the repository baseline.
