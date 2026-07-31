## ADDED Requirements

### Requirement: Progressive SVG historical session visualisation

The shared session-review frontend SHALL render historical numeric telemetry with responsive SVG charts whose geometry is bounded by the chart's current horizontal resolution rather than the session's raw sample count. It SHALL first present session metadata and a useful full-duration overview, retain stable catalog names, units, and null gaps, and avoid loading the complete lossless ride into WebView memory. One bounded overview request is preferred; deferred chart-group loading SHALL be introduced only through one small reusable mechanism when needed to satisfy the performance bound.

#### Scenario: Long-session overview

- **WHEN** a user opens a 150,000-sample session containing the complete detailed telemetry catalog
- **THEN** useful overview SVG geometry becomes visible within one second on representative supported mobile hardware without creating one frontend object per raw sample

#### Scenario: Missing field interval

- **WHEN** the backend reports separate segments around unavailable telemetry
- **THEN** the SVG leaves a visible gap and does not connect or substitute the missing values

#### Scenario: Responsive resize

- **WHEN** the chart's rendered width changes
- **THEN** its requested resolution and SVG geometry converge on the new width without fetching the complete ride

### Requirement: Arbitrary historical chart navigation

Historical SVG charts SHALL provide bounded full-range reset, horizontal pan, two-pointer pinch zoom, and crosshair selection through Pointer Events with capture and cancellation. Touch interaction SHALL preserve usable vertical page scrolling. Equivalent visible controls and keyboard operation SHALL support zoom, pan, reset, and exact-value inspection without double activation. Settled pan or zoom input SHALL abort superseded requests and render the requested range within 250 milliseconds on representative supported mobile hardware.

#### Scenario: Pinch into a short incident

- **WHEN** a primary two-pointer gesture narrows a four-hour overview to a short elapsed range
- **THEN** the chart requests that range at display resolution and renders its extrema-preserving SVG result within the settled-interaction target

#### Scenario: Horizontal pan

- **WHEN** a user pans a zoomed chart horizontally
- **THEN** the elapsed window remains within session bounds, superseded requests are cancelled, and the newest completed request wins

#### Scenario: Cancelled gesture

- **WHEN** a pointer is cancelled or a non-primary pointer attempts a one-pointer action
- **THEN** no stale selection or unintended range mutation is committed

#### Scenario: Keyboard review

- **WHEN** a keyboard or assistive-technology user operates the chart controls
- **THEN** the same bounded zoom, pan, reset, and exact-value capabilities are available with native control semantics

### Requirement: Exact persisted crosshair inspection

The chart SHALL map a tap or drag crosshair to elapsed time, request the deterministic nearest exact persisted sample, and display its elapsed time and available requested values separately from the downsampled envelope. Approximate overview values MUST NOT be presented as exact. Crosshair state SHALL remain coherent when a newer range response arrives and SHALL disclose unavailable values.

#### Scenario: Inspect an extrema bucket

- **WHEN** the user places the crosshair near an overview bucket containing several raw samples
- **THEN** the displayed detail comes from the backend's nearest exact committed sample rather than the bucket average or an interpolated SVG coordinate

#### Scenario: Exact value unavailable

- **WHEN** the selected exact sample has no value for a displayed field
- **THEN** the chart reports it as unavailable rather than zero

### Requirement: Completed and growing session query compatibility

The session-review data owner SHALL use the metadata snapshot boundaries for every historical request and SHALL be able to replace them with newer boundaries without replacing its querying or rendering model. This change SHALL continue to enter history through the sessions screen; presenting the active session's history from the live screen is deferred.

#### Scenario: Completed session

- **WHEN** the user opens a completed session
- **THEN** all overview, zoom, exact-value, and event queries use its stable committed boundaries

#### Scenario: Future active-session entry point

- **WHEN** a future live-screen feature supplies newer active-session metadata boundaries
- **THEN** the existing historical query and SVG model can refresh the growing ride without a different backend protocol
