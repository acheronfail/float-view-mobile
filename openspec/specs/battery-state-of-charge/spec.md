# Battery State of Charge Specification

## Purpose

Provide a consistent estimated battery state of charge from measured voltage, selected cell curves, and saved pack configuration.

## Requirements

### Requirement: Explicit bounded battery profiles

The system SHALL offer the seven profiles and exact voltage knots recorded in `backends/core/product-store/tests/battery-profile-data.json`, retaining source provenance and catalog version. Estimation SHALL require a selected profile and valid saved series and parallel counts. The display-empty voltage SHALL default to 3.0 V per cell when absent and accept only finite values at least 2.7 V and below 4.2 V. Generic linear SHALL require an explicit manual selection or an explicitly selected named preset that resolves to that profile; it SHALL NOT be inferred from controller identity or matching electrical settings. Known controller/saved series disagreement SHALL make the estimate unavailable. Guests and BMS-only connections SHALL have no board estimate.

#### Scenario: Unconfigured board

- **WHEN** a saved board has no selected cell profile or lacks a valid pack count
- **THEN** its estimate is unavailable with reason `not_configured`, without substituting the controller percentage

#### Scenario: Conflicting series counts

- **WHEN** a saved 20S profile connects to a controller reporting 18S
- **THEN** its estimate is unavailable with reason `series_mismatch`

#### Scenario: Explicit generic profile

- **WHEN** the user explicitly selects generic linear in Advanced with valid pack counts
- **THEN** the system estimates using that profile and identifies the manual generic choice as approximate

#### Scenario: Named preset resolves to generic

- **WHEN** the user explicitly selects a named preset without a dedicated curve
- **THEN** the system estimates using the preset's generic profile and valid pack counts
- **AND** Basic presents the selected battery name without approximation wording, a missing-curve warning, or extra confirmation

### Requirement: Cell and load aware interpolation

The system SHALL derive a voltage observation from cell voltage and signed battery current belonging to the same accepted sample, using the pinned piecewise-linear source curves. Discharge current SHALL be normalized by parallel count, and load interpolation SHALL occur between corresponding original voltage knots before inversion. Above measured load, the highest measured curve SHALL be used without extrapolation. Missing/nonpositive current and single-curve profiles SHALL use the zero-load curve for the observation. The system SHALL track charge on the original curve axis and remap the tracked fraction to the displayed [0,1] interval using the zero-load curve's charge fraction at the configured display-empty voltage. Invalid or nonpositive voltage SHALL yield an unavailable estimate. Rendering SHALL round only the final percentage. Instantaneous observations SHALL NOT replace an initialized temporal estimate directly.

#### Scenario: Measured idle knot

- **WHEN** P45B uses minimum 2.7 V and measures 3.73 V per cell at zero current
- **THEN** its initial estimate is 0.5

#### Scenario: Interpolated load

- **WHEN** a 20S2P P45B pack with minimum 2.7 V first measures 72.1 V and 20 A discharge
- **THEN** its initialized estimate is 0.5 from the 10 A per-cell interpolated curve

#### Scenario: Display empty remapping

- **WHEN** P45B uses minimum 3.23 V and measures 3.775 V per cell at zero current
- **THEN** its initial estimate is 0.5 within floating-point tolerance

#### Scenario: Endpoints and excessive current

- **WHEN** voltage lies at or below configured minimum, or at or above 4.2 V per cell
- **THEN** the voltage observation is bounded, while an already initialized percentage follows temporal tracking rather than snapping to an endpoint
- **AND** excessive current is identified as `load_clamped` for a multi-load profile

#### Scenario: Missing current or regeneration

- **WHEN** voltage is valid but current is absent, zero, or negative
- **THEN** the zero-load curve produces a `voltage_only` observation without retaining prior discharge compensation
- **AND** the temporal estimator handles missing current and regeneration according to its charge-tracking rules

### Requirement: Coherent bounded estimator lifecycle

Estimation SHALL follow accepted sample ordering using constant-size process-owned state. Duplicate/out-of-order samples, snapshot reads, and rejected persistence SHALL NOT advance charge, smoothing, or settling. View recreation SHALL NOT restart or advance state. A same-board/controller reconnect or sample gap exceeding five seconds SHALL retain the last fraction as a provisional seed, clear the current integration pair and settling history, and never account for the unobserved interval. Board/controller identity changes, process restart, or estimation-configuration changes SHALL initialize a new provisional estimate from valid telemetry; unrelated profile edits and preset-identity-only edits with unchanged effective estimation values SHALL NOT reset it. An atomic preset selection SHALL reconfigure estimation only once, from the complete accepted pack values. Changing setup modes SHALL NOT change estimator state. A configuration edit SHALL recompute against the latest accepted sample and accepted profile revision. Nonready connections SHALL retain their matching estimate under existing stale-display behavior. New invalid voltage SHALL produce unavailable rather than present an old fraction as current, and SHALL break integration continuity; valid recovery SHALL resume provisionally without integrating the invalid interval.

