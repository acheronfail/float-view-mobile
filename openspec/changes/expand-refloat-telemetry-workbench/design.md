## Context

The current fixture-development application composes one direct or Express-connected THOR300 with VESC 6.02, 6.05, or 6.06 and representative Refloat source packages. Its VESC catalog does not yet include stable 7.00, does not preserve the stable/test-build distinction carried by `COMM_FW_VERSION`, and models the source-defined hardware-CRC suffix only for 6.06 even though the audited 6.05 release also carries it. Its interactive telemetry source generates only the core values currently consumed by Float View, and its dynamic codec advertises eight fields even though audited Refloat 1.1.x and 1.2.x releases define sixteen always-present and ten running-only realtime fields. The legacy compact encoder and both native decoders likewise normalize only a core subset.

The native pipeline already has the important foundations: explicit direct/CAN routing, one serialized VESC stream, exact Refloat compatibility families, native durable sessions, bounded authenticated state, a generic session-series representation, and a native `session_event` store. Apple also has a native extension-field storage concept. This change extends those foundations; it does not create another transport, simulator, frontend, or generated API layer.

The upstream source tags are authoritative for wire behavior. Stable 1.0.0 and 1.0.1 share the legacy `1.0` wire identity; command 1 is their fixed-layout high-precision realtime response and command 10 is their compact, levelled all-data response. Stable 1.1.0–1.1.2 and 1.2.0–1.2.3 use dynamic commands 31 and 32 for negotiated realtime values and detailed state, while a lower-rate command 10 supplies field-weakening current and accumulated values. The 1.2.x profile appends mask-gated active alert and firmware-fault state. Command 35 device history is deliberately deferred.

This change is sequenced after `support-direct-and-can-vesc-connections`. It assumes that change's verified-route, discovery, fixture-topology, and controller-continuity behavior rather than redefining it.

## Goals / Non-Goals

**Goals:**

- Make `just fixture dev` an editable, shareable manual workbench for reported topologies and semantic board states.
- Encode and decode all useful read-only telemetry supplied by supported Refloat source packages.
- Normalize, persist, retrieve, export, and publish the same detailed telemetry on Apple and Android.
- Give the shared frontend stable typed data for detailed live and historical experiences.
- Poll, publish, and log exactly one selected direct-or-CAN controller at a time.
- Retain exact route semantics, native lifecycle ownership, bounded resources, privacy, and source-faithful version differences.
- Compose every audited Refloat release with stable VESC 6.02/6.05/6.06/7.00 through one strict source-faithful controller identity family.
- Make future source fields diagnosable through a bounded extension path without making upstream strings the primary product contract.

**Non-Goals:**

- Automated product control or pass/fail execution from fixture-development.
- BLE service, characteristic, CCCD, MTU, multiple-advertiser, or OS-suspension simulation in the serial byte bridge.
- Refloat tuning, configuration, lights, flywheel, motor movement, or other write workflows.
- Supporting prerelease, unknown, or stable package versions outside 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3.
- Supporting VESC test builds outside the audited catalog, unknown controller releases, 7.01 while its test-version marker is nonzero, or VESC firmware older than 6.02. Audited test-version builds are read-only and expose that access mode in the shared firmware details.
- Deriving torque or current utilization or obtaining controller current-limit configuration.
- Refloat command 35 device history, data-recorder commands 41–43, or multi-controller telemetry sessions.
- Adding a database migration framework or runtime dependency.

## Decisions

### 1. Use one semantic board-state model with version-specific projections

`conformance/fixture/` will own a `SimulatedBoardState` independent of Refloat packet layout. It groups:

```text
motion       speed, ERPM, pitch, balance pitch, roll, distance
electrical   motor/directional/filtered/battery/requested/booster currents,
             duty, voltage, temperatures, field-weakening current
rider input  footpad state and ADCs, remote input
control      final, ATR, brake, torque, turn, and remote setpoints,
             ATR acceleration difference and speed boost
state        package state/mode, stop, setpoint adjustment, wheelslip,
             darkride, charging, alert reason, firmware fault
accumulated  odometer, amp-hours, watt-hours, battery level, charge values
```

