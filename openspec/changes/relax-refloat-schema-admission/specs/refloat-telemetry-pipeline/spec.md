## MODIFIED Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and simulator SHALL preserve complete source-faithful audited telemetry and allow only the product mutations authorized by `refloat-live-controls`. Unknown Refloat releases SHALL use only the bounded validated public read fallback. This fallback SHALL NOT negotiate internal layouts or inherit audited alert/control semantics; VESC shutdown SHALL NOT broaden package-command authority.

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

- **WHEN** an audited Refloat 1.1.x or 1.2.x package responds to command 32 without one or more required semantic IDs
- **THEN** discovery reports unsupported Refloat and does not mark the controller ready

#### Scenario: Unknown or prerelease package

- **WHEN** INFO reports a valid release outside the audited catalog
- **THEN** discovery attempts only bounded public read admission; it reports unsupported Refloat if that protocol cannot be validated, rather than rejecting the release label alone

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

In addition to existing audited releases, the shared backend and simulator SHALL admit exact Refloat 1.3.1-beta3 using beta2's selectable telemetry, product controls, IMU-timeout alerts, history and mutation safety rules. The full beta3 identity SHALL survive live display, saved session context and reconnect. Unaudited releases SHALL receive only independently validated limited read support; stable release admission SHALL NOT retire beta3 capabilities.

#### Scenario: Beta3 direct or CAN discovery

- **WHEN** a verified direct or CAN controller reports exact 1.3.1-beta3
- **THEN** discovery accepts it, polls command 33 and command 35 without command-32 negotiation or a duplicate command-10 fault probe, and preserves its exact identity

#### Scenario: Beta3 timeout authority

- **WHEN** beta3 reports active, unknown, expired or cleared IMU-timeout evidence
- **THEN** the existing beta2 alert/history handling and submission-time mutation gates apply, including queued mutations and the flywheel-disable escape path

#### Scenario: Neighboring unaudited release

- **WHEN** INFO reports a valid unknown beta1, beta4 or stable 1.3.2 identity, or a malformed suffix
- **THEN** valid unknown identities can attempt bounded public reads without write authority, while malformed INFO is rejected without telemetry or write traffic

### Requirement: Audited custom configuration exception

The existing Refloat read and product-control allowlists SHALL additionally permit only VESC custom-configuration commands 92–95 for audited Refloat configuration index `0`, and commands 92–94 for matching limited read profiles under `refloat-configuration`. Refloat app-data tuning, Handtest, remote, RC-move, motor-control, recording, history, unknown custom indexes, malformed custom payloads, and every other mutation SHALL remain unsupported.

#### Scenario: Allowed schema and values read

- **WHEN** an audited controller or validated limited controller with a matching read profile receives a valid route-correct command 92, 93, or 94 for index `0`
- **THEN** the simulator returns only the actual connection schema chunk or complete signed value payload

#### Scenario: Allowed verified write

- **WHEN** an audited writable idle controller in authoritatively normal package mode receives a complete valid command 95 payload through the specified configuration lifecycle
- **THEN** the simulator updates disposable configuration state for later command-93 verification

#### Scenario: Flywheel mutation remains rejected

- **WHEN** flywheel mode is active and a peer attempts command 95 or another mutation other than eligible flywheel disable
- **THEN** no success is simulated, no values change, and one bounded diagnostic is recorded

#### Scenario: Arbitrary mutation remains rejected

- **WHEN** a peer sends an unknown custom index, partial value payload, runtime tune app-data command, or other non-allowlisted mutation
- **THEN** no success is simulated, semantic board state is unchanged, and one bounded diagnostic is recorded

## ADDED Requirements

### Requirement: Preserved complete audited telemetry

Audited telemetry profiles SHALL retain complete detail levels, negotiated fields, applicable fault/alert probes and recorder exclusions. Adding limited support SHALL NOT reduce the existing audited observations.

#### Scenario: Audited telemetry selection

