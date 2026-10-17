# Spec Delta

## MODIFIED Requirements

### Requirement: Backend-owned Bluetooth operations

The backend SHALL own permission resolution, bounded scanning, saved-board matching, stable logical identity derivation, explicit saved-or-guest connection intent, BLE stream setup, connection, controller identification and selection validation, reconnect, disconnection, protocol polling, and firmware discovery. Exactly zero or one BLE device MAY be active. A connection generation SHALL retain its tracking mode for retries and lifecycle recovery. Explicit connections SHALL originate from Home discovery results. The backend SHALL also permit a saved-mode connection from a matching discovery result during the bounded foreground auto-connect opportunity. Saved-board history and configuration routes SHALL not themselves start Bluetooth work; foreground discovery SHALL be independent of the displayed route.

#### Scenario: Bounded scan

- **WHEN** the frontend requests `bluetooth.scan`
- **THEN** the backend starts discovery, stops it within the contract bound, and publishes discovered devices with exact saved-board match information when available

#### Scenario: Competing connection

- **WHEN** a second device is requested while another device is active
- **THEN** the backend returns `invalid_state` and preserves the existing connection

#### Scenario: Known saved connection

- **WHEN** `device.connect` requests a discovered device that matches a saved board in saved mode
- **THEN** the backend accepts the saved connection and does not require a second save decision

#### Scenario: Unknown guest connection

- **WHEN** `device.connect` requests an unknown device in guest mode
- **THEN** the backend accepts the bounded connection without creating a saved-device record or reading a Board Profile as a durable write source

#### Scenario: Retry failed device

- **WHEN** `device.connect` names the same active device while its state is `failed`
- **THEN** the backend accepts a new bounded connection generation using the retained tracking mode, using the latest Board Profile only for saved mode

#### Scenario: WebView recreation

- **WHEN** the frontend WebView is destroyed and recreated
- **THEN** the native connection, tracking mode, identification, pending controller selection, and backend jobs continue where the OS permits and the new frontend recovers them through `state.get`

#### Scenario: Automatic saved connection away from Home

- **WHEN** a current discovery result matches the preferred saved board during an eligible foreground opportunity while a non-Home route is displayed
- **THEN** the backend admits one saved-mode connection through the ordinary connection and identification path without requiring a frontend connection request
