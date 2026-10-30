# Spec Delta

## MODIFIED Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful telemetry plus only the Refloat product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0 plus exact 1.3.1-beta2. The independently allowlisted VESC-core shutdown command MUST NOT broaden the Refloat package-command allowlist. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4 for releases through 1.2.3, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL remain selected only for audited Refloat 1.1.0–1.2.3 and SHALL negotiate and decode their complete field lists and applicable tails. Exact Refloat 1.3.0 SHALL use documented selectable command 33 for primary telemetry and slow values plus its existing bounded command-10 mode-0 active firmware-fault probe. Exact 1.3.1-beta2 SHALL reuse command 33 for primary telemetry and slow values and command 35 for active alerts and bounded history, without a duplicate command-10 fault probe. Commands 31/32 SHALL NOT be requested for the selectable 1.3 family. Existing bounded command-35 alert history SHALL continue for applicable releases; data-recorder commands 41–43 remain unsupported.

#### Scenario: Complete legacy compact response

- **WHEN** a supported 1.0.x controller returns compact telemetry at detail level 4
- **THEN** the backend decodes every present legacy field without substituting a reduced core-only record

#### Scenario: Legacy command 1

- **WHEN** the simulator receives command 1 for Refloat 1.0.x
- **THEN** it returns the source-faithful high-precision legacy realtime layout while the native backends continue polling command 10

#### Scenario: Complete dynamic response

- **WHEN** a supported Refloat 1.1.x or 1.2.x controller advertises the complete known always-present and running-only realtime IDs
- **THEN** the backend decodes all known values, state flags, and version-applicable tails in their negotiated order

#### Scenario: Audited dynamic release

- **WHEN** INFO reports one of 1.1.0–1.1.2 or 1.2.0–1.2.3 and command 32 supplies the complete required profile baseline plus bounded additional IDs
- **THEN** discovery accepts the controller under its audited wire profile and retains additional numeric values as bounded extensions

#### Scenario: Audited selectable release

- **WHEN** INFO reports exact stable Refloat 1.3.0
- **THEN** discovery accepts the selectable profile without command-32 negotiation and begins bounded command-33 polling

#### Scenario: Incomplete or renamed dynamic profile

- **WHEN** a Refloat 1.1.x, 1.2.x, or unknown package responds to command 32 without one or more required semantic IDs
- **THEN** discovery reports unsupported Refloat and does not mark the controller ready

#### Scenario: Unknown or prerelease package

- **WHEN** INFO reports a release outside the audited catalog, including another prerelease or stable 1.3.1
- **THEN** discovery reports unsupported Refloat even if its command numbers are recognizable

#### Scenario: Unsupported write command

- **WHEN** the fixture receives Refloat tuning, arbitrary configuration, Handtest, remote, RC-move, motor-control, recording, history, or another mutation outside the product-control allowlist
- **THEN** it returns no simulated success, records one bounded diagnostic, and does not mutate board state

#### Scenario: Allowed product control

- **WHEN** the fixture receives a version-correct package, flywheel, lighting, or clear-fatal command permitted by `refloat-live-controls` in the current authoritative package mode
- **THEN** it applies only that bounded semantic transition and returns or later reports the source-defined confirmation

#### Scenario: VESC shutdown is not a Refloat command

- **WHEN** the fixture receives exact VESC-core command `[156, 0, 0]` under its independently eligible shutdown profile
- **THEN** it applies only the controller-loss simulation and does not route command 156 into Refloat package handling or broaden the Refloat mutation allowlist

#### Scenario: Flywheel write exclusion

- **WHEN** flywheel mode is active and the fixture receives any mutation other than the exact eligible flywheel-disable command
- **THEN** it reports no simulated success, sends no mutation onward, and preserves semantic and configuration state

#### Scenario: Deferred history command

- **WHEN** a supported release receives the route-correct command-35 read
- **THEN** the fixture returns its bounded source-defined alert mask and retained records without enabling data-recorder or unsupported mutation commands

### Requirement: Raw load and active fault semantics

The product SHALL expose source-available raw actual, requested, directional, filtered, booster, and field-weakening/d-axis current independently in amperes and SHALL NOT derive torque or current utilization. Refloat 1.3.0's signed command-33 `motor_id` SHALL populate the field-weakening/d-axis current value. Its compact booster byte represents normalized torque and MUST NOT populate `boosterCurrentA`; because command 33 supplies no booster current, that field SHALL remain null for 1.3.0. Refloat 1.2.x, 1.3.0, and 1.3.1-beta2 fatal-latch state SHALL remain distinct from active firmware-fault code, transient pushback, and alert-reason state. Session history SHALL distinguish sampled transitions from bounded command-35 recovered alert records and SHALL NOT persist the live-only fatal latch.

#### Scenario: High current

- **WHEN** actual, requested, filtered, and field-weakening/d-axis currents are present
- **THEN** each is retained under its own stable field without a utilization ratio

#### Scenario: Refloat 1.3.0 booster torque

- **WHEN** a normal Refloat 1.3.0 command-10 response contains its booster-torque byte
- **THEN** `boosterCurrentA` remains null and the byte is not mislabeled, persisted, or exported as amperes

#### Scenario: Firmware fault and pushback

- **WHEN** duty pushback is active without a VESC firmware fault
- **THEN** the sample reports duty setpoint adjustment and its alert reason without fabricating an active firmware fault or fatal latch

#### Scenario: Fault ends but latch remains

- **WHEN** Refloat 1.2.x or 1.3.0 clears its active firmware fault but retains the encoded fatal-error state bit
- **THEN** live control state reports an inactive fault and a clearable fatal latch without adding a historical firmware-fault transition

#### Scenario: Unstable alert history remains excluded

- **WHEN** Refloat 1.3.0 returns valid command-35 firmware-fault records
- **THEN** Float View retains bounded recovered history separately from active firmware-fault and fatal-latch state

## ADDED Requirements

### Requirement: Refloat IMU timeout alert and history

For audited 1.3.1-beta2, the backend SHALL decode IMU timeout as a package error distinct from firmware faults and fatal latch. It SHALL expose authoritative active state, retain bounded command-35 activation and clearing records, and persist recovered records for saved rides without inventing an exact occurrence time.

#### Scenario: Active and cleared timeout

- **WHEN** command 35 reports timeout alert ID 2 active and later cleared with code zero
- **THEN** live state changes from active to inactive and both transitions remain distinct from firmware-fault records

#### Scenario: Transient timeout between samples

- **WHEN** a bounded command-35 response contains timeout activation and clearing between telemetry samples
- **THEN** a saved ride retains both records in order and a guest connection retains them only within its bounded live history

#### Scenario: Retained stop reason

- **WHEN** command 33 reports stop condition 7 after command 35 has reported the timeout inactive
- **THEN** the sample retains `imuTimeout` as the stop reason without claiming that the error is still active

#### Scenario: Missing or failed alert observation

- **WHEN** no valid current-generation command-35 response establishes timeout state, or inactive evidence expires after failed polling
- **THEN** live active state is unknown and no false clearing event is recorded; an observed active state persists until a valid clearing response

#### Scenario: Malformed or stale history

- **WHEN** a command-35 response has unsupported IDs or mask bits, inconsistent active/code shape, truncation, trailing bytes, or a stale route or generation
- **THEN** it is rejected atomically without changing live state, cursor, history, or durable events

#### Scenario: Bounded retention and storage failure

- **WHEN** overlapping reads, source-buffer limits, clock wrap, or a failed durable commit occur
- **THEN** records remain deduplicated and bounded, occurrence time remains nullable, and no uncommitted event is reported as durable
