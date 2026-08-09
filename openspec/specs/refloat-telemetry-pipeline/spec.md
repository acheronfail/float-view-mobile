## Purpose

Define the source-faithful Refloat telemetry protocol, normalization, bounded extensions, polling, and durable lifecycle shared by native backends and the conformance simulator.

## Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful telemetry plus only the Refloat product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3. The independently allowlisted VESC-core shutdown command MUST NOT broaden the Refloat package-command allowlist. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL negotiate and decode the complete always-present and running-only field lists plus package state, mode, flags, stop condition, setpoint adjustment, alert reason, charging tail, and the 1.2.x active-alert and fatal-latch state. Command 35 history and data-recorder commands 41–43 SHALL NOT be requested or simulated by this change.

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

- **WHEN** the fixture receives a version-correct package, flywheel, lighting, or clear-fatal command permitted by `refloat-live-controls` in the current authoritative package mode
- **THEN** it applies only that bounded semantic transition and returns or later reports the source-defined confirmation

#### Scenario: VESC shutdown is not a Refloat command

- **WHEN** the fixture receives exact VESC-core command `[156, 0, 0]` under its independently eligible shutdown profile
- **THEN** it applies only the controller-loss simulation and does not route command 156 into Refloat package handling or broaden the Refloat mutation allowlist

#### Scenario: Flywheel write exclusion

- **WHEN** flywheel mode is active and the fixture receives any mutation other than the exact eligible flywheel-disable command
- **THEN** it reports no simulated success, sends no mutation onward, and preserves semantic and configuration state

#### Scenario: Deferred history command

- **WHEN** a client sends command 35 or a data-recorder command
- **THEN** the fixture returns no simulated history or recording data and records one bounded unsupported-command diagnostic

### Requirement: Stable normalized detailed telemetry

Both backends SHALL normalize supported wire families into the exact stable catalog, names, enums, and units required by the `backend-contract` capability. Dynamic `motor.speed` SHALL be interpreted as kilometres/hour because Refloat performs the metres/second conversion before command 31 encoding. Compact command 10 speed SHALL be interpreted as metres/second and converted exactly once to kilometres/hour by the backend. The shape SHALL retain core motor values; motion and IMU values; rider input; requested, filtered, directional, booster, ATR, and tilt-control values; package state and mode; wheelslip, darkride, and charging flags; stop and setpoint-adjustment states; alert reason; field-weakening current; accumulated distance and energy values; battery level; and active firmware-fault information when present. It SHALL NOT contain a derived utilization value. A missing source value SHALL remain null or absent and MUST NOT be fabricated as zero.

#### Scenario: Equivalent source-family values

- **WHEN** legacy and dynamic responses represent the same semantic board state
- **THEN** Apple and Android publish equivalent normalized values despite their different wire layouts

#### Scenario: Source value is unavailable

- **WHEN** a package family or detail level does not carry a normalized field
- **THEN** the field is null or absent in live state, persistence, retrieval, and export rather than reported as a measured zero

#### Scenario: Non-finite value

- **WHEN** a decoded or simulated numeric value is non-finite or outside its protocol bound
- **THEN** the sample is rejected or the invalid field is omitted according to the shared decoder rule, with bounded diagnostics and no invalid JSON or database value

### Requirement: Bounded negotiated extension fields

Known negotiated realtime string IDs SHALL map to typed fields. Unknown valid negotiated IDs whose transmitted values are numeric MAY be retained in an extension map containing at most 32 entries with finite values. A negotiated layout SHALL contain at most 64 fields, each UTF-8 field name SHALL be at most 64 bytes, and all encoded names together SHALL be at most 4,096 bytes. Duplicate IDs, excessive lists, oversized names, or response shapes inconsistent with the negotiated layout SHALL fail telemetry setup or drop the response deterministically and MUST NOT create unbounded state or storage.

#### Scenario: Future numeric field

- **WHEN** a supported package advertises one unknown bounded realtime ID with a numeric value
- **THEN** both backends retain it under the same extension key while typed fields remain unchanged

#### Scenario: Excessive field layout

- **WHEN** a negotiated layout exceeds the supported field or encoded-name bounds
- **THEN** telemetry setup fails within its deadline without allocating or publishing a truncated layout as complete

### Requirement: Serialized route-correct polling

Every detailed telemetry request SHALL use the one selected verified direct-or-CAN controller route and the single serialized VESC request stream. Dynamic command 31 SHALL use the existing adaptive fast/idle cadence. Dynamic command 10 detail level 4 SHALL become due once per second and SHALL occupy the next available request slot without a parallel lane or request queue. A compact timeout SHALL NOT trigger an immediate retry. Reconnect or controller change SHALL clear route-scoped compact state, renegotiate the layout, and revalidate the route before polling resumes.

#### Scenario: Dynamic and slow telemetry

- **WHEN** a dynamic session requires fast realtime values and compact slow-changing values
- **THEN** one compact request uses the next free slot after its one-second deadline and realtime polling resumes after its response or timeout