Each Refloat codec projects only values available in its source package and requested command. Refloat 1.0.x uses the exact legacy layouts. Refloat 1.1.x and 1.2.x advertise the complete source-derived ID lists and encode values in negotiated order; 1.2.x adds its mask-gated alert tail. Unavailable values remain absent.

The stable product catalog is:

| Group | Contract fields | Unit or type |
| --- | --- | --- |
| Motion | `speedKph`, `erpm`, `pitchDeg`, `balancePitchDeg`, `rollDeg` | kilometres/hour, electrical RPM, degrees |
| Electrical | `motorCurrentA`, `directionalMotorCurrentA`, `filteredMotorCurrentA`, `batteryCurrentA`, `requestedBalanceCurrentA`, `boosterCurrentA`, `fieldWeakeningCurrentA`, `dutyCycle`, `packVoltageV`, `controllerTempC`, `motorTempC` | amperes, decimal ratio, volts, degrees Celsius |
| Rider input | `footpadState`, `footpadAdc1V`, `footpadAdc2V`, `remoteInput` | stable enum, volts, decimal input in `[-1, 1]` |
| Control | `setpointDeg`, `atrSetpointDeg`, `brakeTiltSetpointDeg`, `torqueTiltSetpointDeg`, `turnTiltSetpointDeg`, `remoteSetpointDeg`, `atrAccelerationDifferenceErpmPerSecond`, `atrSpeedBoost` | degrees, electrical RPM/second, decimal ratio |
| State | `packageState`, `packageMode`, `stopCondition`, `setpointAdjustment`, `wheelslip`, `darkride`, `charging`, `alertReason`, `firmwareFault` | stable enums, booleans, or nullable active fault |
| Accumulated and charging | `distanceM`, `odometerM`, `ampHoursDrawnAh`, `ampHoursChargedAh`, `wattHoursDrawnWh`, `wattHoursChargedWh`, `batteryLevel`, `chargingCurrentA`, `chargingVoltageV` | metres, amp-hours, watt-hours, decimal ratio, amperes, volts |

Stable state enums cover the source-defined meanings and `unknown`: package state `disabled`, `startup`, `ready`, and `running`; package mode `normal`, `handTest`, and `flywheel`; footpad state `none`, `left`, `right`, and `both`; stop condition `none`, `pitch`, `roll`, `switchHalf`, `switchFull`, `reverseStop`, and `quickStop`; and setpoint adjustment `none`, `centering`, `reverseStop`, `speed`, `duty`, `error`, `highVoltage`, `lowVoltage`, and `temperature`. The source tag fixtures define the allowlisted alert-reason and VESC firmware-fault mappings. A source family that does not carry a catalog field publishes null rather than a substitute.

Encoding packet-shaped state directly in Svelte or the coordinator was rejected because it would duplicate protocol logic and make a shared reported scenario behave differently across versions.

### 2. Select audited controller and package wire profiles and validate capabilities

VESC major/minor identifies a candidate catalog entry rather than proving compatibility by itself. Audited 6.02, 6.05, 6.06, and 7.00 map to one `vescCoreReadOnly` profile because every command Float View consumes retains its identity and required response semantics across those releases. The firmware decoder requires the complete bounded `COMM_FW_VERSION` baseline, including terminated hardware and firmware names, the 12-byte UUID, the full flag block, and `HW_TYPE_VESC`. A zero test-version marker selects normal access; a nonzero marker on an audited version selects read-only access. The source-defined four-byte hardware-CRC suffix is absent for the audited 6.02 fixture and present for audited 6.05/6.06/7.00 fixtures; it is optional bounded provenance, never a compatibility key. Direct or forwarded Refloat INFO and required layout negotiation provide the final behavioral evidence before `ready`. The loopback firmware object exposes the access mode so the frontend labels test firmware as read-only beside VESC and Refloat versions.

