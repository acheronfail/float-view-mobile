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

The live view SHALL present the active saved session's sample count, riding elapsed time, total elapsed time, distance, maximum speed, maximum duty cycle, state, and integrity from the same authoritative snapshot as the latest persisted sample. When the connection is in guest mode and no active session exists, the live view SHALL present live telemetry and a clear temporary/no-history notice instead of session aggregates.

#### Scenario: Persisted sample updates summary

- **WHEN** a saved-mode sample is successfully persisted
- **THEN** the dashboard receives the resulting latest sample and current active-session aggregates in one newer authoritative snapshot

#### Scenario: Guest sample updates live view

- **WHEN** a guest sample is published for live display
- **THEN** the dashboard updates its telemetry, may show the current-ride `distanceM` value when Refloat supplies it, shows that the connection is not being saved or tracked, and renders no fabricated session aggregate or lifetime odometer value

#### Scenario: Unavailable aggregate

- **WHEN** an optional saved-session aggregate has not been established
- **THEN** its assigned cell displays a placeholder rather than a fabricated zero

### Requirement: Extensible live action menu

The live header SHALL provide one accessible menu trigger whose menu contains the current view switch, stateful Refloat package and flywheel actions, lighting actions only when lighting capability is present, Clear fatal error only when a Refloat 1.2.x fatal latch remains after its active firmware fault has ended, Shut down controller when the VESC core profile supports the command, Configure board when saved identity exists, and Stop and disconnect. Each Refloat item SHALL display its last confirmed on/off state, SHALL be disabled unless its native preconditions are currently satisfied, and SHALL remain unavailable while its action is pending. Shutdown SHALL be unavailable unless authoritative controller-action state permits it. While authoritative state reports flywheel enabled, a non-dismissible full-application modal overlay SHALL replace access to normal application interaction and SHALL expose exactly one action, Disable, which requests the safety-gated flywheel-off transition. Disabling Refloat, enabling flywheel, and shutting down the controller SHALL require explicit contextual confirmation before the backend request is sent. Every menu, confirmation, and flywheel-overlay control SHALL retain native button semantics, use primary Pointer Events with cancellation and a guarded keyboard/assistive click path, avoid double activation, and manage focus within the modal surface.

#### Scenario: Open menu

- **WHEN** the user activates the live header menu trigger on a ready writable connection
- **THEN** a keyboard- and pointer-operable menu presents the applicable Refloat controls with confirmed state alongside the current view, configuration, and disconnect actions

#### Scenario: Saved identity pending

- **WHEN** an active connection has not yet produced a saved-device identity
- **THEN** Configure board is unavailable while applicable live-only Refloat controls, connection state, and disconnect remain usable

#### Scenario: Lighting unavailable

- **WHEN** Refloat INFO reports no lighting capability
- **THEN** LED and headlight actions are absent and no lighting request is issued by the frontend

#### Scenario: Read-only or reconnecting connection

- **WHEN** firmware is read-only or the active connection is not ready
- **THEN** Refloat mutation actions are visibly unavailable while current retained state may remain displayed and disconnect remains usable

#### Scenario: Confirm package disablement

- **WHEN** the user chooses to disable an idle enabled Refloat package
- **THEN** an explicit warning explains that balancing will be disabled and sends the request only after confirmation

#### Scenario: Confirm flywheel engagement

- **WHEN** the user chooses to enable flywheel from an eligible idle normal state
- **THEN** an explicit warning requires the board to be disengaged, nose-up, and balanced before confirmation sends the request

#### Scenario: Flywheel mode actions

- **WHEN** authoritative state reports active flywheel mode
- **THEN** a non-dismissible modal overlay blocks the normal application, explains that the board must be disengaged, and presents a single **Disable** button that can issue only the flywheel-disable request

#### Scenario: Flywheel disable remains pending or fails

- **WHEN** Disable is activated while its mutation is pending or the exact command cannot be transmitted or confirmed
- **THEN** no other application action becomes available and the overlay remains until fresh authoritative telemetry reports flywheel disabled

#### Scenario: Clearable fatal latch

- **WHEN** authoritative state reports a latched Refloat fatal error and no active firmware fault
- **THEN** the menu presents Clear fatal error and removes it after the confirmed clear publishes newer state

#### Scenario: Primary and keyboard activation

- **WHEN** a Refloat action is activated by primary pointer or keyboard/assistive click
- **THEN** exactly one backend request is issued, while non-primary activation and the guarded synthetic click do not double activate it

#### Scenario: Confirm controller shutdown

