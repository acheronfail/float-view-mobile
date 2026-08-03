# Live Board Dashboard Specification

## Purpose

Define the focused live-board experience, including responsive telemetry, connection and session state, board configuration, and safe disconnect controls.
## Requirements
### Requirement: Focused responsive live dashboard

The live-board view SHALL remain a focused subordinate screen without primary tabs and SHALL present every field in the current live telemetry contract at an intentional stable position in a responsive grid. The grid SHALL render three, two, or one usable columns as available width decreases, SHALL preserve stable cell placement while values update, and SHALL render unavailable values as placeholders.

#### Scenario: First live sample

- **WHEN** a connected board publishes its first persisted live sample
- **THEN** the dashboard replaces placeholders with the sample's speed, eRPM, voltage, duty, currents, temperatures, ride state, board state, footpad state, sequence, and elapsed time in their assigned cells

#### Scenario: Before the first sample

- **WHEN** a connection is active but no persisted sample is available
- **THEN** the complete dashboard structure remains visible with placeholders and without layout movement

#### Scenario: Narrow viewport

- **WHEN** the live view is rendered on a narrow mobile viewport
- **THEN** every reading remains readable without horizontal page scrolling or omission

#### Scenario: Future telemetry field

- **WHEN** the contract gains another live telemetry field
- **THEN** the frontend assigns it an intentional dashboard position rather than rendering it through a catch-all field bucket

### Requirement: Live connection-state presentation

The dashboard SHALL display the active connection's resolved board name, actual connection state, detected controller route, VESC version, Refloat package version, and VESC access mode when known. It SHALL label a direct route as direct and a CAN route with its CAN ID. A test-version controller SHALL be labeled read-only beside the VESC and Refloat versions. Values SHALL render normally only while the connection is `ready`; every other active connection state SHALL retain and dim the latest sample. The view SHALL NOT display per-sample age or private controller identifiers.

#### Scenario: Direct connection

- **WHEN** authoritative state reports a ready direct controller
- **THEN** the live firmware information identifies the controller connection as direct

#### Scenario: CAN connection

- **WHEN** authoritative state reports a ready controller on CAN ID `42`
- **THEN** the live firmware information identifies the connection as CAN ID `42`

#### Scenario: Test firmware access label

- **WHEN** authoritative firmware information reports an audited nonzero test-version marker
- **THEN** the live firmware information shows `Read-only (test firmware)` beside the VESC and Refloat versions

#### Scenario: Transient reconnect

- **WHEN** a ready connection enters `reconnecting`
- **THEN** the status changes to reconnecting and the latest readings remain visible with dimmed presentation

#### Scenario: Reconnect succeeds

- **WHEN** a reconnecting connection returns to `ready`
- **THEN** the status returns to connected and readings return to normal presentation as persisted updates resume

#### Scenario: Unsupported firmware

- **WHEN** firmware identity is known and marked unsupported
- **THEN** the live view presents the stable unsupported-controller or unsupported-Refloat issue, starts no polling or new session, and retains any previously persisted sample in dimmed presentation

#### Scenario: Recoverable connection failure

- **WHEN** authoritative state contains a failed active connection and connection issue
- **THEN** the live view explains that bounded issue, keeps board configuration and disconnect available, and offers retry for the same device

#### Scenario: Retry after configuration

- **WHEN** the user corrects or clears the preferred CAN ID and activates retry
- **THEN** the frontend sends `device.connect` for the same failed `deviceId` once and follows the new authoritative generation

#### Scenario: Incomplete Board Profile

- **WHEN** the connected board's profile is incomplete
- **THEN** profile completeness does not block or add a warning to the live dashboard

### Requirement: Active-session live summary

The live view SHALL present the active session's sample count, riding elapsed time, total elapsed time, distance, maximum speed, maximum duty cycle, state, and integrity from the same authoritative snapshot as the latest sample.

#### Scenario: Persisted sample updates summary

- **WHEN** a sample is successfully persisted
- **THEN** the dashboard receives the resulting latest sample and current active-session aggregates in one newer authoritative snapshot

#### Scenario: Unavailable aggregate

- **WHEN** an optional active-session aggregate has not been established
- **THEN** its assigned cell displays a placeholder rather than a fabricated zero

### Requirement: Extensible live action menu

The live header SHALL provide one accessible menu trigger whose menu contains Configure board and Stop and disconnect actions and can accept additional actions later without changing the focused route model.

#### Scenario: Open menu

- **WHEN** the user activates the live header menu trigger
- **THEN** a keyboard- and pointer-operable menu presents the two current actions and identifies the trigger accessibly