Unknown controller versions and malformed identity baselines remain unsupported. VESC 7.01 is therefore excluded while upstream reports it as a test build. A future stable version joins `vescCoreReadOnly` only after immutable fixtures prove every consumed command and response; a changed consumed command or required response shape creates a new profile. Any future write command must pass the access-mode gate and is denied for test-version firmware.

Package semver identifies a candidate profile; it is not itself a protocol signature. The compatibility catalog admits only audited stable releases and maps them to `legacyFixed`, `dynamic`, or `dynamicAlerts`. The legacy INFO response cannot distinguish 1.0.0 from 1.0.1, so wire identity `1.0` intentionally represents the audited legacy 1.0.x profile.

For a dynamic candidate, discovery requests command 32 and becomes ready only when the bounded layout contains every required always-present and runtime field for that audited profile, without duplicates. Order is negotiated and additional bounded numeric IDs are accepted as extensions. A missing or renamed required ID rejects compatibility even when the packet is structurally decodable. Exact release fixtures prove the fixed command-31 header, flags, units, and version-applicable tail that command 32 does not describe.

Unknown stable versions and all prereleases remain unsupported until their upstream source is audited and canonical fixtures are added. INFO Git hashes may be retained as bounded diagnostic provenance, but are not the compatibility key because compatible custom builds and cherry-picks need capability validation rather than exact-build identity.

Accepting every semver above 1.1 was rejected because command 32 does not describe units, fixed-header semantics, or tail meanings. Requiring an exact field-list order was rejected because the protocol explicitly negotiates order and safely permits bounded additional fields.

### 3. Make fixture-development a workbench, not a runner

The browser contract will select either a common custom configuration or a versioned scenario document. The server resolves public aliases into private deterministic controller UUIDs, owns the full `BoardConfiguration`, and exposes only bounded display-safe summaries. Presets are editable starting templates, not named expectations.

The workbench will provide:

- topology and discovery editing while powered off;
- coherent telemetry scenes, freeze/resume, and per-field pins while active;
- explicit disconnect/re-advertise and serialized identity/CAN-route transitions;
- response-fragment selection;
- resolved field availability and request/response diagnostics.

There will be no expected result, automatic product command, timed product workflow, or pass/fail state. Canonical physical conformance continues to use `FixtureRunner`.

A scenario may contain several candidates so discovery, ambiguity, and selection bugs remain reproducible. Float View nevertheless has one active telemetry route: only the selected controller is polled, published, and appended to the active session. Selecting a different route first interrupts or completes the old session, clears route-scoped telemetry caches, and then starts telemetry for the new controller. No sample, event, summary, or export combines controllers.

Import/export uses a versioned JSON document of at most 65,536 UTF-8 bytes with at most 16 controller candidates and 64 discovered CAN IDs, canonical decimal CAN IDs, finite numeric values, and an allowlist of fields. Controller aliases are at most 64 UTF-8 bytes. The server derives 12 private deterministic bytes using a standard-library digest over the document namespace and alias; those bytes never enter the browser snapshot, document, diagnostics, or default logs. Serial path, launch token, attachment, power, BLE, and ride runtime state are not document fields.

Persisting workbench state or accepting raw UUIDs was rejected because the primary need is explicit reproducibility and sharing, while accidental retention or identifier disclosure adds lifecycle and privacy risk.

### 4. Normalize into a complete typed sample plus bounded extensions

The hand-written contract remains authoritative. `LiveSample` and the native normalized record will add nullable typed fields for all supported semantic values. Public state-like fields use stable string unions with `unknown`; numbers use one documented unit across platforms.

