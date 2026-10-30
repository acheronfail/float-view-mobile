## Purpose

Define the source-faithful Refloat telemetry protocol, normalization, bounded extensions, polling, and durable lifecycle shared by native backends and the conformance simulator.

## Requirements

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

### Requirement: Stable normalized detailed telemetry

Both backends SHALL normalize supported wire families into the exact stable catalog, names, enums, and units required by the `backend-contract` capability. Commands 31 and 33 speed SHALL be interpreted as kilometres/hour because Refloat performs the metres/second conversion before encoding. Compact command 10 speed SHALL be interpreted as metres/second and converted exactly once to kilometres/hour by the backend. The shape SHALL retain every source-available core motor value; motion and IMU value; rider input; requested, filtered, directional, booster, ATR, and tilt-control value; package state and mode; wheelslip, darkride, and charging flag; stop and setpoint-adjustment state; alert reason; field-weakening/d-axis current; accumulated distance and energy value; battery level; charging value; and active firmware-fault information. Refloat 1.3.0 loop frequency, data-recording state, and capability-gated board GNSS values SHALL use canonical bounded `refloat.*` telemetry extensions and MUST NOT create OS location state. The shape SHALL NOT contain a derived utilization value. A missing source value SHALL remain null or absent and MUST NOT be fabricated as zero.

#### Scenario: Equivalent source-family values

- **WHEN** legacy, negotiated dynamic, and selectable responses represent the same semantic board state
- **THEN** Apple and Android publish equivalent normalized values despite their different wire layouts

#### Scenario: Complete selectable additions

- **WHEN** Refloat 1.3.0 supplies loop frequency, data-recording state, and capability-gated GNSS through command 33
- **THEN** each finite value is retained under its canonical bounded extension name through live state, persistence, retrieval, and export without altering OS location

#### Scenario: Source value is unavailable

- **WHEN** a package family or selected mask does not carry a normalized field
- **THEN** the field is null or absent in live state, persistence, retrieval, and export rather than reported as a measured zero

#### Scenario: Non-finite value

- **WHEN** a decoded or simulated numeric value is non-finite or outside its protocol bound
- **THEN** the response is rejected according to the shared decoder rule, with bounded diagnostics and no invalid JSON or database value

### Requirement: Bounded negotiated extension fields

Known negotiated realtime string IDs SHALL map to typed fields. Unknown valid negotiated IDs whose transmitted values are numeric MAY be retained in an extension map containing at most 32 entries with finite values. A negotiated layout SHALL contain at most 64 fields, each UTF-8 field name SHALL be at most 64 bytes, and all encoded names together SHALL be at most 4,096 bytes. Duplicate IDs, excessive lists, oversized names, or response shapes inconsistent with the negotiated layout SHALL fail telemetry setup or drop the response deterministically and MUST NOT create unbounded state or storage.

#### Scenario: Future numeric field

- **WHEN** a supported package advertises one unknown bounded realtime ID with a numeric value
- **THEN** both backends retain it under the same extension key while typed fields remain unchanged

#### Scenario: Excessive field layout

- **WHEN** a negotiated layout exceeds the supported field or encoded-name bounds
- **THEN** telemetry setup fails within its deadline without allocating or publishing a truncated layout as complete

### Requirement: Serialized route-correct polling

Every detailed telemetry request SHALL use the one selected verified direct-or-CAN controller route and the single serialized VESC request stream. Negotiated command 31 SHALL use the existing adaptive fast/idle cadence for Refloat 1.1.0–1.2.3, while command 10 detail level 4 becomes due once per second for those dynamic releases. Refloat 1.3.0 command 33 SHALL use the same adaptive cadence for its canonical fast mask and a once-per-second cadence for its canonical slow mask; a once-per-second command-10 mode-0 request SHALL serve only as the retained active firmware-fault probe. Slow and fault requests SHALL occupy the next available request slot without a parallel lane or queue, and a timeout SHALL NOT trigger an immediate retry. Reconnect or controller change SHALL clear route-scoped fast, slow, fault, and merge state; commands 31/32 releases SHALL renegotiate their layout, while exact 1.3.0 SHALL restart its fixed audited masks without command 32.

#### Scenario: Dynamic and slow telemetry

- **WHEN** a Refloat 1.1.x or 1.2.x session requires fast realtime values and compact slow-changing values
- **THEN** one detail-level-4 compact request uses the next free slot after its one-second deadline and command-31 polling resumes after its response or timeout

#### Scenario: Selectable fast and slow telemetry

- **WHEN** a Refloat 1.3.0 session requires fast and slow values
- **THEN** command 33 uses the adaptive fast mask and the next eligible one-second slot for the slow mask without requesting slow or GNSS fields at the fast cadence

#### Scenario: Retained active-fault probe

- **WHEN** Refloat 1.3.0 polling is active
- **THEN** a command-10 mode-0 probe uses a bounded low-cadence slot and only its special firmware-fault response may update active firmware fault state

#### Scenario: CAN detailed telemetry

- **WHEN** the verified route is CAN ID 42
- **THEN** every release-selected fast, slow, layout, compact, and fault request is forwarded exactly once to CAN ID 42

#### Scenario: Reconnect

- **WHEN** BLE reconnects during a detailed telemetry session
- **THEN** old responses are ignored and no detailed sample is accepted until the new generation validates the controller and re-establishes its exact telemetry profile

