## MODIFIED Requirements

### Requirement: Lifecycle and connection isolation

Policy SHALL run independently of the frontend and recording cadence. Disconnection, controller replacement, loss of fresh valid board context, and rule configuration changes SHALL cancel pending automatic speech. Reconnection SHALL require fresh telemetry and retain process-local cooldown. Process restart SHALL retain settings but discard speech, readings and deadlines. Late callbacks SHALL not affect a replacement operation.

#### Scenario: Volume-only edit

- **WHEN** volume changes during an utterance
- **THEN** that utterance retains its admitted gain and the next utterance uses the new volume

#### Scenario: WebView replaced

- **WHEN** the WebView is replaced or its listener restarts during a connection
- **THEN** announcement policy continues with the existing service without duplicate timers or reset cooldown

#### Scenario: Disconnect and reconnect

- **WHEN** a board disconnects above threshold and reconnects
- **THEN** its old reading cannot trigger speech, and only a fresh qualifying reading after cooldown can trigger another attempt

#### Scenario: Disable or drop below threshold

- **WHEN** users disable both rules or no enabled reading qualifies
- **THEN** disabling rules cancels pending automatic speech; a threshold crossing with fresh valid readings lets admitted speech finish, prevents further attempts below threshold, and never replays an old reading

#### Scenario: Drop below threshold during playback

- **WHEN** fresh valid readings drop to or below every enabled threshold during automatic speech
- **THEN** the admitted utterance finishes without cancellation, no further speech starts while readings remain below threshold, and later qualifying readings obey the existing cooldown without replaying old values

#### Scenario: Stale readings during playback

- **WHEN** the active board's readings become two seconds old or its controller context becomes invalid during automatic speech
- **THEN** pending automatic speech is cancelled