Known upstream string IDs map to typed fields. Unknown valid dynamic field IDs whose transmitted values are numeric may enter an extension map bounded to 32 entries and finite values. A negotiated layout accepts at most 64 fields, each UTF-8 name is at most 64 bytes, and all encoded names together are at most 4,096 bytes. A persisted `detail_json` or `extension_json` value is at most 16,384 UTF-8 bytes, authenticated messages remain within `MAX_MESSAGE_BYTES` (65,536), diagnostics retain at most 50 items, and a session-detail response returns at most 2,000 series points and 2,000 transition events. Duplicate IDs or a layout beyond any bound fail negotiation rather than publishing a truncated layout.

`detail_json` contains catalog fields with contract-owned names and units that are not first-class query or summary columns. `extension_json` contains only bounded unknown negotiated numeric fields and is not a product-workflow contract. Existing core values used for summaries remain columns. The existing `session_event.details_json` is unrelated event payload storage. Known and extension JSON are validated separately so an upstream field cannot become a stable contract accidentally.

Using only a generic upstream-name map was rejected because product warnings and displays need stable semantics, units, nullability, and cross-version mappings. Enumerating only fixed typed fields was also rejected because a bounded extension path is valuable for diagnostics and forward-compatible plotting.

### 5. Add one periodic compact request to the existing serialized poller

Dynamic command 31 remains the adaptive fast/idle poll. Legacy 1.0.0 continues to use command 10 as its primary poll. For dynamic packages, command 10 at detail level 4 becomes due once per second and supplies fields absent from command 31, including field-weakening current, accumulated values, and charging values.

There is no multi-lane queue or parallel work. The route-aware polling owner retains one in-flight request. When no request is in flight and the compact deadline has passed, it sends command 10 in the next available slot and then resumes command 31. A timeout is recorded once and is not retried immediately; the next normal one-second deadline is the next opportunity. Responses remain tagged with connection generation, selected route, and expected command. Reconnect or controller change clears the deadline and latest compact cache, revalidates the route, and renegotiates before polling resumes.

Polling compact data at the realtime cadence was rejected due to unnecessary BLE load. A priority queue, parallel requests, retries, or adaptive backoff were rejected because one periodic due time is sufficient for requesting messages from the selected VESC and the protocol has no safe correlation identifier.

### 6. Store one complete normalized record atomically

Both native current schemas will store summary/query fields in columns, additional catalog fields in bounded `detail_json`, and unknown negotiated values in bounded `extension_json`, with the existing sequence, observation time, elapsed time, and session identity.

For dynamic packages, an accepted command 10 response replaces the bounded latest-compact cache and records one `slowObservedAt` timestamp for that group. The next accepted command 31 response merges those latest-known compact values and timestamp into its normalized sample. Values remain null before the first compact response. Device-reported accumulated values are authoritative: a newly reported value replaces the previous value even if it is lower because the device restarted or reset a counter. The backend does not clamp, reconstruct, or reinterpret those counters. Legacy command 10 responses are already complete normalized samples and carry their own observation time.

Each accepted normalized sample gets one sample sequence and one atomic append. When one of `packageState`, `packageMode`, `footpadState`, `stopCondition`, `setpointAdjustment`, `wheelslip`, `darkride`, `charging`, `alertReason`, or `firmwareFault` changes, the same transaction also appends a deduplicated `session_event` containing the elapsed time, stable field-name kind, and stable string value. Events use their existing independent monotonic event sequence; multiple changes caused by one sample are ordered by the catalog order above. `SessionDetail.events` exposes the most recent 2,000 transitions in ascending order and sets `eventsTruncated` when earlier events exist. It does not expose or imply command 35 history from before Float View observed the controller.

Session detail, diagnostics, summaries, and export read the normalized stored representation rather than decoding protocol payloads again. Existing disposable-database handling recreates incompatible schemas. No migrations or dual-read fallback are added.

Storing raw frames as the product record was rejected because it couples history to decoder versions, increases privacy and size risk, and cannot provide the stable contract required by the shared frontend.

### 7. Expose raw load signals without utilization