- **WHEN** the user chooses Shut down controller on an eligible idle connection
- **THEN** a modal warns that not every controller supports software shutdown, that unsupported hardware may remain on, and that the board must be stationary before **Shut down** sends exactly one request while **Cancel** sends none

#### Scenario: Shutdown transmission is not power confirmation

- **WHEN** the backend accepts a shutdown command but the connection remains available
- **THEN** the frontend does not claim that power-off succeeded and continues to render the authoritative connection after pending intent clears

#### Scenario: Shutdown causes transport loss

- **WHEN** an accepted shutdown powers off the controller and its attached transport
- **THEN** the live view exits through the normal user-initiated disconnect presentation without showing an automatic-reconnect failure

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

Session review SHALL expose metadata first, then the same supported detailed fields through bounded authenticated columnar series and sample-window requests, plus observed semantic transitions through chronological event pages. It SHALL preserve nullability and units, allow inspection of fixture-generated scenes after disconnect, cap each data request to 16 fields/1,024 buckets/65,536 observations, cap sample pages at 2,000 rows and event pages at 500 rows, and disclose retained/pruned history metadata. The review SHALL compose compatible fields into metadata-defined visual chart groups without changing the independent field columns returned by the data service. Each chart group SHALL use one y-axis: either an explicitly defined fixed domain or a deterministic domain derived from all finite values in the group, including lines currently hidden from view. Display formatting SHALL be consistent between axis labels and tooltip values; bounded ratio fields SHALL be able to show their semantic percentage range without changing transported values. A multi-field group SHALL render one line per field and a legend below the plot. During inspection, each chart group SHALL render one tooltip at the shared elapsed position, with one newline-separated, field-colored value line for each visible field and a marker at that field’s corresponding y-position.

#### Scenario: Review field weakening

- **WHEN** a session includes field-weakening current and high duty
- **THEN** historical detail shows both raw series in their contract units, with each field assigned to its metadata-defined chart group

#### Scenario: Review bounded duty cycle

- **WHEN** a session contains duty-cycle ratios within the normal bounded range
- **THEN** the duty-cycle chart always includes `0%` and `100%` as its axis endpoints and tooltip values use the same percentage formatting

#### Scenario: Compare temperatures

- **WHEN** a session contains motor temperature and controller temperature
- **THEN** both fields appear as separate colored lines in one temperature chart with one shared °C axis, a legend underneath, and one consolidated tooltip containing separate colored lines for both values

#### Scenario: Hidden line does not change the shared scale

- **WHEN** one line in a multi-field chart is hidden through its legend
- **THEN** the remaining line is omitted from the SVG while the chart domain, ticks, viewport, series requests, and other line states remain stable

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

#### Scenario: Hide every line

- **WHEN** the user hides every line in a multi-field chart
- **THEN** the chart and legend remain available, no line is rendered, and any legend item can restore its line without changing the shared viewport

#### Scenario: Consolidated multi-line tooltip

- **WHEN** inspection is active on a chart with multiple visible lines
- **THEN** exactly one tooltip is rendered for that chart, its value lines are separated vertically and colored to match their line tokens, and each value has a marker at the same elapsed x-position

### Requirement: Progressive responsive SVG chart groups

One route-scoped session controller SHALL own metadata, history revision, captured gap policy, the shared elapsed viewport, request generations, bounded cache, and independent loading/error state for series groups and events. After measuring a positive width it SHALL load the first visible stable group, use one reusable intersection boundary to prefetch nearby stable or extension groups for the current viewport, and abort or ignore obsolete work on resize, snapshot or revision replacement, gap-policy change, navigation, or frontend recreation. Stable groups SHALL contain at most nine fields and negotiated extensions SHALL be chunked into groups of at most eight. Cache ownership SHALL be bounded by both entry count and decoded bytes.

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

### Requirement: Synchronous crosshair tooltip presentation

Crosshair position SHALL map to elapsed time, and each loaded chart group SHALL derive its tooltip values synchronously from the loaded series at that position. One shared crosshair SHALL synchronize elapsed position across the charts without issuing an exact-value lookup request. Tooltips SHALL show the field label, formatted value, and unit, remain within the visible viewport, and not intercept plot pointers. Pointer leave SHALL dismiss the inspection, and newer series geometry SHALL keep the crosshair and tooltip position coherent.

#### Scenario: Shared chart inspection

- **WHEN** pointer or touch inspection activates an elapsed position
- **THEN** every chart with loaded data renders its own tooltip from the corresponding loaded series values at the shared position

