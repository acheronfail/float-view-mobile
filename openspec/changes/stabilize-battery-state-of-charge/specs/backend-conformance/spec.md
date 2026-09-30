## ADDED Requirements

### Requirement: Dynamic battery estimation conformance

Canonical conformance SHALL cover temporal battery estimation and the optional capacity contract across the shared server, Apple and Android host integrations, and frontend mock. Deterministic telemetry sequences SHALL exercise known charge consumption, load/recovery steps, regeneration, charging, settled correction, missing current/voltage, profile changes, duplicate samples, persistence rejection, gaps, and reconnect. Oracles SHALL use independently calculated charge and observable temporal bounds rather than reproduce the estimator implementation. Recording/export assertions SHALL establish unchanged raw telemetry semantics. Scenario orchestration SHALL remain in conformance rather than device firmware.

#### Scenario: Dynamic load sequence

- **WHEN** a fixture applies acceleration, sustained load, release, regeneration, and stable rest at known timestamps
- **THEN** assertions verify charge accounting, absence of instantaneous percentage jumps, rest qualification, and gradual correction

#### Scenario: Failure and lifecycle sequence

- **WHEN** telemetry is duplicated, reordered, rejected, interrupted, or resumed, or the WebView is recreated
- **THEN** charge is counted only once for accepted observed intervals and provisional/method state matches lifecycle rules

#### Scenario: Host and mock parity

- **WHEN** equivalent configuration and accepted telemetry are presented through supported host integrations and the mock
- **THEN** capacity validation, method/provisional fields, percentage bounds, and lifecycle outcomes agree within documented numeric and timing tolerances

#### Scenario: Evidence limits

- **WHEN** validation uses synthetic fixtures without measured ride traces or a reference charge measurement
- **THEN** results describe verified stability and accounting rather than claim measured absolute SOC accuracy
