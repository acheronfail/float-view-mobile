## ADDED Requirements

### Requirement: Bounded location-track review

The session-data plane SHALL provide an authenticated, bounded `locations` query using the session ID, published snapshot revision, cursor/limit, and elapsed-time range. Results SHALL include ordered location points, explicit gaps, the next cursor when present, and the location boundary read.

#### Scenario: Session with location data

- **WHEN** the frontend requests a valid location page against the current published snapshot
- **THEN** the backend returns only the requested bounded range and a continuation cursor when more rows remain

#### Scenario: Stale snapshot

- **WHEN** a location query names a superseded published snapshot
- **THEN** the backend returns the contract's bounded conflict/not-found result and the frontend refreshes session metadata before retrying

#### Scenario: Board-only session

- **WHEN** the session has no accepted location rows
- **THEN** the location query returns an empty bounded result and the session review remains fully usable without a map

### Requirement: Sticky map preview

The session view SHALL render a sticky map preview only when the session metadata reports location availability. The map SHALL show the accepted base track, a travelled line through points up to the selected elapsed time, and a rider marker when an eligible point exists.

#### Scenario: Location-bearing session

- **WHEN** a historical session with location data is opened
- **THEN** the session view loads bounded location pages and presents a sticky map preview alongside the existing charts

#### Scenario: Board-only session

- **WHEN** a historical session has no location data
- **THEN** the existing chart/session review renders without reserving an empty map or changing board-only behavior

#### Scenario: Sparse track

- **WHEN** a session contains gaps or only one accepted point
- **THEN** the map renders the accepted data and gap-aware travelled line without fabricating a route

### Requirement: Upstream rider visual treatment

The map SHALL reuse the rider icon asset and marker treatment from `vendor/float-view/src/assets/rider-icon.svg` and the upstream Float View map example, while preserving attribution for the selected tile provider.

#### Scenario: Rider marker

- **WHEN** the selected elapsed time has an eligible location point
- **THEN** the map displays the upstream rider icon at that point with the same bounded marker anchoring behavior

#### Scenario: Tile attribution

- **WHEN** the map is rendered with remote tiles
- **THEN** the required provider attribution remains visible and no geocoding or navigation control is added

### Requirement: Shared elapsed-time selection

The chart and map SHALL use one selected session-relative elapsed-time value. Selection changes SHALL update the travelled polyline and rider marker without issuing an unbounded network request for every pointer movement.

#### Scenario: Move chart selection

- **WHEN** the user moves the selected point in the historical charts
- **THEN** the map advances its travelled line and rider marker to the latest eligible location point at or before that elapsed time

#### Scenario: Gap boundary

- **WHEN** the selected time falls inside a location gap
- **THEN** the map keeps the last eligible travelled point and does not interpolate across the gap

#### Scenario: Gap-free pair

- **WHEN** the selected time lies between two adjacent points in the same gap-free segment and interpolation is permitted by the documented tolerance
- **THEN** the rider marker may be bounded-interpolated while the travelled line remains composed only of accepted track points

### Requirement: Deterministic CSV location projection

When `includeGpsLocationWhenExporting` is enabled, CSV export SHALL add `gpsAccuracy`, `gpsLongitude`, and `gpsLatitude` columns to telemetry rows by selecting the nearest eligible recorded location point within the documented tolerance and repeating that point's values across eligible rows. Equal-distance candidates SHALL be resolved by the lower location sequence, and rows without an eligible point SHALL contain blank GPS fields. CSV SHALL NOT interpolate coordinates; the independent location track remains available through session review and bounded location reads.

#### Scenario: Repeated nearest location

- **WHEN** a telemetry row has one or more recorded location points within the export tolerance and GPS export is enabled
- **THEN** the CSV row contains the nearest point's accuracy, longitude, and latitude, and adjacent eligible telemetry rows may repeat the same values

#### Scenario: Tie break

- **WHEN** two eligible location points are equally distant from a telemetry row
- **THEN** the CSV row uses the point with the lower location sequence

#### Scenario: No eligible point

- **WHEN** no location point falls within the export tolerance
- **THEN** the telemetry row contains blank GPS fields and the independent location track remains available through session review and bounded reads

#### Scenario: GPS export disabled

- **WHEN** the user exports a session while GPS export is disabled
- **THEN** the CSV omits the GPS columns regardless of whether the session contains location data