#### Scenario: Tooltip near an edge

- **WHEN** inspection occurs near a chart edge
- **THEN** the tooltip repositions within the visible chart viewport without obscuring or capturing the plot pointer

### Requirement: Range-bound historical transition pages

The session feature SHALL request chronological transition pages for the current history revision, snapshot, and visible elapsed range, preserve deterministic order without duplicate rows, expose explicit continuation controls, and discard the cursor/page collection when the revision, snapshot, or range changes. It SHALL never automatically accumulate all session transitions in memory.

#### Scenario: Continue events

- **WHEN** more than 500 transitions exist in the visible range and the user requests continuation
- **THEN** the next validated page appends in chronological order without duplicates or loss

#### Scenario: Viewport changes during paging

- **WHEN** the user pans or zooms after one or more event pages loaded
- **THEN** old pages and cursors cannot populate the new range and the first new-range page becomes authoritative

### Requirement: Completed and growing session query compatibility

The route-scoped controller SHALL use one published metadata revision and snapshot for every historical request and SHALL be able to replace either with newer metadata without replacing its querying, caching, rendering, tooltip, or event model. A history-revision change SHALL abort and discard every older cache entry, cursor, and series response before refreshing. This change SHALL continue to enter history through the sessions screen; presenting active-session history from the live screen and presenting ride-prune controls are deferred.

#### Scenario: Completed session

- **WHEN** the user opens a completed session
- **THEN** overview, zoom, retained-window, and event queries use its stable final revision and boundary and disclose whether the ride was pruned

#### Scenario: Future active-session entry point

- **WHEN** a future live-screen feature supplies newer active-session metadata
- **THEN** the existing historical controller can refresh the growing ride through the same protocol and rendering model

#### Scenario: Ride pruned outside the route

- **WHEN** refreshed metadata reports a newer history revision
- **THEN** the controller rejects every stale result and reconstructs the viewport from retained history without claiming that removed samples remain exact

### Requirement: Configurable historical continuity threshold

The shared settings UI SHALL expose an optional history-gap threshold stored in seconds. It SHALL accept only finite values from 0.1 through 3,600, SHALL allow the value to be cleared to automatic mode, and SHALL explain that automatic mode uses ride phase, persistence cadence, and retained pruning resolution. A settings revision SHALL invalidate series geometry whose cache key captured another gap policy without invalidating retained sample rows solely because continuity presentation changed.

#### Scenario: Set explicit threshold

- **WHEN** a user saves a valid threshold in seconds
- **THEN** subsequent chart requests and cache keys capture that value and gaps appear wherever an elapsed delta exceeds it

#### Scenario: Restore automatic gaps

- **WHEN** the user clears the configured value
- **THEN** subsequent charts use the phase- and resolution-aware derived default and explain that automatic mode is active

### Requirement: Live location option and warning

The live dashboard SHALL present a small non-blocking location option or warning whenever recording is disabled, permission is unresolved/denied, or location is unavailable while a board is connected. The warning SHALL not obscure board telemetry or prevent disconnect/configuration actions.

#### Scenario: Recording disabled

- **WHEN** a board is connected and `locationRecordingEnabled` is false
- **THEN** the live view offers the location-recording option without claiming that permission is missing

#### Scenario: Permission required

- **WHEN** recording is enabled but location permission is unresolved or denied
- **THEN** the live view shows a compact warning that identifies location recording as unavailable and provides the permission action

#### Scenario: Board-only fallback

- **WHEN** the user leaves location disabled or declines permission
- **THEN** the live dashboard continues presenting current board telemetry and session summaries with no map requirement

### Requirement: Re-prompt or settings activation

Activating the live location option/warning SHALL invoke the shared contract permission action. The frontend SHALL not perform platform detection; the backend SHALL choose an OS re-prompt when possible or platform settings navigation otherwise.

#### Scenario: Re-prompt available

- **WHEN** the user activates the warning and the OS allows another permission prompt
- **THEN** the native backend presents the prompt and publishes the resulting status

#### Scenario: Settings required

- **WHEN** the user activates the warning after a permanent denial or restricted state
- **THEN** the native backend presents the platform settings destination or reports a bounded unavailable error while board logging remains active

### Requirement: Location status recovery

The live dashboard SHALL derive its warning and option from complete authoritative state and SHALL recover it after a frontend recreation without storing permission state only in the WebView.

#### Scenario: Recreated live view

