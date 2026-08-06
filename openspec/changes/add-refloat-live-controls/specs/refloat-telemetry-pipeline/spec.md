## MODIFIED Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful telemetry plus only the product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL negotiate and decode the complete always-present and running-only field lists plus package state, mode, flags, stop condition, setpoint adjustment, alert reason, charging tail, and the 1.2.x active-alert and fatal-latch state. Command 35 history and data-recorder commands 41–43 SHALL NOT be requested or simulated by this change.

#### Scenario: Complete legacy compact response

- **WHEN** a supported 1.0.x controller returns compact telemetry at detail level 4
- **THEN** the backend decodes every present legacy field without substituting a reduced core-only record

#### Scenario: Legacy command 1

- **WHEN** the simulator receives command 1 for Refloat 1.0.x
- **THEN** it returns the source-faithful high-precision legacy realtime layout while the native backends continue polling command 10

#### Scenario: Complete dynamic response

- **WHEN** a supported dynamic controller advertises the complete known always-present and running-only realtime IDs
- **THEN** the backend decodes all known values, state flags, and version-applicable tails in their negotiated order

#### Scenario: Audited dynamic release

- **WHEN** INFO reports one of 1.1.0–1.1.2 or 1.2.0–1.2.3 and command 32 supplies the complete required profile baseline plus bounded additional IDs
- **THEN** discovery accepts the controller under its audited wire profile and retains additional numeric values as bounded extensions

#### Scenario: Incomplete or renamed dynamic profile

- **WHEN** a known or unknown package responds to command 32 without one or more required semantic IDs
- **THEN** discovery reports unsupported Refloat and does not mark the controller ready

#### Scenario: Unknown or prerelease package

- **WHEN** INFO reports a release outside the audited stable catalog or includes a prerelease suffix
- **THEN** discovery reports unsupported Refloat even if its command numbers are recognizable

#### Scenario: Unsupported write command

- **WHEN** the fixture receives Refloat tuning, arbitrary configuration, Handtest, remote, RC-move, motor-control, recording, history, or another mutation outside the product-control allowlist
- **THEN** it returns no simulated success, records one bounded diagnostic, and does not mutate board state

#### Scenario: Allowed product control

- **WHEN** the fixture receives a version-correct package, flywheel, lighting, or clear-fatal command permitted by `refloat-live-controls`
- **THEN** it applies only that bounded semantic transition and returns or later reports the source-defined confirmation

#### Scenario: Deferred history command

- **WHEN** a client sends command 35 or a data-recorder command
- **THEN** the fixture returns no simulated history or recording data and records one bounded unsupported-command diagnostic

### Requirement: Raw load and active fault semantics

The product SHALL expose raw actual, requested, directional, filtered, booster, and field-weakening current independently in amperes and SHALL NOT derive torque or current utilization. Refloat 1.2.x fatal-latch state SHALL remain distinct from its active firmware-fault code, transient pushback, and alert-reason state. Session history SHALL contain only transitions observed by Float View, SHALL NOT persist the live-only fatal latch, and SHALL NOT claim to be command 35 device history.

#### Scenario: High current

- **WHEN** actual, requested, filtered, and field-weakening currents are present
- **THEN** each is retained under its own stable field without a utilization ratio

#### Scenario: Firmware fault and pushback

- **WHEN** duty pushback is active without a VESC firmware fault
- **THEN** the sample reports duty setpoint adjustment and its alert reason without fabricating an active firmware fault or fatal latch

#### Scenario: Fault ends but latch remains

- **WHEN** Refloat 1.2.x clears its active firmware-fault mask but retains the encoded fatal-error state bit
- **THEN** live control state reports an inactive fault and a clearable fatal latch without adding a historical firmware-fault transition