The normalized model exposes actual motor current, requested balance current, directional and filtered current, booster current, duty, and field-weakening current independently. The fixture supplies coherent high-current and field-weakening scenes in those raw units. It does not carry simulated controller-limit metadata, and neither backend nor frontend calculates a torque or current-utilization percentage.

Utilization is deferred because Refloat realtime does not provide the applicable positive and regenerative controller limits. Adding it later requires a separate contract and authoritative limit source.

### 8. Present detailed telemetry progressively in one shared frontend

The live view will retain its focused summary and add accessible grouped detail for motion, load, temperatures, rider input, control contributions, accumulated values, and semantic warnings. Safety states use text and structure in addition to color. Historical session detail uses the same names and units, with bounded charts or event annotations for discrete state transitions.

Frequently updated disclosures and controls follow the repository Pointer Events rule with guarded keyboard/click activation. The frontend never computes authoritative state transitions from raw values when Refloat already supplies package, stop, setpoint-adjustment, wheelslip, or alert state.

A platform-specific native diagnostics screen was rejected because it would duplicate the product frontend and prevent consistent desktop/touch testing.

### 9. Add read support incrementally and retain a strict write allowlist

The fixture completes the source-faithful Refloat 1.0.x command 1 encoder because the retained fixture capability already promises legacy realtime compatibility. Command 1 is a fixed-layout high-precision control snapshot; it is not added to native polling. Native Refloat 1.0.x polling continues to use the broader command 10 response. Dynamic backends implement commands 31/32 plus the periodic command 10 request.

Command 35 alert history and data-recorder commands 41–43 are out of scope. Active alert and firmware-fault values already present in command 31 remain supported, and Float View records the transitions it observes in its own durable session.

All tuning and actuator commands remain rejected by the fixture and unissued by the backends. Adding them later requires a separate contract, safety, persistence, and conformance design.

## Risks / Trade-offs

- [Detailed samples increase state, database, and export size] → Keep explicit field, extension, point, diagnostic, and export bounds; use existing downsampling and avoid polling slow values at realtime cadence.
- [The prerequisite connection change touches the same owners] → Do not begin implementation until `support-direct-and-can-vesc-connections` is complete; build on its explicit route, discovery, continuity, and fixture-topology contracts.
- [Legacy and dynamic values do not have perfect one-to-one semantics] → Preserve nullability, document mappings and units, and test equivalent semantic fixtures without fabricating absent values.
- [Latest-known compact values can look newer than they are] → Persist and publish the compact group's `slowObservedAt`, clear it with route generation, and never label it with the later realtime observation time.
- [Upstream negotiated IDs can change] → Admit only audited stable releases, require the profile's complete semantic baseline during command-32 negotiation, and retain additional bounded numeric IDs as extensions.
- [A rich workbench UI can become an unsafe raw protocol editor] → Expose semantic controls and allowlisted scenario fields; keep packet encoding, UUIDs, commands, and serial ownership server-side.
- [Detailed warning displays may be mistaken for board control] → Use read-only language and keep unsupported writes rejected throughout the stack.

## Migration Plan

1. Complete, sync, and archive `support-direct-and-can-vesc-connections`.
2. Add the source-tag field catalog, canonical semantic fixtures, and exact version encoders/decoders with no public contract switch.
3. Land the breaking contract, mock, both normalized models, current SQL schemas, stores, serializers, transition events, and canonical conformance as one coordinated vertical slice.
4. Enable the one-second compact request and detailed live snapshots after Apple/Android persistence equivalence passes.
5. Add the fixture workbench controls and shared live/historical presentation.
6. Run focused checks, the detailed software matrix, `just verify`, and one consolidated representative physical BLE checkpoint.
7. Sync specifications and archive after the complete gate. Rollback during test-stage development deletes incompatible disposable databases and reverts the coordinated contract/backend/frontend slice.

## Open Questions

None. Compact values use the latest-reported cache with `slowObservedAt`; utilization and command 35 history are deferred; the product records only one selected controller per session.
