## MODIFIED Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful telemetry plus only the Refloat product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0, plus exact 1.3.1-beta1. The independently allowlisted VESC-core shutdown command MUST NOT broaden the Refloat package-command allowlist. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4 for releases through 1.2.3, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL remain selected only for audited Refloat 1.1.0–1.2.3 and SHALL negotiate and decode their complete field lists and applicable tails. Exact Refloat 1.3.0 and 1.3.1-beta1 SHALL use documented selectable command 33 for primary telemetry and slow values. Existing audited command-35 recovery SHALL remain selected where already used; a command-10 mode-0 fault probe SHALL be used only where the selected profile lacks command-35 recovery, without a duplicate polling lane. Commands 31/32 SHALL NOT be requested for Refloat 1.3.0 or 1.3.1-beta1. For exact 1.3.1-beta1, bounded command-35 alert recovery SHALL supplement command 33 as specified below. Data-recorder commands 41–43 remain excluded; this beta exception SHALL NOT broaden any older release profile.

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

- **WHEN** INFO reports a release outside the audited stable catalog or includes a prerelease suffix other than exact audited 1.3.1-beta1
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

- **WHEN** a client sends command 35 outside an audited read profile or a data-recorder command
- **THEN** the fixture returns no simulated history or recording data and records one bounded unsupported-command diagnostic

### Requirement: Raw load and active fault semantics

The product SHALL expose source-available raw actual, requested, directional, filtered, booster, and field-weakening/d-axis current independently in amperes and SHALL NOT derive torque or current utilization. Refloat 1.3.0's signed command-33 `motor_id` SHALL populate the field-weakening/d-axis current value. Its compact booster byte represents normalized torque and MUST NOT populate `boosterCurrentA`; because command 33 supplies no booster current, that field SHALL remain null for 1.3.0. Refloat 1.2.x and 1.3.0 fatal-latch state SHALL remain distinct from active firmware-fault code, transient pushback, and alert-reason state. For the beta, recovered command-35 activation and clear records SHALL supplement observed transitions with distinct source and observation timing. The live-only fatal latch SHALL NOT be persisted as history. Older release semantics remain unchanged.

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

- **WHEN** Refloat 1.3.0 exposes command 35 alert masks, fault names, or history
- **THEN** Float View does not request, publish, or persist them and retains only alert reason, active firmware fault, and live fatal latch

## ADDED Requirements

### Requirement: Beta selectable telemetry parity

Exact 1.3.1-beta1 SHALL retain audited 1.3.0 command-33 fields, masks, validation, adaptive fast/slow cadence, GNSS capability gates, normalized units, unavailable-value nulls, and control confirmation. Beta additions SHALL NOT reinterpret booster torque as current or board GNSS as OS location.

#### Scenario: Equivalent normal telemetry

- **WHEN** 1.3.0 and 1.3.1-beta1 report equivalent normal fast and slow responses
- **THEN** normalized values and extensions agree, and beta polling does not introduce command 31 or 32

### Requirement: Distinct IMU timeout state

For exact 1.3.1-beta1 the application SHALL decode stop code 7 as `imuTimeout` and command-35 alert ID 2 as an IMU timeout with independent active state. Timeout SHALL remain distinct from VESC firmware faults, generic pushback, and the fatal latch; firmware-fault alert ID 1 SHALL retain its existing meaning.

#### Scenario: Active timeout with zero firmware code

- **WHEN** the beta reports active alert ID 2 with code 0 and stop code 7
- **THEN** live state reports an active IMU timeout and the typed stop reason without fabricating a firmware fault or fatal latch

#### Scenario: Concurrent independent alerts

- **WHEN** firmware-fault and IMU-timeout alerts are simultaneously active or clear independently
- **THEN** each state is retained independently and neither masks, clears, or relabels the other

### Requirement: Bounded beta alert recovery

Beta command-35 reads SHALL use the existing serialized selected-route scheduler and bounded low-cadence history cursor. Each complete response SHALL validate masks, record IDs, types, extent and code semantics before publication. Up to 20 source records per response SHALL be accepted and deduplicated within bounded state; timeout activation with code 0 SHALL be valid. Controller-provided names SHALL NOT become trusted labels.

#### Scenario: Timeout between samples

- **WHEN** command 35 recovers a timeout activation and clear that occurred between command-33 samples
- **THEN** both are retained once as distinct timeout history with source ticks, observation time, and nullable approximate occurrence time

#### Scenario: Mixed and repeated history

- **WHEN** a valid response interleaves firmware-fault and timeout records or overlaps an earlier cursor
- **THEN** wire order is preserved, duplicates are suppressed, and existing firmware-fault history remains correctly typed

#### Scenario: Invalid response

- **WHEN** a response exceeds bounds or has unsupported masks or IDs, inconsistent active/code state, truncated data, oversized names, or trailing bytes
- **THEN** the complete response is rejected without advancing the cursor or publishing partial live or durable state

#### Scenario: Timeout or stale response

- **WHEN** a history read times out or a reply belongs to an old route or generation
- **THEN** no immediate retry or parallel request lane is created and stale state cannot update the new connection

### Requirement: Durable timeout transitions

Saved sessions SHALL retain observed timeout state transitions and recovered activation/clear records through bounded history queries and exports. Recovered records SHALL be distinguished from sampled transitions and SHALL NOT inflate firmware-fault history. Guest connections SHALL retain bounded live state only. Existing supported data and archives SHALL remain readable without resets.

#### Scenario: Commit and reload

- **WHEN** a timeout record commits and the frontend or process restarts
- **THEN** saved history and export preserve its kind, active state, source/observation timing and ordering without relabeling it as a firmware fault

#### Scenario: History storage failure

- **WHEN** durable history cannot commit or reaches its established bounded capacity
- **THEN** no partial success is published and existing bounded storage/session-integrity failure behavior applies

#### Scenario: Guest recovery

- **WHEN** a guest connection recovers timeout history
- **THEN** live timeout state can update without creating a board, session, event row, or archive
