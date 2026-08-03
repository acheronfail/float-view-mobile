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

Session review SHALL expose metadata first, then the same supported detailed fields through bounded authenticated columnar series/exact-sample requests, plus observed semantic transitions through chronological event pages. It SHALL preserve nullability and units, allow inspection of fixture-generated scenes after disconnect, cap each data request to 16 fields/1,024 buckets/65,536 observations, cap sample pages at 2,000 rows and event pages at 500 rows, and disclose retained/pruned history metadata. The review SHALL compose compatible fields into metadata-defined visual chart groups without changing the independent field columns returned by the data service. Each chart group SHALL use one y-axis: either an explicitly defined fixed domain or a deterministic domain derived from all finite values in the group, including lines currently hidden from view. Display formatting SHALL be consistent between axis labels and exact values; bounded ratio fields SHALL be able to show their semantic percentage range without changing transported values. A multi-field group SHALL render one line per field and a legend below the plot. During inspection, each chart group SHALL render one tooltip at the shared elapsed position, with one newline-separated, field-colored value line for each visible field and a marker at that field’s corresponding y-position.

#### Scenario: Review field weakening

- **WHEN** a session includes field-weakening current and high duty
- **THEN** historical detail shows both raw series in their contract units, with each field assigned to its metadata-defined chart group

#### Scenario: Review bounded duty cycle

- **WHEN** a session contains duty-cycle ratios within the normal bounded range
- **THEN** the duty-cycle chart always includes `0%` and `100%` as its axis endpoints and exact values use the same percentage formatting

#### Scenario: Compare temperatures

- **WHEN** a session contains motor temperature and controller temperature
- **THEN** both fields appear as separate colored lines in one temperature chart with one shared °C axis, a legend underneath, and one consolidated tooltip containing separate colored lines for both values

#### Scenario: Hidden line does not change the shared scale

- **WHEN** one line in a multi-field chart is hidden through its legend
- **THEN** the remaining line is omitted from the SVG while the chart domain, ticks, viewport, exact-sample requests, and other line states remain stable

#### Scenario: Missing or unavailable field

- **WHEN** a requested group column is absent, null, or split by a continuity gap
- **THEN** the affected line remains unavailable or broken without being replaced by zero, connected across the gap, or allowed to block other loaded lines in the group

#### Scenario: Recreated session route

- **WHEN** the session route or frontend controller is recreated
- **THEN** metadata-defined grouping and axis policies are rebuilt, all lines start visible, and no prior route’s hidden-line state or stale geometry is applied

### Requirement: Detailed telemetry interactions

Any interactive telemetry disclosure, group selector, chart control, session annotation control, or historical chart legend SHALL use native semantics, appropriate `touch-action`, primary Pointer Events with cancellation, and a guarded keyboard and assistive-technology click path without double activation. A chart legend item SHALL expose whether its associated line is visible, SHALL toggle only that line, and SHALL remain operable on desktop and touch clients. Hidden legend items SHALL be visibly greyed and struck through and SHALL also expose an equivalent non-color state. Legend state SHALL not change historical request bounds, viewport state, or backend data.

#### Scenario: Touch disclosure

- **WHEN** a primary touch pointer opens a detailed telemetry group in a frequently updating live view
- **THEN** it opens exactly once and remains operable while samples continue updating

#### Scenario: Non-primary activation

- **WHEN** a non-primary pointer or cancelled gesture targets a detailed telemetry control
- **THEN** no action is performed

#### Scenario: Prompt recovered

- **WHEN** the frontend is recreated while selection remains pending
- **THEN** the prompt is reconstructed from `state.get` without starting another connection or discovery request

#### Scenario: User disconnects instead

- **WHEN** the user chooses to disconnect while the prompt is visible
- **THEN** the frontend requests disconnection and closes the prompt when authoritative state removes the active connection

#### Scenario: Show or hide one chart line

- **WHEN** the user activates a legend item with a primary pointer, touch, keyboard, or assistive technology
- **THEN** the associated line toggles exactly once, the legend exposes the new pressed/visibility state, and all other lines remain unchanged

#### Scenario: Hidden legend item remains discoverable

- **WHEN** a line is hidden
- **THEN** its legend label remains present, greyed, and struck through, with accessible text indicating that activation will show the line again

#### Scenario: Hide the active inspection field

- **WHEN** the user hides the field currently driving exact tooltip presentation
- **THEN** inspection moves to a visible sibling in the same chart group or becomes unavailable without issuing a stale request for the hidden field

#### Scenario: Hide every line

- **WHEN** the user hides every line in a multi-field chart
- **THEN** the chart and legend remain available, no line is rendered, and any legend item can restore its line without changing the shared viewport

#### Scenario: Consolidated multi-line tooltip

- **WHEN** inspection is active on a chart with multiple visible lines
- **THEN** exactly one tooltip is rendered for that chart, its value lines are separated vertically and colored to match their line tokens, and each value has a marker at the same elapsed x-position

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
