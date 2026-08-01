## ADDED Requirements

### Requirement: Detailed live telemetry contract

`LiveSample` SHALL expose one stable typed nullable shape for the detailed telemetry catalog across the mock, Apple, and Android backends. In addition to existing core values it SHALL represent motion and IMU values, rider inputs, requested and filtered control values, ATR and tilt contributions, package state and mode, stop and setpoint-adjustment state, wheelslip, darkride, charging, alert reason, field-weakening current, accumulated values, battery level, active firmware fault, one nullable ISO-8601 `slowObservedAt`, and a bounded extension map. The existing numeric `boardState` SHALL be superseded by `packageState`, and numeric `footpadState` SHALL become the stable footpad enum; raw numeric codes MAY remain only in bounded diagnostics. `LiveSample` SHALL NOT expose a derived torque or current-utilization value.

#### Scenario: Complete state snapshot

- **WHEN** detailed telemetry is available for the active connection
- **THEN** `state.get` and the next `state.changed` contain the same latest committed typed values, nulls for unavailable fields, and bounded extensions

#### Scenario: Cross-platform shape

- **WHEN** Apple and Android decode semantically equivalent responses
- **THEN** their authenticated JSON uses the same field names, enum values, units, nullability, and bounds

### Requirement: Stable detailed telemetry catalog

The typed catalog SHALL contain:

- motion fields `speedKph`, `erpm`, `pitchDeg`, `balancePitchDeg`, and `rollDeg`;
- electrical fields `motorCurrentA`, `directionalMotorCurrentA`, `filteredMotorCurrentA`, `batteryCurrentA`, `requestedBalanceCurrentA`, `boosterCurrentA`, `fieldWeakeningCurrentA`, `dutyCycle`, `packVoltageV`, `controllerTempC`, and `motorTempC`;
- rider-input fields `footpadState`, `footpadAdc1V`, `footpadAdc2V`, and `remoteInput`;
- control fields `setpointDeg`, `atrSetpointDeg`, `brakeTiltSetpointDeg`, `torqueTiltSetpointDeg`, `turnTiltSetpointDeg`, `remoteSetpointDeg`, `atrAccelerationDifferenceErpmPerSecond`, and `atrSpeedBoost`;
- state fields `packageState`, `packageMode`, `stopCondition`, `setpointAdjustment`, `wheelslip`, `darkride`, `charging`, `alertReason`, and `firmwareFault`; and
- accumulated/charging fields `distanceM`, `odometerM`, `ampHoursDrawnAh`, `ampHoursChargedAh`, `wattHoursDrawnWh`, `wattHoursChargedWh`, `batteryLevel`, `chargingCurrentA`, and `chargingVoltageV`.

`speedKph` SHALL use kilometres/hour; angular fields degrees; current fields amperes; voltage fields volts; temperatures degrees Celsius; distances metres; energy watt-hours; charge amp-hours; duty, battery level, ATR speed boost, and remote input decimal ratios; and ATR acceleration difference electrical RPM/second.

#### Scenario: Catalog serialization

- **WHEN** every catalog field is populated with a finite protocol-valid value
- **THEN** the mock, Apple, and Android serialize the exact stable names and units without aliases or platform-only fields

### Requirement: Stable detailed telemetry enums and units

Public detailed state values SHALL use bounded stable string enums with an `unknown` representation where forward compatibility requires it. `packageState` SHALL cover `disabled`, `startup`, `ready`, and `running`; `packageMode` SHALL cover `normal`, `handTest`, and `flywheel`; `footpadState` SHALL cover `none`, `left`, `right`, and `both`; `stopCondition` SHALL cover `none`, `pitch`, `roll`, `switchHalf`, `switchFull`, `reverseStop`, and `quickStop`; and `setpointAdjustment` SHALL cover `none`, `centering`, `reverseStop`, `speed`, `duty`, `error`, `highVoltage`, `lowVoltage`, and `temperature`. Alert reasons and firmware faults SHALL use the source-fixture allowlists plus `unknown`. Numeric values SHALL use the catalog units consistently in live samples, session series, diagnostics, and exports. Raw upstream numeric state codes MAY be retained only in bounded diagnostics and MUST NOT replace the stable public meaning.

