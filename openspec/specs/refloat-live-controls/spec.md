# refloat-live-controls Specification

## Purpose

TBD - created by archiving change add-refloat-live-controls. Update Purpose after archive.

## Requirements

### Requirement: Exact audited Refloat control profiles

The native backends and conformance simulator SHALL implement only the product control commands audited for stable Refloat 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3. Package enablement SHALL use source command 12 with a disabled boolean and SHALL be treated as a persistent package configuration write. Flywheel SHALL use source command 22 with Refloat's default calibration parameters and footpad abort enabled. Refloat 1.0.x and 1.1.x lighting SHALL use command 202 with its one-byte mask and flags, while Refloat 1.2.x lighting SHALL use command 20 with its four-byte mask and one-byte flags. Fatal-error clearing SHALL use command 36 and SHALL be available only for audited Refloat 1.2.x. Every other Refloat mutation, tuning, recording, history, Handtest, remote, RC-move, or motor-control command MUST remain unsupported.

#### Scenario: Legacy lighting control

- **WHEN** LEDs or headlights are read or changed on an audited Refloat 1.0.x or 1.1.x package
- **THEN** the backend sends route-correct command 202 with the exact legacy mask shape and accepts only its bounded lighting-state response

#### Scenario: Stable lighting control

- **WHEN** LEDs or headlights are read or changed on an audited Refloat 1.2.x package
- **THEN** the backend sends route-correct command 20 with the exact stable mask shape and accepts only its bounded lighting-state response

#### Scenario: Package enablement

- **WHEN** an eligible user requests the opposite Refloat package enabled state
- **THEN** the backend sends command 12 with the corresponding disabled value and completes only after later telemetry confirms the requested package state

#### Scenario: Flywheel transition

- **WHEN** an eligible user requests flywheel on or off
- **THEN** the backend sends command 22 with the audited default payload and completes only after later telemetry confirms the requested package mode

#### Scenario: Version-limited fatal clearing

- **WHEN** an audited Refloat 1.2.x package has a clearable fatal latch
- **THEN** the backend sends command 36 value 1 and completes only after later telemetry reports that the latch is clear

#### Scenario: Excluded mutation

- **WHEN** any caller or fixture peer attempts a Refloat command outside the exact product-control and read allowlists
- **THEN** no unsupported command is sent or simulated, board state is unchanged, and one bounded diagnostic is recorded

### Requirement: Native safety and access gates

The native backend SHALL authorize each Refloat control through one connection-scoped device mutation coordinator against the current connection generation, selected route, exact compatibility profile, firmware access mode, authoritative control state, latest board state, and pending operation immediately before scheduler submission. The coordinator SHALL be the sole authority for Refloat writes and controller shutdown; no control owner may retain a parallel lock or direct write path. All controls require a ready read/write connection. Package disablement and flywheel enablement additionally require an idle board; package disablement requires normal package mode; flywheel enablement requires ready package state and normal package mode. Flywheel disablement requires authoritative flywheel mode but MUST remain available while the package run state is active and normalized as riding because upstream command 22 performs `state_flywheel_off` followed by `flywheel_stop`. While authoritative package mode is flywheel, every device mutation, including controller shutdown, MUST be rejected except that exact command 22 off transition. Fatal clearing requires a latched fatal error with no active firmware fault and non-flywheel package mode. Lighting changes MAY occur while riding when lighting capability is present but MUST NOT occur in flywheel mode. A rejected action MUST send no board command and MUST leave authoritative state unchanged apart from a bounded error or cleared stale pending marker.

#### Scenario: Read-only firmware

- **WHEN** any Refloat control is requested on an audited test-version controller
- **THEN** the backend returns `invalid_state`, sends no mutating packet, and preserves the current control state

#### Scenario: Disable while riding

- **WHEN** package disablement is requested while the latest authoritative ride state is not exactly idle
- **THEN** the backend returns `invalid_state` without sending command 12

#### Scenario: Flywheel enable precondition failure

- **WHEN** flywheel enablement is requested while Refloat is not idle and ready in normal mode
- **THEN** the backend returns `invalid_state` without sending command 22

#### Scenario: Active flywheel disable

- **WHEN** exact flywheel disable is requested while authoritative package mode is flywheel and its run state is normalized as riding
- **THEN** the backend sends only command 22 off and waits for telemetry to confirm normal mode

#### Scenario: Flywheel-mode mutation exclusion

- **WHEN** package enablement, lighting, fatal clearing, or any other device mutation is requested while authoritative package mode is flywheel
- **THEN** the backend returns `invalid_state` and sends no mutating packet, while an eligible request to disable flywheel may send only command 22

#### Scenario: Active fatal fault

- **WHEN** fatal clearing is requested while the underlying firmware fault remains active
- **THEN** the backend returns `invalid_state`, keeps the fatal latch, and sends no command 36

#### Scenario: Riding lighting change

- **WHEN** a ready read/write connection in non-flywheel package mode with lighting capability requests a new LED or headlight state while riding
- **THEN** the backend permits the runtime lighting action without changing package configuration or ride persistence

#### Scenario: Shutdown competes with Refloat mutation

- **WHEN** controller shutdown is pending and a Refloat action arrives, or a Refloat action is pending and shutdown arrives
- **THEN** the shared mutation coordinator returns `invalid_state` for the competing action and emits no second mutation

