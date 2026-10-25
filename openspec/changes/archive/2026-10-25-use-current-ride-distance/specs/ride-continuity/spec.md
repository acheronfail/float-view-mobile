## ADDED Requirements

### Requirement: Shared current-ride live distance

Live distance widgets, Live Activity and Dynamic Island SHALL display accumulated current-ride distance using observed speed over time, restored from saved distance on resume. Raw board trip resets SHALL NOT reset this total. Reconnect and missing-speed intervals SHALL add no unobserved distance. New rides SHALL start at zero. Accumulation SHALL use bounded state and preserve raw historical trip telemetry and supported recordings.

#### Scenario: Board powers off during a break

- **WHEN** a saved ride resumes after the board restarts and reports zero trip distance
- **THEN** every live distance display retains the saved ride distance and adds subsequent observed travel

#### Scenario: Reconnect without board reset

- **WHEN** the same ride reconnects repeatedly while the board trip counter retains its value
- **THEN** accumulated distance is retained without adding the board's previous trip distance again or integrating offline time

#### Scenario: Process replacement

- **WHEN** the app restarts and resumes an eligible saved ride
- **THEN** the first live distance uses the restored saved total without integrating the interruption

#### Scenario: Missing speed or failed persistence

- **WHEN** speed is missing or a saved sample fails persistence
- **THEN** no travel is fabricated across the missing observation and no failed saved sample is published as committed

#### Scenario: New ride and temporary connection

- **WHEN** a new saved ride or temporary connection begins
- **THEN** live distance starts at zero and advances only from observed travel in that ride