### Requirement: Durable detailed telemetry lifecycle

For a commands-31/32 package, each accepted compact response SHALL replace the latest compact cache and its `slowObservedAt`; the next accepted realtime response SHALL merge that cache into one normalized sample. For Refloat 1.3.0, each accepted slow command-33 response SHALL replace the latest selectable slow cache and its `slowObservedAt`; the next accepted fast command-33 response SHALL merge that cache into one normalized sample. A command-10 mode-0 response for 1.3.0 SHALL update only current active firmware-fault state and MUST NOT replace typed or extension telemetry. Device-reported accumulated values SHALL replace previous values even when they decrease. Each accepted normalized sample SHALL be assigned one monotonic session sequence and stored durably with bounded typed detail and extension values in the same commit unit. A changed discrete state SHALL append one deduplicated typed `session_event` in that transaction. Live state, session metadata/data pages, and export SHALL derive from the normalized records and observed events. The current disposable schema SHALL be updated consistently on Apple and Android without a migration framework.

#### Scenario: Persist and recover detailed sample

- **WHEN** a detailed sample is accepted and the frontend is recreated
- **THEN** the live snapshot and session metadata/data recover the same committed values and sequence without requiring the WebView to have observed the original event

#### Scenario: Selectable slow merge

- **WHEN** an accepted Refloat 1.3.0 slow response is followed by a current-generation fast response
- **THEN** the resulting sample contains the latest slow typed and extension values with their `slowObservedAt` and one new monotonic sample sequence

#### Scenario: Device counter decreases

- **WHEN** a compact or selectable slow response reports an odometer or energy total lower than the latest cached value
- **THEN** the backend accepts the new finite protocol-valid value, records its new `slowObservedAt`, and does not clamp or reconstruct the old value

#### Scenario: Partial storage failure

- **WHEN** durable storage cannot commit a detailed sample
- **THEN** no partially populated sample is exposed as committed and the existing bounded failure and session-integrity rules apply

#### Scenario: Detailed export

- **WHEN** a session containing detailed and extension telemetry is exported
- **THEN** the export contains stable typed names, bounded extension values, nulls for unavailable fields, and no private Bluetooth or controller identifier

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

### Requirement: Configuration safety evidence

The native application SHALL use authoritative normalized Refloat telemetry as safety evidence for every device mutation and for current-configuration authority. Persistent configuration eligibility SHALL require exactly idle ride state, normal package mode, and a fresh sample belonging to the current connection generation. While flywheel mode is active, every mutation except eligible flywheel disable SHALL be unavailable, loaded current configuration SHALL be invalidated, and command 93 SHALL be withheld so temporary in-memory calibration cannot become an authoritative base. Missing, unknown, stale, or contradictory required safety evidence MUST make the write unavailable. Footpad readings SHALL NOT be required safety evidence for configuration writes. Configuration reads SHALL remain independent of ride-session persistence and SHALL NOT create samples, events, aggregates, extensions, or exports.

#### Scenario: Fresh idle evidence

- **WHEN** current-generation telemetry reports idle and normal mode, regardless of footpad contact
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

### Requirement: Complete Refloat 1.3.0 selectable telemetry

For exact stable Refloat 1.3.0, the backends and conformance simulator SHALL encode command 33 with control flag `0x01` for float32 and only documented non-reserved bits from mask 1 and mask 2. The decoder SHALL require the response to echo the exact requested control flags and masks, derive the exact response extent from those masks, decode every selected field in source order and source type, and reject unsupported flags or bits, mismatches, truncation, trailing bytes, non-finite values, invalid state encodings, and out-of-range coordinates without partial publication. The implementation SHALL support every documented non-reserved field: extra and state flags; motor, battery, temperature, IMU, footpad, remote, setpoint, balance-current, and loop-frequency values; odometer, distance, charging and energy values; motor d-axis current; and GNSS latitude, longitude, altitude, speed, accuracy, and source timestamp.

#### Scenario: Complete fast response

- **WHEN** exact Refloat 1.3.0 echoes the canonical float32 fast mask with a complete response
- **THEN** state flags and every requested motor, IMU, footpad, remote, setpoint, current, temperature, and loop-frequency value are decoded in source order

#### Scenario: Complete slow response

- **WHEN** exact Refloat 1.3.0 echoes the canonical float32 slow mask with a complete response
- **THEN** battery state of charge, odometer, distance, charging, energy, and signed motor d-axis values are decoded in source order

#### Scenario: Capability-gated GNSS response

- **WHEN** INFO advertises GNSS capability and the slow request selects all documented GNSS bits
- **THEN** finite latitude, longitude, altitude, speed, accuracy, and source timestamp values are retained as board-provided telemetry without creating an OS location observation

#### Scenario: GNSS unavailable

- **WHEN** INFO does not advertise GNSS capability
- **THEN** command-33 requests omit every GNSS bit and the backend does not fabricate board coordinates or infer availability from zero values

#### Scenario: Echo or extent mismatch

- **WHEN** a command-33 response changes an echoed flag or mask, omits any mask-selected byte, or appends trailing bytes
- **THEN** the complete response is rejected without publishing or caching any selected value

#### Scenario: Invalid numeric value

- **WHEN** a selected float is non-finite or a selected GNSS coordinate is outside its valid range
- **THEN** the complete response is rejected with a bounded diagnostic and cannot become live, durable, exported, or mutation-safety state

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
