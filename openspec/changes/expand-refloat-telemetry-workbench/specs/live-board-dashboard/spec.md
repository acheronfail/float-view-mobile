## ADDED Requirements

### Requirement: Detailed live board telemetry
The shared live dashboard SHALL present available detailed telemetry in coherent groups for motion, electrical load, temperatures, rider input, control contributions, and accumulated values. It SHALL clearly distinguish measured, requested, directional, filtered, booster, and field-weakening currents in amperes and duty as a ratio. It SHALL NOT derive or display torque or current utilization. Missing values SHALL display as unavailable rather than zero.

#### Scenario: Detailed dynamic session
- **WHEN** a Refloat 1.2.3 session supplies full dynamic telemetry
- **THEN** the dashboard can display its IMU, footpad, current, ATR, tilt, remote, and core motor values without platform branching

#### Scenario: Legacy unavailable value
- **WHEN** the selected legacy detail level does not provide a field shown by the dashboard
- **THEN** that field is hidden or marked unavailable and is not displayed as zero

### Requirement: Live semantic board states
The live experience SHALL surface package state and mode, footpad state, wheelslip, darkride, charging, stop condition, setpoint adjustment or pushback type, alert reason, and bounded firmware fault with accessible text and non-color-only distinctions. Safety-relevant states SHALL remain visible while active without implying that the application controls the board.

#### Scenario: Duty pushback
- **WHEN** telemetry reports running state, duty pushback, and a duty alert reason
- **THEN** the live view identifies duty pushback distinctly from high-voltage, low-voltage, temperature, speed, and firmware-fault states

#### Scenario: Wheelslip and footpad
- **WHEN** wheelslip is active and only one footpad side is engaged
- **THEN** both states are conveyed accessibly and neither is inferred solely from current or speed

### Requirement: Detailed historical telemetry review
Session detail SHALL expose the same supported detailed fields through bounded numeric charts and values, plus observed semantic transitions through bounded typed event annotations. It SHALL preserve nullability and units, allow a developer to inspect fixture-generated scenes after disconnect, keep each response within 2,000 series points and 2,000 events, and disclose when earlier events were truncated.

#### Scenario: Review field weakening
- **WHEN** a session includes field-weakening current and high duty
- **THEN** historical detail shows both raw series independently in their contract units

#### Scenario: Review stop event
- **WHEN** a session transitions through a stop condition
- **THEN** session detail makes the state transition inspectable alongside surrounding telemetry

### Requirement: Detailed telemetry interactions
Any interactive telemetry disclosure, group selector, chart control, or session annotation control SHALL use native semantics, appropriate `touch-action`, primary Pointer Events with cancellation, and a guarded keyboard and assistive-technology click path without double activation.

#### Scenario: Touch disclosure
- **WHEN** a primary touch pointer opens a detailed telemetry group in a frequently updating live view
- **THEN** it opens exactly once and remains operable while samples continue updating

#### Scenario: Non-primary activation
- **WHEN** a non-primary pointer or cancelled gesture targets a detailed telemetry control
- **THEN** no action is performed
