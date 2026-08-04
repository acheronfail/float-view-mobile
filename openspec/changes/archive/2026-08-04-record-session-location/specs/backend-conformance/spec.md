## ADDED Requirements

### Requirement: Location contract coverage

The canonical suite SHALL cover the location setting, permission/status actions, stable errors, state boundary metadata, authenticated `locations` pages, cursor/limit bounds, stale snapshots, and board-only fallback for the mock and every supported native backend.

#### Scenario: Permission matrix

- **WHEN** conformance exercises not-determined, granted, denied, restricted/unavailable, and settings-required fixtures
- **THEN** every target returns the same bounded status/action semantics and board logging remains usable

#### Scenario: Location query

- **WHEN** conformance requests a valid page, an over-limit page, and a stale-snapshot page
- **THEN** it verifies ordered bounded results, explicit gaps, continuation behavior, and the expected stable error

### Requirement: Location persistence and projection coverage

The canonical suite SHALL verify monotonic location sequencing, elapsed-time alignment, accuracy preservation, durable snapshot boundaries, restart recovery, provisional-session deletion, pruning, explicit gaps, deterministic nearest-row CSV tie-breaking, blank unmatched CSV fields, and independent archive-track retention.

#### Scenario: Sparse location track

- **WHEN** a fixture emits telemetry at a higher cadence than location observations with a deliberate gap
- **THEN** conformance verifies no fabricated coordinates, correct gap metadata, and correct chart/map/CSV time projection

#### Scenario: Recovery boundary

- **WHEN** the backend is interrupted during a location append and restarted
- **THEN** conformance verifies that only the last complete location boundary is readable and board telemetry remains consistent

### Requirement: Background and frontend recreation coverage

Physical iOS and Android conformance SHALL include an authorized location-enabled `riding` state, a mobile background transition, fixture telemetry during the transition, foreground/rebootstrap, and verification that native location rows and boundaries continued without a WebView-owned recorder.

#### Scenario: Android background checkpoint

- **WHEN** Android backgrounds an active location-enabled session and then returns to the foreground
- **THEN** the shared suite reads additional location rows and the same session boundary after bootstrap refresh

#### Scenario: iOS background checkpoint

- **WHEN** iOS backgrounds an active location-enabled session and then returns to the foreground
- **THEN** the shared suite reads preserved native state and additional location rows when Core Location delivered them, or the documented bounded unavailable/gap result

### Requirement: Privacy and battery diagnostics

Conformance SHALL assert that coordinates do not appear in complete state events or default diagnostics and SHALL record bounded provider/update counts for battery-policy verification without requiring a user-configurable interval.

#### Scenario: Coordinate redaction

- **WHEN** conformance captures state events and failure diagnostics after location writes
- **THEN** no latitude, longitude, or full location payload appears outside an explicit session-data/export assertion

#### Scenario: Update count

- **WHEN** a session enters and exits the location-eligible state
- **THEN** provider/update accounting shows capture started and stopped within bounded lifecycle timing and no idle-only updates were persisted
