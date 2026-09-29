## MODIFIED Requirements

### Requirement: Sticky map preview

The session view SHALL render a sticky map preview when the session metadata reports track location availability or a committed ride marker contains a recorded location. The map SHALL show recorded ride marker icons, the accepted base track when present, a travelled line through points up to the selected elapsed time, and a rider marker when an eligible point exists.

#### Scenario: Location-bearing session

- **WHEN** a historical session with location data is opened
- **THEN** the session view loads bounded location pages and presents a sticky map preview alongside the existing charts

#### Scenario: Board-only session

- **WHEN** a historical session has neither track location data nor recorded marker locations
- **THEN** the existing chart/session review renders without reserving an empty map or changing board-only behavior

#### Scenario: Sparse track

- **WHEN** a session contains gaps or only one accepted point
- **THEN** the map renders the accepted data and gap-aware travelled line without fabricating a route

#### Scenario: Marker positions without a track

- **WHEN** recorded marker positions exist but no base track remains
- **THEN** the map shows the marker positions without fabricating route segments or a rider position