#### Scenario: CAN detailed telemetry

- **WHEN** the verified route is CAN ID 42
- **THEN** layout, realtime, and compact requests are each forwarded exactly once to CAN ID 42

#### Scenario: Reconnect

- **WHEN** BLE reconnects during a detailed telemetry session
- **THEN** old responses are ignored and no detailed sample is accepted until the new generation validates the controller and renegotiates its telemetry family

### Requirement: Durable detailed telemetry lifecycle

For a dynamic package, each accepted compact response SHALL replace the latest compact cache and its `slowObservedAt`; the next accepted realtime response SHALL merge that cache into one normalized sample. Device-reported accumulated values SHALL replace previous values even when they decrease. Each accepted normalized sample SHALL be assigned one monotonic session sequence and stored durably with bounded typed detail and extension values in the same commit unit. A changed discrete state SHALL append one deduplicated typed `session_event` in that transaction. Live state, session metadata/data pages, and export SHALL derive from the normalized records and observed events. The current disposable schema SHALL be updated consistently on Apple and Android without a migration framework.

#### Scenario: Persist and recover detailed sample

- **WHEN** a detailed sample is accepted and the frontend is recreated
- **THEN** the live snapshot and session metadata/data recover the same committed values and sequence without requiring the WebView to have observed the original event

#### Scenario: Device counter decreases

- **WHEN** a compact response reports an odometer or energy total lower than the latest cached value
- **THEN** the backend accepts the new finite protocol-valid value, records its new `slowObservedAt`, and does not clamp or reconstruct the old value

#### Scenario: Partial storage failure

- **WHEN** durable storage cannot commit a detailed sample
- **THEN** no partially populated sample is exposed as committed and the existing bounded failure and session-integrity rules apply

#### Scenario: Detailed export

- **WHEN** a session containing detailed and extension telemetry is exported
- **THEN** the export contains stable typed names, bounded extension values, nulls for unavailable fields, and no platform Bluetooth identifier; canonical VESC controller UUIDs MAY be included

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

### Requirement: Configuration safety evidence

The native application SHALL use authoritative normalized Refloat telemetry as safety evidence for every device mutation and for current-configuration authority. Persistent configuration eligibility SHALL require exactly idle ride state, normal package mode, no engaged footpad, and a fresh sample belonging to the current connection generation. While flywheel mode is active, every mutation except eligible flywheel disable SHALL be unavailable, loaded current configuration SHALL be invalidated, and command 93 SHALL be withheld so temporary in-memory calibration cannot become an authoritative base. Missing, unknown, stale, or contradictory evidence MUST make the write unavailable. Configuration reads SHALL remain independent of ride-session persistence and SHALL NOT create samples, events, aggregates, extensions, or exports.

#### Scenario: Fresh idle evidence

- **WHEN** current-generation telemetry reports idle, normal mode, and no footpad engagement
- **THEN** the safety evidence may permit a write if every other access and mutation condition also passes

#### Scenario: Missing legacy evidence

- **WHEN** a package family cannot supply one required safety field authoritatively
- **THEN** writes remain unavailable rather than treating the missing field as safe

#### Scenario: Configuration read during riding

- **WHEN** current or default values are requested while riding in non-flywheel package mode
- **THEN** the read may proceed at scheduler priority without changing or persisting telemetry state

#### Scenario: Configuration access during flywheel

- **WHEN** authoritative telemetry reports flywheel mode
- **THEN** schema and default reads may continue, current reads are rejected or withheld, and no current revision or device mutation is authorized except eligible flywheel disable

### Requirement: Audited custom configuration exception

The existing Refloat read and product-control allowlists SHALL additionally permit only VESC custom-configuration commands 92–95 for audited stable Refloat configuration index `0` under `refloat-configuration`. Refloat app-data tuning, Handtest, remote, RC-move, motor-control, recording, history, unknown custom indexes, malformed custom payloads, and every other mutation SHALL remain unsupported.

#### Scenario: Allowed schema and values read

- **WHEN** an audited supported controller receives a valid route-correct command 92, 93, or 94 for index `0`
- **THEN** the simulator returns only the exact source-version schema chunk or complete signed value payload

#### Scenario: Allowed verified write

- **WHEN** an audited writable idle controller in authoritatively normal package mode receives a complete valid command 95 payload through the specified configuration lifecycle
- **THEN** the simulator updates disposable configuration state for later command-93 verification

#### Scenario: Flywheel mutation remains rejected

- **WHEN** flywheel mode is active and a peer attempts command 95 or another mutation other than eligible flywheel disable
- **THEN** no success is simulated, no values change, and one bounded diagnostic is recorded

#### Scenario: Arbitrary mutation remains rejected

- **WHEN** a peer sends an unknown custom index, partial value payload, runtime tune app-data command, or other non-allowlisted mutation
- **THEN** no success is simulated, semantic board state is unchanged, and one bounded diagnostic is recorded
