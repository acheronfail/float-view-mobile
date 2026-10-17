# Spec Delta

## MODIFIED Requirements

### Requirement: Home and live connection flow

Home SHALL show Bluetooth readiness, discovery, and connection controls when no board is active. Discovery SHALL include an advertising saved board whose exact platform identifier matches even when its advertisement omits the UART service, while unrelated advertisements remain hidden. An accepted explicit connection request, an automatic connection while Home is displayed, or a recovered active connection without a preferred restored destination SHALL navigate to a live view that represents connecting, identifying, selecting-controller, ready, reconnecting, disconnected, and failed backend states. During frontend startup, an eligible process-lifetime UI checkpoint destination SHALL take precedence over recovered Home-to-Live navigation. Foreground auto-connect SHALL preserve a displayed or restoring non-live destination; its lifecycle discovery SHALL not depend on mounting Home.

#### Scenario: Accepted connection

- **WHEN** the backend accepts a device connection request
- **THEN** the frontend navigates to the live view and renders subsequent identification, selection, and connection states from authoritative snapshots

#### Scenario: Recovered active connection

- **WHEN** the frontend reconnects after lifecycle recreation, `state.get` contains an active connection and no eligible checkpoint destination takes precedence
- **THEN** it replaces the inactive Home view with the live view without starting another native connection

#### Scenario: Restored non-live destination

- **WHEN** the frontend is recreated with an eligible Sessions, session detail, board history or Settings checkpoint while a connection remains active
- **THEN** it restores that destination without briefly navigating to Live, mounting Home for discovery or starting another native connection

#### Scenario: Recovered controller selection

- **WHEN** the recovered active connection is awaiting a controller choice and the chosen destination is Live
- **THEN** the live route presents that choice from authoritative candidates without returning to Home

#### Scenario: Recovered failed connection

- **WHEN** the recovered active connection is failed with a connection issue and the chosen destination is Live
- **THEN** the live route retains its configuration, retry, and disconnect actions without returning to Home

#### Scenario: Live route without a connection

- **WHEN** the live route is opened while authoritative state has no active connection
- **THEN** the frontend returns to Home

#### Scenario: Explicit disconnect

- **WHEN** the active board is disconnected from the live view
- **THEN** the frontend returns to Home after authoritative state reports no active connection

#### Scenario: Automatic connection during restoration

- **WHEN** foreground auto-connect admits a connection before or after a non-live checkpoint destination is restored
- **THEN** the frontend preserves that destination and reflects connection state through normal authoritative snapshots without navigation to Live

#### Scenario: Automatic connection from Home

- **WHEN** foreground auto-connect admits a connection while Home is displayed and no pending restored destination takes precedence
- **THEN** Home transitions to Live and renders the normal connection flow without submitting another connection command
