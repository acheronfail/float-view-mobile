## ADDED Requirements

### Requirement: Current ride access

Live and Sessions SHALL offer Current ride for an active saved session. It SHALL open the shared session viewer without disconnecting or changing recording. The active viewer SHALL provide Back to Live and Recording or Reconnecting status. Guest connections SHALL NOT offer recorded current-ride review.

#### Scenario: Open and return

- **WHEN** a saved-mode rider opens Current ride and returns to Live
- **THEN** the same connection and session remain active throughout

#### Scenario: Guest connection

- **WHEN** the connection is in guest mode
- **THEN** Current ride is absent

### Requirement: Ongoing ride review

The active viewer SHALL follow the growing persisted ride by default, using bounded consistent snapshots. Inspection SHALL pause following and preserve chart selections, field visibility and map position. Jump to live SHALL resume following. Refresh SHALL stop while hidden or unmounted, allow at most one metadata refresh in flight, and reject stale responses after navigation.

#### Scenario: Inspect and resume

- **WHEN** the user pans, zooms, selects a point, or navigates or filters the map
- **THEN** automatic following pauses until Jump to live, with recording continuing independently

#### Scenario: Growing ride

- **WHEN** the visible viewer follows an active ride
- **THEN** charts, markers, events and recorded location refresh at a bounded cadence without resetting field choices

#### Scenario: Reconnect and completion

- **WHEN** a connection reconnects or its session completes
- **THEN** existing history stays visible, reconnecting is indicated, and a retained completed session becomes ordinary review with final data

#### Scenario: Refresh failure or discarded ride

- **WHEN** refresh fails or a provisional ride is discarded at close
- **THEN** the viewer preserves the last successful data, reports the problem, and provides return navigation

#### Scenario: Hidden or replaced viewer

- **WHEN** the viewer is hidden, closed or replaced by another session
- **THEN** polling stops and outstanding responses cannot replace another session's data
