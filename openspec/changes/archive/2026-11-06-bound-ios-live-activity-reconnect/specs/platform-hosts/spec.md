## ADDED Requirements

### Requirement: Bounded iOS background reconnect presentation

iOS SHALL retain an existing Live Activity for at most a short grace period after observing background connection loss while execution is available. It SHALL remove it at grace expiry or earlier when background execution is denied or expires. Presentation cleanup SHALL preserve backend reconnect and ride-resume policy. Abrupt termination before cleanup is outside this guarantee.

#### Scenario: Brief loss recovers

- **WHEN** the board reconnects within the background grace period
- **THEN** the same activity resumes readings and the cleanup execution allowance is released

#### Scenario: Board stays unavailable

- **WHEN** background reconnect reaches its grace deadline
- **THEN** the activity is immediately removed and its execution allowance is released while backend reconnect remains independent

#### Scenario: Execution unavailable

- **WHEN** extra background execution is denied or expires during the grace period
- **THEN** cleanup is requested immediately without waiting for the backend reconnect timeout

#### Scenario: Foreground transition

- **WHEN** the app backgrounds with an existing reconnecting activity
- **THEN** the bounded grace period begins
- **WHEN** the app returns to the foreground before grace expiry
- **THEN** the background cleanup allowance is released and normal foreground reconnect presentation continues

#### Scenario: Reconnect after presentation ended

- **WHEN** the board reconnects after background presentation cleanup
- **THEN** backend operation can resume but the activity remains absent until the app is foregrounded and connected

#### Scenario: Repeated lifecycle and replacement

- **WHEN** connection generations change, a person dismisses the activity, or repeated lifecycle notifications arrive
- **THEN** cleanup work remains bounded, old expiration callbacks cannot end a replacement activity, and duplicate notifications do not extend the grace deadline
