## MODIFIED Requirements

### Requirement: Detailed historical telemetry review

Session metadata SHALL appear before history and the shared session feature SHALL progressively render the same supported detailed numeric fields through responsive SVG charts whose observations and DOM geometry are bounded by requested fields and current horizontal CSS-pixel resolution rather than raw sample count. Charts SHALL preserve stable names, units, real elapsed spacing, finite extrema, deterministic axes, and configured or derived continuity gaps. Transition events SHALL be inspectable through chronological range pages alongside the shared viewport. The frontend MUST NOT load or retain the complete lossless ride, silently truncate history, interpolate exact values from an envelope, or branch by native platform.

#### Scenario: Review field weakening

- **WHEN** a session includes field-weakening current and high duty
- **THEN** historical review shows both series independently in their contract units with their visible-range extrema preserved

#### Scenario: Review stop event

- **WHEN** a session transitions through a stop condition inside the visible elapsed range
- **THEN** chronological event paging makes the transition inspectable alongside surrounding telemetry without embedding every session event

#### Scenario: Long complete-catalog session

- **WHEN** a user opens a 150,000-sample session containing every stable numeric field and bounded extensions
- **THEN** metadata and a useful first visible SVG group appear within the mobile target without creating one frontend object or DOM node per raw sample

#### Scenario: Missing field interval

- **WHEN** the backend reports separate continuity segments around unavailable telemetry
- **THEN** the SVG leaves a visible gap and does not connect, average, or substitute the missing values

### Requirement: Detailed telemetry interactions

Historical chart controls and inspection SHALL use native control semantics and Pointer Events with explicit gesture arbitration, capture, cancellation, and `touch-action: pan-y`. A tap SHALL select a crosshair; a one-pointer background drag beyond the movement threshold SHALL select an elapsed range for zoom; the selection SHALL show start/end vertical guides and opaque outside regions; and two active touch pointers regardless of `isPrimary` SHALL pinch around their midpoint. Mouse hover, hovering pen, touch or contact pen selection, keyboard operation, and assistive technology SHALL reach the same exact retained-source tooltip model. Guarded pointer/click activation SHALL prevent double actions, and vertical page scrolling SHALL remain usable.

#### Scenario: Mouse hover inspection

- **WHEN** a mouse or hovering pen moves across a chart
- **THEN** a bounded crosshair tooltip tracks elapsed position and reports the nearest exact retained source value when it arrives without presenting envelope geometry as exact

#### Scenario: Touch inspection

- **WHEN** a primary touch or contact pen taps the plot or drags the existing crosshair handle
- **THEN** the same tooltip is pinned or updated with exact-value, unit, elapsed-time, pending, and unavailable semantics suitable for a client without hover

#### Scenario: Two-touch pinch

- **WHEN** a second active touch joins the chart gesture even though it is not the primary pointer
- **THEN** the viewport pinches around the two-pointer midpoint and neither contact is misclassified as an invalid one-pointer activation

#### Scenario: Drag range selection

- **WHEN** a primary pointer drags across a chart background beyond the minimum selection distance
- **THEN** vertical guides follow the selection endpoints, the outside regions are visibly opaque, and release zooms every chart to the selected elapsed range

#### Scenario: Non-primary one-pointer activation

- **WHEN** a non-primary pointer attempts to initiate a one-pointer selection or control activation
- **THEN** no unintended action is committed

#### Scenario: Cancelled gesture

- **WHEN** a pointer is cancelled, capture is lost, the browser takes a vertical scroll, or the route is disposed
- **THEN** transient gesture state and requests are cleared without stale selection or range mutation

#### Scenario: Keyboard and assistive inspection

- **WHEN** a user operates visible zoom/pan/reset controls or the native/equivalent crosshair slider with keyboard or assistive technology
- **THEN** the same bounded viewport and exact-tooltip capabilities are exposed with meaningful value text and no double activation

## ADDED Requirements

### Requirement: Progressive responsive SVG chart groups

One route-scoped session controller SHALL own metadata, history revision, captured gap policy, the shared elapsed viewport, request generations, bounded cache, and independent loading/error state for series groups, exact inspection, and events. After measuring a positive width it SHALL load the first visible stable group, use one reusable intersection boundary to prefetch nearby stable or extension groups for the current viewport, and abort or ignore obsolete work on resize, snapshot or revision replacement, gap-policy change, navigation, or frontend recreation. Stable groups SHALL contain at most nine fields and negotiated extensions SHALL be chunked into groups of at most eight. Cache ownership SHALL be bounded by both entry count and decoded bytes.

#### Scenario: Initial visible group

- **WHEN** metadata is available and the first chart group has a positive measured width
- **THEN** that group is requested before offscreen groups and produces a useful overview without waiting for the complete catalog

#### Scenario: Scroll to a later group

- **WHEN** a later group approaches the viewport after zoom or pan changed the shared range
- **THEN** the reusable loader requests that group for the newest snapshot, range, and measured resolution rather than the original overview

#### Scenario: Responsive resize

- **WHEN** the chart width changes repeatedly during layout or orientation changes
- **THEN** integer CSS-pixel bucket targets are coalesced and geometry converges on the latest width without fetching lossless history or duplicating equivalent requests

#### Scenario: Frontend recreation

- **WHEN** the WebView or route controller is replaced
- **THEN** native logging continues, obsolete requests/cache are released, and a new controller reconstructs metadata and visible history from the backend

### Requirement: Shared bounded historical viewport

