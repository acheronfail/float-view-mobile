## MODIFIED Requirements

### Requirement: Shared ride completion actions

Live SHALL place End ride after Disconnect. Every End ride action SHALL require confirmation explaining that reconnecting starts a new ride. Confirmed completion SHALL finalize the ride and disconnect when connected. Home SHALL show a paused-ride card with board, remaining window and End ride; the paused session viewer SHALL show the same status/action. Cancellation SHALL retain the ride; qualifying data SHALL remain saved. Controls SHALL support primary pointer and guarded keyboard activation.

#### Scenario: Finish a paused ride
- **WHEN** the user activates End ride on Home or the paused session viewer
- **THEN** a confirmation appears; cancellation preserves the ride and acceptance finalizes it once without reconnecting

#### Scenario: Finish while riding
- **WHEN** the user selects End ride on Live in any ride state
- **THEN** completion and disconnection occur only after confirmation

#### Scenario: Menu order
- **WHEN** the Live menu shows both Disconnect and End ride
- **THEN** End ride appears after Disconnect
