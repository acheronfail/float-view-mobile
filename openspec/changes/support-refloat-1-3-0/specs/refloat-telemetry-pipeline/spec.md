## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful telemetry plus only the Refloat product-control allowlist defined by `refloat-live-controls` for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, 1.2.0–1.2.3, and 1.3.0. The independently allowlisted VESC-core shutdown command MUST NOT broaden the Refloat package-command allowlist. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4 for releases through 1.2.3, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL remain selected only for audited Refloat 1.1.0–1.2.3 and SHALL negotiate and decode their complete field lists and applicable tails. Exact Refloat 1.3.0 SHALL use documented selectable command 33 for primary telemetry and slow values plus a bounded command-10 mode-0 probe only for its special active firmware-fault response. Commands 31/32 SHALL NOT be requested for Refloat 1.3.0. Command 35 history and data-recorder commands 41–43 SHALL NOT be requested or simulated by this change.

#### Scenario: Complete legacy compact response

- **WHEN** a supported 1.0.x controller returns compact telemetry at detail level 4
- **THEN** the backend decodes every present legacy field without substituting a reduced core-only record

#### Scenario: Legacy command 1

- **WHEN** the simulator receives command 1 for Refloat 1.0.x
- **THEN** it returns the source-faithful high-precision legacy realtime layout while the native backends continue polling command 10

#### Scenario: Complete negotiated dynamic response

- **WHEN** a supported Refloat 1.1.x or 1.2.x controller advertises the complete known always-present and running-only realtime IDs
- **THEN** the backend decodes all known values, state flags, and version-applicable tails in their negotiated order

#### Scenario: Audited negotiated release

- **WHEN** INFO reports one of 1.1.0–1.1.2 or 1.2.0–1.2.3 and command 32 supplies the complete required profile baseline plus bounded additional IDs
- **THEN** discovery accepts the controller under its audited wire profile and retains additional numeric values as bounded extensions

#### Scenario: Audited selectable release

- **WHEN** INFO reports exact stable Refloat 1.3.0
- **THEN** discovery accepts the selectable profile without command-32 negotiation and begins bounded command-33 polling

#### Scenario: Incomplete or renamed negotiated profile

- **WHEN** a Refloat 1.1.x, 1.2.x, or unknown package responds to command 32 without one or more required semantic IDs
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

### Requirement: Serialized route-correct polling

Every detailed telemetry request SHALL use the one selected verified direct-or-CAN controller route and the single serialized VESC request stream. Negotiated command 31 SHALL use the existing adaptive fast/idle cadence for Refloat 1.1.0–1.2.3, while command 10 detail level 4 becomes due once per second for those dynamic releases. Refloat 1.3.0 command 33 SHALL use the same adaptive cadence for its canonical fast mask and a once-per-second cadence for its canonical slow mask; a once-per-second command-10 mode-0 request SHALL serve only as the retained active firmware-fault probe. Slow and fault requests SHALL occupy the next available request slot without a parallel lane or queue, and a timeout SHALL NOT trigger an immediate retry. Reconnect or controller change SHALL clear route-scoped fast, slow, fault, and merge state; commands 31/32 releases SHALL renegotiate their layout, while exact 1.3.0 SHALL restart its fixed audited masks without command 32.

#### Scenario: Negotiated dynamic and compact telemetry

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

The product SHALL expose source-available raw actual, requested, directional, filtered, booster, and field-weakening/d-axis current independently in amperes and SHALL NOT derive torque or current utilization. Refloat 1.3.0's signed command-33 `motor_id` SHALL populate the field-weakening/d-axis current value. Its compact booster byte represents normalized torque and MUST NOT populate `boosterCurrentA`; because command 33 supplies no booster current, that field SHALL remain null for 1.3.0. Refloat 1.2.x and 1.3.0 fatal-latch state SHALL remain distinct from active firmware-fault code, transient pushback, and alert-reason state. Session history SHALL contain only transitions observed by Float View, SHALL NOT persist the live-only fatal latch, and SHALL NOT claim to be command 35 device history.

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
