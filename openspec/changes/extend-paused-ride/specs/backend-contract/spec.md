## ADDED Requirements

### Requirement: Pause extension command

Authenticated `session.extendPause` SHALL accept only `{ sessionId: string }` and return Accepted for the matching unexpired paused ride. Apple, Android and the mock SHALL publish its renewed deadline in a newer complete snapshot. The deadline SHALL be the greater of its current value and backend time plus the configured resume window. Requests for active, expired, ended or different rides SHALL fail with `invalid_state`; malformed parameters SHALL fail with `invalid_request`.

#### Scenario: Authoritative extension

- **WHEN** a matching paused ride is extended
- **THEN** the deadline is persisted before success, settings and ride identity are retained, and a replaced frontend sees the renewed deadline

#### Scenario: Invalid or expired request

- **WHEN** the identity is stale, the ride is active, or backend time has reached its deadline
- **THEN** the request fails with invalid_state and cannot revive or extend another ride

#### Scenario: Persistence failure

- **WHEN** the renewed deadline cannot be stored
- **THEN** the request fails without publishing an uncommitted deadline or deleting stored data

#### Scenario: Shorter configured window

- **WHEN** the configured window would produce an earlier deadline
- **THEN** extension preserves the existing later deadline
