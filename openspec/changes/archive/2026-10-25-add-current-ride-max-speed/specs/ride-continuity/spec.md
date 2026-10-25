## ADDED Requirements

### Requirement: Current ride maximum speed

The service SHALL publish the greatest absolute finite speed observed in the current ride, or unavailable before any valid speed. Reconnects and view recreation SHALL preserve it. Resuming a saved ride SHALL restore its committed peak. A new ride SHALL clear it. Tracking SHALL use bounded state and SHALL NOT estimate speeds in recording gaps or change supported recordings.

#### Scenario: Forward and reverse peaks

- **WHEN** a ride observes speeds of 20, -30 and 10 km/h
- **THEN** its maximum is 30 km/h and later missing speed does not erase it

#### Scenario: Reconnect and restart

- **WHEN** the same ride reconnects or resumes after process replacement
- **THEN** its retained or committed peak remains and increases only for a greater observed speed

#### Scenario: New ride and unavailable data

- **WHEN** a new ride starts without valid speed or the prior ride has no saved peak
- **THEN** its maximum is unavailable until a finite speed is observed and no previous ride peak leaks

#### Scenario: Persistence failure

- **WHEN** a ride cannot be restored because storage fails
- **THEN** the existing failure is reported without fabricating a successful restoration or deleting supported data
