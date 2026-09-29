# Spec Delta

## Purpose

Provide a consistent estimated battery state of charge from measured voltage, selected cell curves, and saved pack configuration.

## ADDED Requirements

### Requirement: Explicit bounded battery profiles

The system SHALL offer the seven profiles and exact voltage knots recorded in this change's `profile-data.json`, retaining source provenance and catalog version. Estimation SHALL require a selected profile and valid saved series and parallel counts. The display-empty voltage SHALL default to 3.0 V per cell when absent and accept only finite values at least 2.7 V and below 4.2 V. Generic linear SHALL require explicit selection. Known controller/saved series disagreement SHALL make the estimate unavailable. Guests and BMS-only connections SHALL have no board estimate.

#### Scenario: Unconfigured board
- **WHEN** a saved board has no selected cell profile or lacks a valid pack count
- **THEN** its estimate is unavailable with reason `not_configured`, without substituting the controller percentage

#### Scenario: Conflicting series counts
- **WHEN** a saved 20S profile connects to a controller reporting 18S
- **THEN** its estimate is unavailable with reason `series_mismatch`

#### Scenario: Explicit generic profile
- **WHEN** the user selects generic linear with valid pack counts
- **THEN** the system estimates using that profile and identifies it as approximate

### Requirement: Cell and load aware interpolation

The system SHALL calculate a finite fraction in [0,1] using cell voltage, piecewise-linear source curves, and configured minimum remapping. It SHALL remap each load curve to zero at the selected minimum and full at 4.2 V, interpolate corresponding voltage knots between measured load levels, then invert the resulting curve. Discharge current SHALL be normalized by parallel count. Current above measured load SHALL use the highest measured curve without extrapolation. Missing/nonpositive current and single-curve profiles SHALL use the zero-load curve. Invalid or nonpositive voltage SHALL yield an unavailable estimate. Rendering SHALL round only the final percentage.

#### Scenario: Measured idle knot
- **WHEN** P45B uses minimum 2.7 V and measures 3.73 V per cell at zero current
- **THEN** its estimate is 0.5

#### Scenario: Interpolated load
- **WHEN** a 20S2P P45B pack with minimum 2.7 V first measures 72.1 V and 20 A discharge
- **THEN** its initialized estimate is 0.5 from the 10 A per-cell interpolated curve

#### Scenario: Display empty remapping
- **WHEN** P45B uses minimum 3.23 V and measures 3.775 V per cell at zero current
- **THEN** its estimate is 0.5 within floating-point tolerance

#### Scenario: Endpoints and excessive current
- **WHEN** voltage lies at or below configured minimum, or at or above 4.2 V per cell
- **THEN** the estimate is respectively 0 or 1 even when discharge current exceeds the measured curves
- **AND** excessive current is identified as `load_clamped` for a multi-load profile

#### Scenario: Missing current or regeneration
- **WHEN** voltage is valid but current is absent, zero, or negative
- **THEN** the zero-load curve produces a `voltage_only` estimate without retaining prior discharge compensation

### Requirement: Coherent bounded estimator lifecycle

Estimation SHALL follow accepted sample ordering and use bounded constant-size current-filter state. It SHALL smooth positive pack current using a two-second time-constant EMA, initialized from the first valid sample, and reset on reconnect, board/controller change, estimation-configuration change, or a sample gap exceeding five seconds. Duplicate/out-of-order samples SHALL NOT advance the filter. Missing/nonpositive current SHALL clear it. View recreation SHALL NOT restart or advance it. A configuration edit SHALL recompute the latest accepted sample coherently. Nonready connections SHALL retain their matching last estimate under existing stale-display behavior; new invalid voltage SHALL produce unavailable rather than reuse an old percentage as current.

#### Scenario: Reconnect and view recreation
- **WHEN** a WebView is recreated while the connection remains ready
- **THEN** it receives the existing estimate without resetting current smoothing
- **WHEN** the controller subsequently reconnects
- **THEN** the first accepted sample initializes new current-filter state

#### Scenario: Profile change while live
- **WHEN** the selected profile or pack configuration changes successfully
- **THEN** the next authoritative snapshot associates the recomputed estimate with that profile revision and the latest accepted sample

#### Scenario: Repeated sample
- **WHEN** a sample is duplicated or arrives out of order
- **THEN** it does not change the estimate or advance filtering

#### Scenario: Telemetry gap
- **WHEN** the next accepted sample arrives more than five seconds after the previous one
- **THEN** filtering initializes from its current instead of carrying the previous load forward
