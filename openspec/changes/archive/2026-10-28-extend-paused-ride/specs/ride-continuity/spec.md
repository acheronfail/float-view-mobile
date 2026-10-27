## ADDED Requirements

### Requirement: Extend a paused ride

Home and the paused session viewer SHALL offer Give me more time beside End ride. Each accepted extension SHALL retain the ride identity and restart the configured resume window from backend time without shortening the existing deadline. Extensions SHALL survive process restart and replace expiry scheduling without initiating Bluetooth or recording work.

#### Scenario: Repeated extension and resume

- **WHEN** a rider extends an unexpired paused ride repeatedly and later reconnects before the renewed deadline
- **THEN** the countdown reflects the latest deadline and recording resumes the same ride with the existing honest recording gap

#### Scenario: Restart and stale expiry

- **WHEN** the process restarts after an extension or an old expiry callback arrives
- **THEN** the renewed deadline remains authoritative and the ride closes only at that deadline

#### Scenario: Interactive controls and failure

- **WHEN** the action is activated by a primary pointer or keyboard
- **THEN** one request is sent, duplicate activation is guarded while pending, and failures appear on the card without claiming success
- **AND** non-primary pointers do not activate the action

#### Scenario: Clear pause wording and feedback

- **WHEN** a paused ride is shown
- **THEN** the card explains the remaining reconnect time and that reconnecting after expiry starts a new ride
- **WHEN** Give me more time succeeds
- **THEN** the updated countdown is shown with a brief confirmation of the remaining reconnect time
- **AND** failed requests show an error without success feedback