#### Scenario: Reconnect and view recreation

- **WHEN** a WebView is recreated while the connection remains ready
- **THEN** it receives the existing estimate without resetting charge tracking or smoothing
- **WHEN** the controller subsequently reconnects
- **THEN** the first accepted sample resumes provisionally from the retained charge without integrating across the disconnect

#### Scenario: Profile change while live

- **WHEN** the selected profile or pack configuration changes successfully
- **THEN** the next authoritative snapshot associates the recomputed estimate with that profile revision and the latest accepted sample

#### Scenario: Repeated sample

- **WHEN** a sample is duplicated or arrives out of order
- **THEN** it does not change the estimate or advance temporal state

#### Scenario: Telemetry gap

- **WHEN** the next accepted sample arrives more than five seconds after the previous one
- **THEN** the estimate resumes provisionally from its last charge, clears settling history, and does not integrate charge across the gap

### Requirement: Temporal charge tracking and settled voltage correction

With configured pack capacity and consecutive finite signed current samples no more than five seconds apart, the system SHALL integrate net battery charge by elapsed time and capacity, decreasing on discharge and increasing on regeneration or observed charging. Charge SHALL remain bounded in [0,1]. Missing current SHALL break integration continuity and select smoothed voltage tracking provisionally; absent capacity SHALL select smoothed voltage tracking without making configuration incomplete. Neither path SHALL infer current across unobserved intervals. Voltage correction during charge integration SHALL require at least 30 seconds of continuous low current and stable voltage, SHALL stop on renewed load, and SHALL change displayed percentage by at most 0.1 percentage points per second. Voltage-only tracking SHALL obey the same rate bound after initialization. Actual integrated charge SHALL NOT be constrained by this voltage-correction bound. Initial estimates and estimates after uncertain intervals SHALL be provisional until settled observations and the tracked percentage converge. Momentary full/empty voltage or a charging flag SHALL NOT force percentage endpoints.

#### Scenario: Heavy-load pulse

- **WHEN** an initialized 9 Ah pack observes constant 60 A discharge for one second without a gap or rest correction
- **THEN** its internal charge decreases by 1/540 before display-empty remapping, regardless of the instantaneous sag-derived percentage

#### Scenario: Voltage-only load step

- **WHEN** capacity is absent and an initialized 80 percent estimate receives a 30 percent voltage observation for one second
- **THEN** the displayed estimate changes by at most 0.1 percentage points and remains identified as voltage-smoothed

#### Scenario: Regeneration and charging

- **WHEN** consecutive valid battery-current observations are negative
- **THEN** charge integration adds the observed charge without treating voltage recovery as additional immediate charge
- **AND** charging indication and measured voltage remain responsive

#### Scenario: Brief throttle release

- **WHEN** current drops near zero for less than 30 seconds after load
- **THEN** charge-integrated estimation does not recalibrate from recovered voltage

#### Scenario: Settled correction

- **WHEN** sufficiently low current and stable voltage persist for at least 30 seconds
- **THEN** the tracked estimate approaches the settled observation gradually within the correction bound
- **AND** provisional status clears only when the tracked and settled displayed percentages differ by at most two percentage points

#### Scenario: Capacity is missing or current becomes unavailable

- **WHEN** capacity is absent or a current sample is missing or non-finite
- **THEN** the estimator uses bounded voltage smoothing, does not assume zero consumption, and does not report charge integration for that update
- **AND** missing current marks the estimate provisional and prevents integration until a new consecutive valid pair exists

#### Scenario: Reserve mapping and endpoints

- **WHEN** display-empty voltage is above the original curve's empty endpoint or very close to 4.2 V
- **THEN** charge consumption is integrated on the original charge axis before reserve remapping, and the displayed result stays finite and bounded
- **AND** voltage correction limits apply to displayed percentage rather than the unremapped fraction

#### Scenario: Nominal preset capacity with a generic observation

- **WHEN** an explicitly selected X7S P30B preset supplies 6 Ah, a generic curve, and consecutive finite battery-current samples
- **THEN** charge integration uses the persisted 6 Ah and settled correction uses the generic zero-load observation without inventing P30B load curves
- **AND** generic profile selection does not make configuration incomplete

#### Scenario: Cleared capacity stays cleared

- **WHEN** capacity is explicitly cleared for a named preset and telemetry resumes after reconnect
- **THEN** the estimate uses voltage smoothing and no catalog read or mode switch restores nominal capacity
