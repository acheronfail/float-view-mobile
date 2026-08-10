## ADDED Requirements

### Requirement: Lighting query and mutation distinction

Canonical conformance SHALL prove that the shared Refloat simulator distinguishes zero-mask lighting status queries from nonzero-mask lighting mutations across representative legacy and stable releases, direct and CAN routes, and normal and flywheel modes. Evidence SHALL include exact query responses, unchanged disposable state, retained wrong-route and malformed rejection, retained test-version read-only behavior, and retained flywheel rejection for every actual mutation. The focused physical gate SHALL exercise direct and CAN writable control sequences on Android, iOS, and macOS without timing waivers.

#### Scenario: Deterministic query matrix

- **WHEN** software conformance sends source-correct zero-mask lighting requests through representative supported versions and routes
- **THEN** each request receives the current bounded lighting response without state mutation or a safety diagnostic

#### Scenario: Flywheel race regression

- **WHEN** a scheduled lighting refresh occurs between flywheel enable and disable in the canonical control sequence
- **THEN** the query succeeds, flywheel disable remains reachable, and every nonzero-mask competing mutation remains rejected

#### Scenario: All-host physical regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on Android, iOS, and macOS
- **THEN** every host completes its route-correct sequence without fixture rejection or motor movement

