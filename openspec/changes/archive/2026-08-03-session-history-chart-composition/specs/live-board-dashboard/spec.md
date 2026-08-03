## MODIFIED Requirements

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
