## MODIFIED Requirements

### Requirement: Current ride access

The Live action menu SHALL offer Charts… and Sessions SHALL offer Current ride for an active saved session. Live SHALL NOT reserve dashboard space for a separate current-ride button. It SHALL open the shared session viewer without disconnecting or changing recording. The active viewer SHALL provide Back to Live and Recording or Reconnecting status. Guest connections SHALL NOT offer recorded current-ride review.

#### Scenario: Open and return

- **WHEN** a saved-mode rider opens Charts… from Live or Current ride from Sessions and returns to Live
- **THEN** the same connection and session remain active throughout

#### Scenario: Guest connection

- **WHEN** the connection is in guest mode
- **THEN** Charts… and Current ride are absent