#### Scenario: Saved identity pending

- **WHEN** an active connection has not yet produced a saved-device identity
- **THEN** Configure board is unavailable while connection state and disconnect remain usable

### Requirement: Live board-configuration sheet

Configure board SHALL open a modal bottom sheet that covers almost the entire viewport and renders the same board configuration editor as the routed Boards flow, including the optional preferred CAN ID. The sheet SHALL respect safe areas, contain focus, provide a close control, dismiss from Escape or backdrop activation, and restore focus to its trigger.

#### Scenario: Open live configuration

- **WHEN** the user selects Configure board for an identified active board
- **THEN** the sheet opens over the live dashboard and displays the board's resolved name, editable custom name, read-only platform Bluetooth identifier, optional preferred CAN ID, and remaining Board Profile fields

#### Scenario: Backdrop dismissal after text edit

- **WHEN** backdrop activation blurs a valid edited text field
- **THEN** the resulting save completes before the sheet dismisses and the updated authoritative value is visible after dismissal

#### Scenario: Reconnecting while configuring

- **WHEN** the active connection begins reconnecting while its configuration sheet is open
- **THEN** local board configuration remains usable and the underlying live status reflects reconnecting

### Requirement: Conditional disconnect confirmation

Stop and disconnect SHALL proceed without confirmation only when the latest persisted ride state is exactly `idle`. A non-idle, unknown, or unavailable latest ride state SHALL require explicit confirmation before the disconnect request is sent.

#### Scenario: Idle disconnect

- **WHEN** the latest persisted ride state is `idle` and the user selects Stop and disconnect
- **THEN** the frontend requests disconnect immediately and returns Home after authoritative state removes the active connection

#### Scenario: Riding disconnect

- **WHEN** the latest persisted ride state is not `idle` and the user selects Stop and disconnect
- **THEN** the frontend requests confirmation and disconnects only after the user confirms

#### Scenario: No live sample

- **WHEN** no latest persisted sample is available and the user selects Stop and disconnect
- **THEN** the frontend requests confirmation before disconnecting

### Requirement: Ambiguous controller prompt

The shared live view SHALL present an accessible modal controller-selection prompt whenever authoritative state is `selecting_controller`. It SHALL state how many VESC controllers were found, list every CAN-ID-sorted candidate's CAN ID and available display-safe identifying information, identify unsupported VESC versions, and require an explicit supported choice before sending `device.selectController`. Disconnect MUST remain available.

#### Scenario: Two controllers found

- **WHEN** state contains two current controller candidates
- **THEN** the prompt explains that two VESC controllers were found and renders both CAN IDs with their available hardware/model, firmware name, and VESC version

#### Scenario: Unsupported candidate

- **WHEN** a candidate has `vescSupported` false
- **THEN** the prompt keeps it visible with an unsupported explanation and disables its selection control

#### Scenario: Candidate activated

- **WHEN** the user activates one candidate with a primary pointer or keyboard
- **THEN** the frontend sends one selection request, prevents duplicate activation while pending, and follows subsequent authoritative state

### Requirement: Detailed live board telemetry

The shared live dashboard SHALL present available detailed telemetry in coherent groups for motion, electrical load, temperatures, rider input, control contributions, and accumulated values. It SHALL clearly distinguish measured, requested, directional, filtered, booster, and field-weakening currents in amperes and duty as a ratio. It SHALL NOT derive or display torque or current utilization. Missing values SHALL display as unavailable rather than zero.

#### Scenario: Detailed dynamic session

- **WHEN** a Refloat 1.2.3 session supplies full dynamic telemetry
- **THEN** the dashboard can display its IMU, footpad, current, ATR, tilt, remote, and core motor values without platform branching

#### Scenario: Legacy unavailable value

- **WHEN** the selected legacy detail level does not provide a field shown by the dashboard
- **THEN** that field is hidden or marked unavailable and is not displayed as zero

### Requirement: Live semantic board states

The live experience SHALL surface package state and mode, footpad state, wheelslip, darkride, charging, stop condition, setpoint adjustment or pushback type, alert reason, and bounded firmware fault with accessible text and non-color-only distinctions.

#### Scenario: Duty pushback

- **WHEN** telemetry reports running state, duty pushback, and a duty alert reason
- **THEN** the live view identifies duty pushback distinctly from high-voltage, low-voltage, temperature, speed, and firmware-fault states

#### Scenario: Wheelslip and footpad

- **WHEN** wheelslip is active and only one footpad side is engaged
- **THEN** both states are conveyed accessibly and neither is inferred solely from current or speed

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
