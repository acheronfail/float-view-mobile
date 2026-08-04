## ADDED Requirements

### Requirement: Live location option and warning

The live dashboard SHALL present a small non-blocking location option or warning whenever recording is disabled, permission is unresolved/denied, or location is unavailable while a board is connected. The warning SHALL not obscure board telemetry or prevent disconnect/configuration actions.

#### Scenario: Recording disabled

- **WHEN** a board is connected and `locationRecordingEnabled` is false
- **THEN** the live view offers the location-recording option without claiming that permission is missing

#### Scenario: Permission required

- **WHEN** recording is enabled but location permission is unresolved or denied
- **THEN** the live view shows a compact warning that identifies location recording as unavailable and provides the permission action

#### Scenario: Board-only fallback

- **WHEN** the user leaves location disabled or declines permission
- **THEN** the live dashboard continues presenting current board telemetry and session summaries with no map requirement

### Requirement: Re-prompt or settings activation

Activating the live location option/warning SHALL invoke the shared contract permission action. The frontend SHALL not perform platform detection; the backend SHALL choose an OS re-prompt when possible or platform settings navigation otherwise.

#### Scenario: Re-prompt available

- **WHEN** the user activates the warning and the OS allows another permission prompt
- **THEN** the native backend presents the prompt and publishes the resulting status

#### Scenario: Settings required

- **WHEN** the user activates the warning after a permanent denial or restricted state
- **THEN** the native backend presents the platform settings destination or reports a bounded unavailable error while board logging remains active

### Requirement: Location status recovery

The live dashboard SHALL derive its warning and option from complete authoritative state and SHALL recover it after a frontend recreation without storing permission state only in the WebView.

#### Scenario: Recreated live view

- **WHEN** the live WebView reconnects during an active session
- **THEN** it renders the current location status/action availability from `state.get` and does not require a new board connection
