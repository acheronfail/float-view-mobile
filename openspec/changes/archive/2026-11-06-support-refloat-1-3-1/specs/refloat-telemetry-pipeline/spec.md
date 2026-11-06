## ADDED Requirements

### Requirement: Stable Refloat 1.3.1 admission

The shared backend and simulator SHALL admit exact stable Refloat 1.3.1 using beta3's selectable telemetry, stable controls, typed IMU-timeout alerts, history and mutation safety rules. Beta2 and beta3 SHALL remain supported; observed identities SHALL remain exact.

#### Scenario: Stable release connects

- **WHEN** supported VESC firmware reports exact stable 1.3.1
- **THEN** readiness uses command 33 and command 35 with no dynamic-layout negotiation or duplicate command-10 fault probe
- **AND** reconnect and saved session context preserve `1.3.1`

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

- **WHEN** INFO reports a release outside the audited catalog, including another prerelease or stable 1.3.2
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

### Requirement: Audited beta3 telemetry and control compatibility

In addition to existing audited releases, the shared backend and simulator SHALL admit exact Refloat 1.3.1-beta3 using beta2's selectable telemetry, product controls, IMU-timeout alerts, history and mutation safety rules. The full beta3 identity SHALL survive live display, saved session context and reconnect. Other unaudited releases SHALL remain unsupported.

#### Scenario: Beta3 direct or CAN discovery

- **WHEN** a verified direct or CAN controller reports exact 1.3.1-beta3
- **THEN** discovery accepts it, polls command 33 and command 35 without command-32 negotiation or a duplicate command-10 fault probe, and preserves its exact identity

#### Scenario: Beta3 timeout authority

- **WHEN** beta3 reports active, unknown, expired or cleared IMU-timeout evidence
- **THEN** the existing beta2 alert/history handling and submission-time mutation gates apply, including queued mutations and the flywheel-disable escape path

#### Scenario: Neighboring unaudited release

- **WHEN** INFO reports beta1, beta4, stable 1.3.2 or a malformed suffix
- **THEN** discovery remains unsupported without write traffic
