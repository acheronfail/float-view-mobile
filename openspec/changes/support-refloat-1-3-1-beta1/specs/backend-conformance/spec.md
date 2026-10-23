## ADDED Requirements

### Requirement: Refloat beta source and identity evidence

Canonical fixtures SHALL derive from exact `v1.3.1-beta1` source and record its immutable commit, INFO suffix, command-33 profile, command-35 masks/records, stop code, control payloads and full configuration identity. Software evidence SHALL prove exact admission and preserved behavior for every previously supported release.

#### Scenario: Identity boundary matrix

- **WHEN** canonical tests enumerate the audited beta, stable 1.3.1, other suffixes, malformed identities and existing audited releases
- **THEN** only the audited catalog is admitted and every beta configuration/cache/Tune identity retains its suffix

#### Scenario: Source-derived schema

- **WHEN** the beta configuration fixtures are generated and decoded
- **THEN** computed fingerprint, wire signature, field order, bounds, payload sizes and verified readback match the pinned source without changing frozen baseline fixtures

### Requirement: Cross-host IMU timeout evidence

Focused codec, service, persistence, frontend and canonical conformance tests SHALL prove timeout activation, clearing, mixed firmware faults, transient recovery, deduplication, bounds, malformed-input rejection, mutation gates and reconnect isolation. Both native host integrations SHALL expose the same contract through the shared service.

#### Scenario: Direct and CAN parity

- **WHEN** Apple and Android integrations process equivalent beta scenarios directly and through CAN
- **THEN** normalized live state, timeout history, controls, saved/guest behavior and exports agree and forwarding occurs exactly once

#### Scenario: Preservation and failure evidence

- **WHEN** tests reopen supported storage, import baseline archives, inject failed commits, overlap history cursors or replace the connection generation
- **THEN** existing data remains readable, duplicates and partial commits are absent, and stale responses cannot alter current state

#### Scenario: Focused physical checkpoint

- **WHEN** beta interoperability is assessed on an actual beta-equipped controller over affected routes and hosts
- **THEN** the report identifies the exact package/build and exercised discovery, telemetry, configuration/readback, controls and reconnect behavior, separates simulated timeout injection from real firmware timeout evidence, and records unavailable coverage explicitly

#### Scenario: Honest verification scope

- **WHEN** the change is reviewed for completion
- **THEN** checks follow affected consumers under the repository verification policy; no unrelated full-platform matrix is required and simulated or software evidence is not reported as actual IMU failure validation
