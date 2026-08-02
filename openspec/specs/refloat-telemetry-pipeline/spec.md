## Purpose

Define the source-faithful Refloat telemetry protocol, normalization, bounded extensions, polling, and durable lifecycle shared by native backends and the conformance simulator.

## Requirements

### Requirement: Supported detailed Refloat read protocol

The backends and conformance simulator SHALL implement source-faithful read-only telemetry for audited stable Refloat source packages 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3. The simulator SHALL encode the fixed-layout Refloat 1.0.x command 1 realtime response for retained legacy compatibility; native polling SHALL use command 10 for 1.0.x. Compact command 10 SHALL expose every supported detail level through level 4, including controller state, footpad values, control setpoints, field-weakening current when available, distance, temperatures, odometer, energy totals, battery level, charging values, and active firmware faults. Dynamic commands 31/32 SHALL negotiate and decode the complete always-present and running-only field lists plus package state, mode, flags, stop condition, setpoint adjustment, alert reason, charging tail, and the 1.2.x active-alert tail. Command 35 history and data-recorder commands 41–43 SHALL NOT be requested or simulated by this change.

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

- **WHEN** the fixture receives a Refloat tuning, configuration, lighting, flywheel, motor-control, or other unsupported write command
- **THEN** it returns no simulated success, records one bounded diagnostic, and does not mutate board state

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
- **THEN** the export contains stable typed names, bounded extension values, nulls for unavailable fields, and no private Bluetooth or controller identifier

### Requirement: Raw load and active fault semantics

The product SHALL expose raw actual, requested, directional, filtered, booster, and field-weakening current independently in amperes and SHALL NOT derive torque or current utilization. Refloat 1.2.x active firmware-fault state SHALL remain distinct from transient pushback and alert-reason state. Session history SHALL contain only transitions observed by Float View and SHALL NOT claim to be command 35 device history.

#### Scenario: High current

- **WHEN** actual, requested, filtered, and field-weakening currents are present
- **THEN** each is retained under its own stable field without a utilization ratio

#### Scenario: Firmware fault and pushback

- **WHEN** duty pushback is active without a VESC firmware fault
- **THEN** the sample reports duty setpoint adjustment and its alert reason without fabricating an active firmware fault
