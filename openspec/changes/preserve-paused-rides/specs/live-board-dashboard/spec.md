## MODIFIED Requirements

### Requirement: Conditional disconnect confirmation

Disconnect SHALL proceed without confirmation only when the latest persisted ride state is exactly `idle`. Disconnect SHALL pause eligible saved rides; End ride SHALL finalize and disconnect using the same confirmation rule. Confirmation copy SHALL distinguish these outcomes. A non-idle, unknown, or unavailable latest ride state SHALL require explicit confirmation before the disconnect request is sent.

#### Scenario: Idle disconnect

- **WHEN** the latest persisted ride state is `idle` and the user selects Disconnect
- **THEN** the frontend requests disconnect immediately and returns Home after authoritative state removes the active connection

#### Scenario: Riding disconnect

- **WHEN** the latest persisted ride state is not `idle` and the user selects Disconnect
- **THEN** the frontend requests confirmation and disconnects only after the user confirms

#### Scenario: No live sample

- **WHEN** no latest persisted sample is available and the user selects Disconnect
- **THEN** the frontend requests confirmation before disconnecting