- **WHEN** a connection uses an audited release mapping
- **THEN** the backends and conformance simulator implement source-faithful telemetry plus only the Refloat product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0 plus exact 1.3.1-beta2 and beta3. The independently allowlisted VESC-core shutdown command MUST NOT broaden the Refloat package-command allowlist. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4 for releases through 1.2.3, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL remain selected only for audited Refloat 1.1.0–1.2.3 and SHALL negotiate and decode their complete field lists and applicable tails. Exact Refloat 1.3.0 SHALL use documented selectable command 33 for primary telemetry and slow values plus its existing bounded command-10 mode-0 active firmware-fault probe. Exact 1.3.1-beta2 and beta3 SHALL reuse command 33 for primary telemetry and slow values and command 35 for active alerts and bounded history, without a duplicate command-10 fault probe. Commands 31/32 SHALL NOT be requested for the selectable 1.3 family. Existing bounded command-35 alert history SHALL continue for applicable releases; data-recorder commands 41–43 remain unsupported.

### Requirement: Validated limited public telemetry

Limited support SHALL use recognized Refloat INFO v2 and the documented command-33 float32 protocol. A bounded core exchange SHALL validate exact echoed flags/masks, payload extent, tick interpretation and finite numeric values. Optional read groups SHALL be proven separately and SHALL NOT revoke usable core telemetry when unavailable.

#### Scenario: Core protocol establishes usable reads

- **WHEN** bounded exact-name Refloat INFO v2 has a valid complete version/suffix, recognized flags and supported tick rate
- **THEN** one generation-scoped command-33 probe requests only known state, speed and pack-voltage fields with the documented float32 flag and fixed mask
- **AND** only an exact complete finite response establishes limited support; no command-ID scan, alternate encoding guess or unbounded retries occur

#### Scenario: New informational capability bits

- **WHEN** an otherwise valid fixed-layout INFO v2 response contains new capability bits
- **THEN** those bits grant no feature authority and do not alone prevent the known public read probe; unknown layouts or invalid echoed request flags still fail

#### Scenario: Optional telemetry failure

- **WHEN** a fixed documented optional command-33 numeric group is unsupported, malformed or times out
- **THEN** that group is disabled for the generation after its single bounded discovery attempt, independently proven core polling and recording continue, and absent values remain null
- **AND** polling uses only validated fixed groups (remaining mask-1 numeric fields and non-GNSS slow mask-2 fields) through the existing serialized direct/CAN request path

#### Scenario: Unusable core response

- **WHEN** the core response has mismatched flags/masks, truncated or extra bytes, nonfinite numbers or unsupported tick interpretation
- **THEN** admission fails without partial samples, invented values, saved activation or mutation traffic

#### Scenario: No internal or unstable fallback probes

- **WHEN** a valid unknown release is evaluated or polled
- **THEN** only recognized public telemetry probes are attempted; commands 10, 31/32, 35, recorder commands and mutation-family queries are not probed to discover compatibility

### Requirement: Unknown limited status remains unknown

Limited telemetry SHALL preserve independent valid numeric observations when categorical values are unknown. Unobserved fault, fatal and timeout state SHALL remain null, and no alert history or clear transition SHALL be fabricated. Unknown package/ride state SHALL NOT establish safety or riding-time authority.

#### Scenario: Future categorical value

- **WHEN** a valid limited response contains an unknown package state, mode, stop reason, newly used reserved state bits or other categorical value
- **THEN** its existing unknown/null representation is used while independent finite numeric observations remain usable and recordable
- **AND** unknown ride state does not accumulate riding time or authorize status-dependent behavior

#### Scenario: Unavailable alert authority

- **WHEN** a limited connection has no independently verified alert profile
- **THEN** active firmware faults, fatal and IMU-timeout state stay unavailable, alert history stays empty/unavailable, and no healthy, inactive or recovered event is inferred
- **AND** an observed stop-condition code alone does not establish active timeout state
