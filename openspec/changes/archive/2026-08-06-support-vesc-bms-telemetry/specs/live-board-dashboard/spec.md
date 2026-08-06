## ADDED Requirements

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