### Requirement: Serialized route-correct control lifecycle

Every Refloat control write, lighting status request, confirmation probe, and controller shutdown SHALL use the selected controller route exactly once through the connection-wide bounded VESC request scheduler. At most one frontend mutation action MAY be pending. Due riding telemetry SHALL retain priority over control-state refresh work, while an accepted user action SHALL be processed ahead of auxiliary discovery and background control refresh. Lighting status SHALL be queried on ready entry, after reconnect, after a lighting action, and at a bounded low cadence while lighting is available. One-way package, flywheel, and fatal commands SHALL be confirmed through subsequent scheduled telemetry rather than by fabricating an acknowledgement; shutdown SHALL report only scheduler acceptance and then follow its bounded connection-loss lifecycle. Timeout, malformed response, disconnect, route replacement, or generation change SHALL terminate the matching action, clear its pending marker, and MUST NOT allow a stale response to mutate the replacement connection.

#### Scenario: Direct control

- **WHEN** an accepted control targets a direct controller
- **THEN** the scheduler sends the exact unwrapped Refloat command and associates any response or confirmation with the current action

#### Scenario: CAN-routed control

- **WHEN** an accepted control targets a selected CAN controller
- **THEN** VESC routing wraps the exact Refloat command once for that CAN ID and the response is attributed only to that route and generation

#### Scenario: Competing action

- **WHEN** a second control action arrives while another control action is pending
- **THEN** the backend returns `invalid_state` without growing a second control queue or sending the second command

#### Scenario: Unconfirmed one-way command

- **WHEN** a package, flywheel, or fatal command is sent but telemetry does not confirm it within the bounded deadline
- **THEN** the request returns `unavailable`, authoritative state is not optimistically changed, and normal telemetry scheduling continues

#### Scenario: Reconnect during action

- **WHEN** the connection generation changes while a control action is pending
- **THEN** the old action terminates, Refloat control state returns to loading, and the new generation re-establishes capability and state before accepting another action

### Requirement: Authoritative live-only control state

Native services SHALL derive and publish complete Refloat control state independently of WebView lifetime. Package and flywheel values SHALL come from normalized package telemetry; lighting availability SHALL come from Refloat INFO and current lighting values from command 20 or 202 responses; Refloat 1.2.x fatal-latch and active-fault values SHALL remain distinct. Beginning and ending an action, accepting a confirmed value, entering reconnect, and losing availability SHALL each publish a newer complete state snapshot. Control state SHALL remain live-only and MUST NOT be written into Board Profiles, session samples, session events, exports, or Tune storage.

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated while the native board connection remains ready
- **THEN** `state.get` restores the complete current Refloat controls and any pending action without requiring the frontend to rediscover device protocol state

#### Scenario: External lighting change

- **WHEN** Refloat lighting changes outside Float View and a later bounded lighting refresh succeeds
- **THEN** the backend publishes the returned LED and headlight values in a newer authoritative snapshot

#### Scenario: Fatal latch without active fault

- **WHEN** Refloat 1.2.x telemetry reports a fatal latch after the underlying firmware fault has ended
- **THEN** state reports the latch as true, the active fault as false, and fatal clearing as clearable

#### Scenario: Disconnect

- **WHEN** the active connection is removed
- **THEN** its Refloat control state and pending action are removed without persisting them into board or session history

### Requirement: Coordinated semantic and full-configuration controls

Semantic package, flywheel, lighting, and fatal-error controls SHALL retain their dedicated exact-version commands and authoritative confirmation behavior when full Refloat configuration is available. They SHALL use the same native mutation coordinator as configuration write and Tune apply, SHALL invalidate or refresh current configuration values when a confirmed semantic action changes a serialized configuration field, and MUST NOT be reimplemented as frontend edits to the generated form. While flywheel mode is active the coordinator SHALL reject every device mutation except the exact eligible command 22 transition that disables flywheel. Entering flywheel SHALL invalidate loaded current configuration values; they MUST remain unavailable until normal mode is confirmed and a fresh command-93 read succeeds. A configuration write SHALL not optimistically alter semantic control state; immediate scheduled response/telemetry refresh SHALL re-establish any affected package or lighting state.

#### Scenario: Package state changes outside editor

- **WHEN** a confirmed semantic package action changes a value represented by the loaded configuration schema
- **THEN** the native owner marks current configuration values stale and requires a fresh command-93 read before a full write

#### Scenario: Full write affects live controls

- **WHEN** a verified full configuration changes package or lighting-related values
- **THEN** the native owner schedules the applicable authoritative telemetry or lighting refresh and updates semantic state only from that source rather than from the submitted draft

#### Scenario: Independent lighting runtime state

- **WHEN** runtime lighting state is not part of the persistent schema value being edited
- **THEN** the dedicated lighting control continues independently while respecting the shared mutation coordinator, and the UI distinguishes current runtime state from the persistent startup default

#### Scenario: Flywheel protects temporary configuration

- **WHEN** telemetry confirms entry into flywheel mode
- **THEN** all loaded current configuration values become stale and every mutation except eligible flywheel disable is rejected until normal mode and a fresh current read are confirmed