All loaded charts SHALL share one inclusive elapsed viewport bounded by the published session range with a minimum one-millisecond span. Visible native buttons SHALL provide zoom in/out, pan earlier/later, and full-duration reset. During a pan or pinch, current bounded paths SHALL provide animation-frame transform/clip feedback without a series request per pointer move; after input settles, the controller SHALL abort the prior generation, request refined current-resolution geometry, and render only a response matching the newest session, history revision, snapshot, gap policy, fields, range, buckets, and generation. An uncached settled pan or zoom SHALL target 250 milliseconds on the iPhone 12 and Samsung Galaxy S9 reference devices; exceeding the target SHALL NOT turn a bounded cancellable operation into a correctness failure.

#### Scenario: Pinch into a short incident

- **WHEN** two touch contacts narrow a four-hour overview to a short elapsed range
- **THEN** the current SVG responds during the gesture and the settled chart replaces it with extrema-preserving range data within the target

#### Scenario: Range zoom

- **WHEN** a background drag selects an elapsed range near either session edge
- **THEN** the selected range remains clamped to the published session bounds, superseded work is cancelled, and the newest matching response wins

#### Scenario: Full-range reset

- **WHEN** reset is activated by pointer, keyboard, or assistive technology
- **THEN** all chart groups return to the published complete elapsed range through the same bounded request model

### Requirement: Exact crosshair tooltip presentation

Crosshair position SHALL map to elapsed time and exact inspection SHALL request the deterministic nearest retained source sample for the applicable loaded field group. At most one field SHALL be active for tooltip presentation at a time while one shared crosshair synchronizes elapsed position across the group. At most one nearest lookup per group SHALL be in flight with one coalesced latest target; the exact retained row SHALL populate the bounded group cache without presenting multiple competing tooltips. The tooltip SHALL distinguish loading from exact data, show elapsed time, stable label, exact value and unit or unavailable state, remain within the visible viewport, and not intercept plot pointers. Pointer leave SHALL dismiss unpinned hover; touch/pen selection SHALL remain pinned until replacement, explicit close, Escape, or route disposal. Crosshair state SHALL remain coherent when newer series geometry arrives.

#### Scenario: Inspect an extrema bucket

- **WHEN** the crosshair targets a bucket containing multiple raw values
- **THEN** tooltip content comes from the backend's nearest exact retained source row and not the bucket extrema, an average, or an interpolated SVG coordinate

#### Scenario: Shared group crosshair

- **WHEN** pointer, touch, or keyboard inspection activates one field in a multi-field group
- **THEN** one tooltip presents that field while every chart in the group reflects the same elapsed crosshair and may reuse the returned exact row

#### Scenario: Pending exact lookup

- **WHEN** pointer or slider movement advances while an older exact request remains in flight
- **THEN** the visual crosshair moves immediately, the tooltip identifies pending exact data, and only the coalesced newest target may replace its content

#### Scenario: Exact value unavailable

- **WHEN** the selected exact row has no value for a displayed field
- **THEN** the tooltip reports unavailable rather than zero, a neighboring value, or an envelope estimate

#### Scenario: Tooltip near an edge

- **WHEN** inspection occurs near a chart or viewport edge on a narrow touch client
- **THEN** the tooltip repositions within the visible viewport without obscuring or capturing the active crosshair handle

### Requirement: Range-bound historical transition pages

The session feature SHALL request chronological transition pages for the current history revision, snapshot, and visible elapsed range, preserve deterministic order without duplicate rows, expose explicit continuation controls, and discard the cursor/page collection when the revision, snapshot, or range changes. It SHALL never automatically accumulate all session transitions in memory.

#### Scenario: Continue events

- **WHEN** more than 500 transitions exist in the visible range and the user requests continuation
- **THEN** the next validated page appends in chronological order without duplicates or loss

#### Scenario: Viewport changes during paging

- **WHEN** the user pans or zooms after one or more event pages loaded
- **THEN** old pages and cursors cannot populate the new range and the first new-range page becomes authoritative

### Requirement: Completed and growing session query compatibility

The route-scoped controller SHALL use one published metadata revision and snapshot for every historical request and SHALL be able to replace either with newer metadata without replacing its querying, caching, rendering, tooltip, or event model. A history-revision change SHALL abort and discard every older cache entry, cursor, exact request, and series response before refreshing. This change SHALL continue to enter history through the sessions screen; presenting active-session history from the live screen and presenting ride-prune controls are deferred.

#### Scenario: Completed session

- **WHEN** the user opens a completed session
- **THEN** overview, zoom, exact-value, retained-window, and event queries use its stable final revision and boundary and disclose whether the ride was pruned

#### Scenario: Future active-session entry point

- **WHEN** a future live-screen feature supplies newer active-session metadata
- **THEN** the existing historical controller can refresh the growing ride through the same protocol and rendering model

#### Scenario: Ride pruned outside the route

- **WHEN** refreshed metadata reports a newer history revision
- **THEN** the controller rejects every stale result and reconstructs the viewport from retained history without claiming that removed samples remain exact

### Requirement: Configurable historical continuity threshold

The shared settings UI SHALL expose an optional history-gap threshold stored in seconds. It SHALL accept only finite values from 0.1 through 3,600, SHALL allow the value to be cleared to automatic mode, and SHALL explain that automatic mode uses ride phase, persistence cadence, and retained pruning resolution. A settings revision SHALL invalidate series geometry whose cache key captured another gap policy without invalidating exact retained rows solely because continuity presentation changed.

#### Scenario: Set explicit threshold

- **WHEN** a user saves a valid threshold in seconds
- **THEN** subsequent chart requests and cache keys capture that value and gaps appear wherever an elapsed delta exceeds it

#### Scenario: Restore automatic gaps

- **WHEN** the user clears the configured value
- **THEN** subsequent charts use the phase- and resolution-aware derived default and explain that automatic mode is active
