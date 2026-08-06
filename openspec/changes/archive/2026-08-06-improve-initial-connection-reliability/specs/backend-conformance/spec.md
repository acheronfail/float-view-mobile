## ADDED Requirements

### Requirement: Initial identification recovery parity

The canonical software suite SHALL verify that Apple and Android implement the same bounded recovery behavior for a missing initial direct firmware response, including generation cancellation and retry exhaustion.

#### Scenario: Dropped first response recovers on both backends

- **WHEN** the simulated BLE peer drops the first direct firmware response and returns a valid response to the recovery request
- **THEN** each native backend reaches `ready` after exactly one transport connection and two direct firmware writes

#### Scenario: Recovery remains bounded and generation-safe

- **WHEN** both direct firmware responses are absent or the connection generation changes before the recovery callback
- **THEN** conformance observes no third request, no stale state mutation, and no durable session before valid identification completes