- **WHEN** the live WebView reconnects during an active session
- **THEN** it renders the current location status/action availability from `state.get` and does not require a new board connection

### Requirement: Conditional BMS menu action

The shared live-board menu SHALL contain one `BMS` action after at least one VBMS identity has been detected in the current connection generation and SHALL omit it while no BMS identity is known or status is `not_found`. The same frontend behavior SHALL apply on desktop and touch clients without platform branching.

#### Scenario: BMS becomes detected

- **WHEN** auxiliary discovery identifies a direct or CAN-routed VBMS
- **THEN** the next authoritative state causes the live menu to expose the BMS action

#### Scenario: No BMS detected

- **WHEN** auxiliary discovery is incomplete without a candidate or completes as `not_found`
- **THEN** the live menu contains no BMS action and controller telemetry remains unchanged

#### Scenario: Ambiguous or unavailable BMS

- **WHEN** detected BMS state is `ambiguous` or `unavailable`
- **THEN** the BMS action remains available so the user can inspect status and corrective guidance

### Requirement: Responsive BMS sheet presentation

Activating the BMS menu action SHALL open the shared responsive sheet/popover and render only authoritative BMS state. The presentation SHALL show route, status, observation freshness, summary voltage/current/state-of-charge/state-of-health values, cell voltages and balancing, sensor and device temperatures, humidity and pressure, session and lifetime counters, and optional data-version and status values, using explicit unavailable placeholders and the configured unit system where conversion applies.

#### Scenario: Attached BMS values

- **WHEN** the BMS sheet opens with an attached complete snapshot
- **THEN** every contract field has an intentional stable label and placement and bounded cell and temperature rows are rendered in source order

#### Scenario: Stale retained values

- **WHEN** status is unavailable and a prior snapshot remains
- **THEN** the sheet visibly marks the values stale and shows their age rather than presenting them as current

#### Scenario: Multiple BMS candidates

- **WHEN** status is ambiguous
- **THEN** the sheet lists bounded CAN-ID-sorted display-safe candidates, polls none, and directs the user to set `preferredBmsCanId` in Board Profile

#### Scenario: Frontend recreation with open route

- **WHEN** the WebView is recreated and navigates back to the live board while detected BMS state remains authoritative
- **THEN** the menu and newly opened sheet reconstruct without requesting historical or platform-specific BMS data

### Requirement: Accessible BMS interactions

The BMS menu action and sheet SHALL retain native button and dialog semantics, use primary Pointer Events with cancellation and a guarded keyboard or assistive-technology click path, prevent double activation, apply an appropriate `touch-action`, contain and restore focus, and keep dismissal usable on desktop and touch clients.

#### Scenario: Primary pointer activation

- **WHEN** a primary pointer activates the enabled BMS action in a frequently updating live view
- **THEN** exactly one sheet opens and the synthesized click does not activate it again

#### Scenario: Non-primary pointer activation

- **WHEN** a non-primary pointer activates the BMS action
- **THEN** no sheet opens

#### Scenario: Keyboard activation and dismissal

- **WHEN** a focused BMS action is activated by keyboard and the sheet is later dismissed
- **THEN** it opens exactly once, exposes meaningful dialog labeling, and restores focus to the invoking action

### Requirement: Dedicated standalone BMS page

The shared frontend SHALL route an active connection with authoritative `deviceKind: bms` to a dedicated full-page BMS experience that reuses the same BMS content component as the live-board sheet. The page SHALL present authoritative status, freshness, values, unavailable state, and units, expose disconnect, and omit board views, Refloat controls, controller shutdown, Board configuration, ride state, and session controls.

#### Scenario: Standalone BMS becomes classified

- **WHEN** an accepted asynchronous connection publishes `deviceKind: bms`
- **THEN** the frontend replace-navigates to the BMS page and renders the current authoritative BMS state

#### Scenario: Board connection remains live

- **WHEN** the active connection publishes `deviceKind: board`
- **THEN** the frontend uses the existing live-board page and retains the BMS sheet for any auxiliary BMS

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated while a BMS-only connection remains active
- **THEN** bootstrap state routes directly to the BMS page without restarting native discovery or polling

#### Scenario: BMS disconnect

- **WHEN** the user activates disconnect from the standalone BMS page
- **THEN** the shared device disconnect method runs once and successful authoritative removal returns navigation to Home

#### Scenario: Route mismatch

- **WHEN** the BMS page observes no active connection or a board connection, or the live page observes a BMS connection
- **THEN** it replace-navigates to Home or the authoritative connection-kind route without exposing incompatible controls