#### Scenario: Unknown future state

- **WHEN** a valid response contains an unrecognized package state, mode, stop condition, setpoint adjustment, or alert reason
- **THEN** the contract reports `unknown` for that semantic value without rejecting otherwise valid bounded telemetry

#### Scenario: Angular and electrical units

- **WHEN** pitch, setpoints, currents, voltage, energy, distance, and field-weakening data are serialized
- **THEN** every consumer observes the contract-defined degrees, amperes, volts, watt-hours, amp-hours, and metres without platform-specific conversion

### Requirement: Live and historical telemetry symmetry

Every typed detailed field accepted into a durable session SHALL be retrievable through session detail and diagnostics under the same stable name and unit used by the live sample. Session exports SHALL include the same semantic fields and bounded extensions. `SessionDetail` SHALL contain numeric `series`, bounded typed `events`, and `eventsTruncated`. Each event SHALL contain `elapsedMs`, a `kind` from `packageState`, `packageMode`, `footpadState`, `stopCondition`, `setpointAdjustment`, `wheelslip`, `darkride`, `charging`, `alertReason`, or `firmwareFault`, and the new stable string `value`. Boolean transitions SHALL use `active` and `cleared`; a nullable reason or fault clearing SHALL use `none`. Events SHALL be appended only when an observed discrete value changes. When more than 2,000 events exist, session detail SHALL return the most recent 2,000 in ascending order and set `eventsTruncated` true. Downsampling MAY reduce numeric points but MUST NOT reinterpret values, merge incompatible state categories, or fabricate device history from before Float View observed the controller.

#### Scenario: Review a simulated warning

- **WHEN** a duty-pushback sample is persisted and the session is later opened
- **THEN** historical series and export identify the same duty value, setpoint-adjustment state, and alert reason shown live

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated after detailed samples were committed
- **THEN** state and session requests recover authoritative native values without depending on browser memory

#### Scenario: Review a discrete transition

- **WHEN** wheelslip becomes active and later clears during a session
- **THEN** session detail contains one `wheelslip=active` event and one `wheelslip=cleared` event alongside the bounded numeric series

#### Scenario: Event response is truncated

- **WHEN** a session contains more than 2,000 observed transitions
- **THEN** session detail returns the newest 2,000 in chronological order and reports `eventsTruncated=true`

### Requirement: One controller per telemetry session

Live telemetry, transition events, durable samples, summaries, diagnostics, and exports SHALL belong to exactly one selected direct-or-CAN controller route at a time. Discovery MAY expose multiple candidates. Selecting or validating a different controller SHALL end the existing session and clear route-scoped telemetry before values from the new controller are accepted.

#### Scenario: Selected controller changes

- **WHEN** the active selection changes from a direct controller to a CAN-routed controller
- **THEN** the prior session ends before the first new-controller sample and no session detail or export combines values from both controllers

### Requirement: Bounded telemetry serialization

Detailed live samples, extension keys and counts, session-detail responses, and diagnostics windows SHALL retain explicit bounds. Authenticated messages SHALL remain at most 65,536 bytes. A negotiated field name SHALL be at most 64 UTF-8 bytes, an extension map SHALL contain at most 32 values, serialized typed detail and extension objects SHALL each be at most 16,384 UTF-8 bytes, diagnostics SHALL contain at most 50 items, and one session-detail response SHALL contain at most 2,000 series points and 2,000 transition events. Private controller UUIDs, Bluetooth identifiers beyond the existing device contract, launch tokens, and full unrequested telemetry MUST NOT appear in samples, extensions, errors, or default logs.

#### Scenario: Oversized extension data

- **WHEN** native telemetry contains extension data beyond the contract bound
- **THEN** the backend applies the specified deterministic rejection or omission rule and emits valid bounded contract state

#### Scenario: Privacy inspection

- **WHEN** live state, session detail, diagnostics, and exports are inspected
- **THEN** no private learned or simulated controller UUID is present
