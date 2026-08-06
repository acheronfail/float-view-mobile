## ADDED Requirements

### Requirement: Dedicated standalone BMS page

The shared frontend SHALL route an active connection with authoritative `deviceKind: bms` to a dedicated full-page BMS experience that reuses the same BMS content component as the live-board sheet. The page SHALL present authoritative status, freshness, values, unavailable state, and units, expose disconnect, and omit board views, Refloat controls, controller shutdown, Board configuration, ride state, and session controls.

#### Scenario: Standalone BMS becomes classified

- **WHEN** an accepted asynchronous connection publishes `deviceKind: bms`
- **THEN** the frontend replace-navigates to the BMS page and renders the current authoritative BMS state

#### Scenario: Board connection remains live

- **WHEN** the active connection publishes `deviceKind: board`
- **THEN** the frontend uses the existing live-board page and retains the BMS sheet for any auxiliary BMS

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated while a BMS-only connection remains active
- **THEN** bootstrap state routes directly to the BMS page without restarting native discovery or polling

#### Scenario: BMS disconnect

- **WHEN** the user activates disconnect from the standalone BMS page
- **THEN** the shared device disconnect method runs once and successful authoritative removal returns navigation to Home

#### Scenario: Route mismatch

- **WHEN** the BMS page observes no active connection or a board connection, or the live page observes a BMS connection
- **THEN** it replace-navigates to Home or the authoritative connection-kind route without exposing incompatible controls
