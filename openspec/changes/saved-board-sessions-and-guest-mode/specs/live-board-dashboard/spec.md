## MODIFIED Requirements

### Requirement: Active-session live summary

The live view SHALL present the active saved session's sample count, riding elapsed time, total elapsed time, distance, maximum speed, maximum duty cycle, state, and integrity from the same authoritative snapshot as the latest persisted sample. When the connection is in guest mode and no active session exists, the live view SHALL present live telemetry and a clear temporary/no-history notice instead of session aggregates.

#### Scenario: Persisted sample updates summary

- **WHEN** a saved-mode sample is successfully persisted
- **THEN** the dashboard receives the resulting latest sample and current active-session aggregates in one newer authoritative snapshot

#### Scenario: Guest sample updates live view

- **WHEN** a guest sample is published for live display
- **THEN** the dashboard updates its telemetry, may show the current-ride `distanceM` value when Refloat supplies it, shows that the connection is not being saved or tracked, and renders no fabricated session aggregate or lifetime odometer value

#### Scenario: Unavailable aggregate

- **WHEN** an optional saved-session aggregate has not been established
- **THEN** its assigned cell displays a placeholder rather than a fabricated zero
