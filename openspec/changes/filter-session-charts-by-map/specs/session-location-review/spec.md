# Spec Delta

## ADDED Requirements

### Requirement: Explicit map filtering mode

Session review SHALL offer an off-by-default **Filter charts by map** toggle when a complete usable location track is available. The mode SHALL be transient to the open review. Enabling it SHALL filter every telemetry chart by the current map area; disabling it SHALL restore the preceding ordinary chart viewport.

#### Scenario: Enable from a zoomed chart
- **WHEN** the user enables filtering after zooming the ordinary charts
- **THEN** the current map bounds select intervals from the full loaded track, without intersecting the earlier chart viewport or automatically moving the map

#### Scenario: Disable or hide map
- **WHEN** the user disables filtering or hides the map
- **THEN** filtering turns off and the previous ordinary chart viewport and navigation become available

#### Scenario: Unavailable track
- **WHEN** only marker locations, isolated points, incomplete track data, or a failed track load are available
- **THEN** spatial filtering is unavailable with an explanation, and ordinary session review remains usable

#### Scenario: Reopen review
- **WHEN** the user changes sessions or recreates the review
- **THEN** filtering starts disabled and no prior session's spatial selection is reused

### Requirement: Geographic selection preserves ride chronology

Map filtering SHALL include the portions of continuous recorded route segments inside the visible geographic bounds, ordered by original ride time. Boundary crossings SHALL be interpolated only within eligible continuous location segments. Explicit location gaps and excessive time gaps SHALL not create inferred included intervals.

#### Scenario: Repeated visits
- **WHEN** the route leaves and later re-enters the visible area
- **THEN** each visit produces its own ordered included interval and intervening out-of-area time is excluded

#### Scenario: Crossing with endpoints outside
- **WHEN** a continuous route edge crosses the visible area while both recorded endpoints lie outside it
- **THEN** the crossing portion is included using its interpolated entry and exit times

#### Scenario: Unknown location
- **WHEN** a time span lies in a location gap or has no continuous track
- **THEN** that span is excluded without extrapolating a route from isolated points or markers

#### Scenario: Empty area
- **WHEN** no positive-duration route portion intersects the map
- **THEN** charts show an explicit empty-area state and offer a full-route reset instead of showing unfiltered or stale telemetry

### Requirement: Disjoint chart timeline

Filtered charts SHALL concatenate included intervals in chronological order, with width proportional to included duration and aligned dashed vertical separators between disjoint intervals. Data paths and tooltip interpolation SHALL not bridge excluded time. Inspection and labels SHALL retain original ride timestamps, and automatic value scales SHALL use only included telemetry.

#### Scenario: Multiple intervals
- **WHEN** the map includes two two-minute visits separated by ten excluded minutes
- **THEN** each visit receives equal chart duration width, the excluded ten minutes occupy no duration width, and a separator marks the discontinuity on every chart

#### Scenario: Inspect near a separator
- **WHEN** the user inspects either side of a separator
- **THEN** the chart resolves to the corresponding original ride time and updates the shared map selection without interpolating across the break

#### Scenario: Marker outside selection
- **WHEN** a selected marker or fault lies outside the included intervals
- **THEN** it does not expand the filtered charts or recenter the map, and chart inspection is cleared with an outside-filter indication

### Requirement: Map owns filtered chart navigation

While map filtering is enabled, every chart zoom and pan control and gesture SHALL be disabled, including drag, pinch, wheel, and keyboard navigation paths that change the viewport. Inspection and field visibility SHALL remain usable. Map-driven chart updates and chart selection SHALL not automatically change map bounds.

#### Scenario: Chart gesture during filtering
- **WHEN** the user attempts chart zoom or pan while filtering is enabled
- **THEN** the included intervals and map bounds remain unchanged, and disabled navigation controls explain that the map controls the range

#### Scenario: Settle map movement
- **WHEN** map pan or zoom finishes
- **THEN** all charts use the new included intervals without chart-to-map recenter feedback

#### Scenario: Reset route
- **WHEN** the user requests full-route reset while filtering is enabled
- **THEN** the map fits the complete route and charts include all eligible located intervals while filtering remains enabled

### Requirement: Bounded and revision-consistent spatial review

Spatial filtering SHALL use complete bounded track data and bounded telemetry requests against one published snapshot. Superseded work SHALL be cancelled or ignored. Loading, failures, and resource limits SHALL be visible; partial data SHALL never be presented as a complete spatial result. A retry or disabling the filter SHALL remain available after failure.

#### Scenario: More than four location pages
- **WHEN** the complete track requires more than four pages within the supported review limits
- **THEN** all pages are considered before filtering is available, including portions near the end of the ride

#### Scenario: Resource limit
- **WHEN** track size, interval count, or decoded result size exceeds the supported limit
- **THEN** the view reports that filtering cannot complete and does not silently truncate or merge across excluded portions

#### Scenario: Rapid movement or teardown
- **WHEN** map bounds change again or review closes while queries are pending
- **THEN** obsolete results cannot overwrite the current view and pending work is released

#### Scenario: Transport or snapshot failure
- **WHEN** a filtered query fails, reconnects, or encounters a superseded snapshot
- **THEN** the view reports loading or failure, refreshes metadata when required, and retries with bounded attempts without mixing snapshots
