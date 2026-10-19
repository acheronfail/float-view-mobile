# vesc-device-connection Specification

## Purpose

Define bounded BLE discovery, direct and CAN controller identification, exact compatibility selection, route-correct connection lifecycles, and centralized device mutation authorization.

## Requirements

### Requirement: Bounded VESC BLE discovery and stream setup

The shared BLE transport policy selected by every native backend SHALL mirror VESC Tool's saved-device discovery rule: it MAY observe BLE advertisements without a service filter but SHALL publish a device only when the advertisement contains the Nordic UART service UUID or its exact Apple peripheral UUID or Android Bluetooth address already belongs to a saved board. Saved status SHALL bypass only the advertisement check. Every connection SHALL still boundedly discover the required UART service and characteristics, enable notifications, and segment outbound frames to the available BLE write payload. Operation at the default ATT MTU SHALL remain supported; an increased MTU MUST be an optimization rather than a connection prerequisite. Native adapters SHALL supply OS capabilities and execute GATT operations while shared policy owns filtering, write-mode selection, bounded segmentation and ordered write admission. Platform payload restrictions and notification interoperability SHALL be preserved.

#### Scenario: Default ATT MTU

- **WHEN** a compatible VESC exposes only the default BLE write payload
- **THEN** the backend segments complete VESC frames into safe chunks and completes identification and telemetry without requiring a larger MTU

#### Scenario: Setup element unavailable

- **WHEN** the UART service, required characteristic, or required notification capability is unavailable
- **THEN** setup fails within its deadline with bounded diagnostic context and does not enter `ready`

#### Scenario: Saved device omits advertised service

- **WHEN** a previously saved platform identifier advertises without the VESC UART service UUID after process restart or during reconnect
- **THEN** the backend publishes that exact device, may connect by its stored identifier, and still requires successful bounded UART service and characteristic validation

#### Scenario: Unknown device omits advertised service

- **WHEN** an advertisement lacks the UART service UUID and its identifier does not exactly match a saved board
- **THEN** the backend does not publish or connect it

### Requirement: Exact direct VBMS connection classification

After BLE UART setup, a structurally valid direct identity matching stock `HW_TYPE_VESC_BMS` version 6.00 or hardware name `VBMS32`, `HW_TYPE_CUSTOM_MODULE`, version 6.06 SHALL immediately establish a BMS-only connection. The backend MUST NOT run CAN controller discovery, Refloat negotiation, motor configuration, controller polling or controls, location capture, or durable-session work for that connection. All other identities SHALL retain the existing fail-closed controller or gateway path.

#### Scenario: Exact standalone VBMS connects

- **WHEN** the direct endpoint returns an exact supported VBMS firmware identity
- **THEN** the backend reaches `ready` as a BMS device and sends no CAN ping, forwarded controller probe, or Refloat request

#### Scenario: Numeric or custom Bluetooth name

- **WHEN** a peripheral advertises a numeric, default, or user-configured name but returns an exact supported VBMS identity
- **THEN** classification uses the firmware identity and not the advertised name

#### Scenario: Unlisted custom module

- **WHEN** a direct endpoint reports an unaudited version, hardware type, or hardware name
- **THEN** it does not become BMS-only and remains subject to existing bounded controller/gateway discovery

#### Scenario: BMS-only reconnect

- **WHEN** a BMS-only BLE connection is lost and the same device reconnects
- **THEN** the new generation revalidates the exact direct identity and resumes only direct BMS polling without creating a controller session

### Requirement: Saved identity precedes application readiness

After UART setup, the backend SHALL establish saved-device identity only for a connection explicitly chosen in saved mode, and only after a valid local VESC firmware response. In saved mode it SHALL derive or resolve the stable opaque `deviceHistoryId` from the meaningful board-level VESC identity when available, attach the local platform association to an existing matching imported board when possible, and load the Board Profile before choosing a controller route. Raw firmware identifiers SHALL remain native-private. It MUST NOT create a durable session until route, supported controller identity, Refloat compatibility, and telemetry setup reach application `ready`. A guest connection MAY reach `ready` without creating saved identity or loading a durable Board Profile, provided the same bounded identity and compatibility checks succeed.

#### Scenario: First valid saved VESC transport

- **WHEN** a first-time saved-mode BLE peer supplies a valid local VESC firmware response
- **THEN** the backend resolves or creates the saved board by stable opaque logical identity, attaches the local platform association, publishes `savedDeviceId`, loads the default or imported Board Profile, and continues identification without creating a session

#### Scenario: Saved peer matches imported board

- **WHEN** a saved-mode BLE peer produces a stable logical identity already present in an imported board entry
- **THEN** the backend reuses that board, attaches the local platform association, and does not create a duplicate board or split its session history

#### Scenario: Stable identity unavailable

- **WHEN** a valid saved-mode VESC response has no meaningful stable board identity
- **THEN** the backend creates an opaque random logical identity, does not claim an automatic cross-platform physical match, and continues with normal saved-board setup

#### Scenario: First valid guest VESC transport

- **WHEN** a first-time guest BLE peer supplies a valid local VESC firmware response
- **THEN** the backend continues identification without saving the peer as a board or creating a session

#### Scenario: Arbitrary UART peer

- **WHEN** a BLE peer exposes UART but never supplies a valid local VESC firmware response
- **THEN** the backend fails identification without saving it as a board or creating a session in either tracking mode

#### Scenario: Negotiation completes in saved mode

- **WHEN** controller and telemetry negotiation completes after saved identity is established
- **THEN** application state becomes `ready` and exactly one durable session is created or resumed according to lifecycle rules

#### Scenario: Negotiation completes in guest mode

- **WHEN** controller and telemetry negotiation completes for a guest connection
- **THEN** application state becomes `ready` with live telemetry available and no durable session or saved-board identity

### Requirement: Explicit controller route

Every controller-bound firmware, Refloat, layout, telemetry, and adaptive-logging request SHALL use one established route that is either direct or CAN-forwarded to an ID from `0` through `253`. The command encoder SHALL apply CAN forwarding exactly once for a CAN route and SHALL leave direct commands unwrapped.

#### Scenario: Direct controller

- **WHEN** the BLE endpoint firmware identifies itself as a VESC motor controller
- **THEN** the backend establishes a direct route and sends Refloat discovery and telemetry requests without CAN forwarding

#### Scenario: CAN controller

- **WHEN** discovery selects controller CAN ID `42` behind a gateway
- **THEN** every applicable controller request is forwarded exactly once to CAN ID `42`

### Requirement: Gateway controller discovery

When a valid local VESC-protocol endpoint is not a motor controller, the backend SHALL treat it as a potential gateway and prove that capability through a configured forwarded firmware probe or CAN discovery plus forwarded firmware probing. When no CAN preference is configured and no private automatic hint matches, it SHALL perform bounded CAN discovery, normalize at most 64 unique IDs in the supported range, sort them by CAN ID, sequentially probe their firmware, and retain devices whose parseable firmware hardware type is VESC.

#### Scenario: One VESC among mixed CAN devices

- **WHEN** CAN discovery returns a VESC motor controller together with non-VESC nodes
- **THEN** the backend selects the sole VESC candidate and ignores the other nodes as telemetry targets

#### Scenario: No VESC controller

- **WHEN** bounded discovery and firmware probing find no VESC motor controller
- **THEN** the connection fails without choosing an unprobed or non-VESC CAN node

#### Scenario: Duplicate and invalid ping IDs

- **WHEN** a gateway returns duplicate IDs or values outside `0` through `253`
- **THEN** the backend discards invalid values, probes each valid unique ID at most once, and keeps all work bounded

#### Scenario: Excessive CAN IDs

- **WHEN** discovery produces more than 64 normalized unique IDs
- **THEN** the connection fails with `too_many_can_devices`, probes no truncated subset as though it were complete, and directs the user to configure a CAN ID

#### Scenario: Generic forwarding endpoint

- **WHEN** a non-motor VESC-protocol endpoint has a name other than VESC Express but successfully forwards a configured or discovered controller firmware request
- **THEN** the backend treats it as a gateway without relying on its advertised or hardware display name

### Requirement: Configured CAN route

When a gateway-connected saved board has `preferredCanId`, the backend SHALL probe that CAN route directly without requiring CAN ping to advertise it. The preference SHALL be authoritative until changed or cleared and MUST NOT apply when the BLE endpoint is itself a VESC motor controller.

#### Scenario: Ping omits configured controller

- **WHEN** `preferredCanId` is `42`, the BLE endpoint is a gateway, and CAN ping is empty or omits `42`
- **THEN** the backend probes CAN ID `42` directly and can reach `ready` when it is a compatible VESC

#### Scenario: Configured route is wrong

- **WHEN** the configured ID times out or reports non-VESC or incompatible firmware
- **THEN** the connection fails within its deadline with `preferred_controller_unavailable` for an absent/non-VESC target, `unsupported_controller` for unsupported VESC firmware, or `controller_changed` for a stored UUID mismatch, and does not silently select another CAN ID

#### Scenario: Preference on direct connection

- **WHEN** a saved board has a CAN preference but its BLE endpoint identifies as a VESC motor controller
- **THEN** the backend uses the direct route and does not forward controller commands

### Requirement: Multiple-controller selection

When discovery finds more than one VESC motor controller, the backend SHALL pause before Refloat negotiation and publish at most 16 candidates sorted by CAN ID. Each candidate SHALL contain CAN ID, hardware/model name when available, firmware name when available, VESC version, and `vescSupported`. It MUST NOT expose firmware UUIDs, hide unsupported motor controllers, or silently select a candidate. In saved mode, an accepted choice SHALL update the Board Profile; in guest mode, it SHALL update only the active in-memory route.

#### Scenario: Ambiguous gateway

- **WHEN** discovery identifies two compatible VESC controllers
- **THEN** connection state becomes `selecting_controller`, both display-safe candidates are published, and no controller reaches telemetry polling until a selection is accepted

#### Scenario: Supported and unsupported controllers

- **WHEN** discovery identifies more than one motor controller and at least one VESC firmware version is unsupported
- **THEN** all controllers remain visible, unsupported candidates are marked and cannot be selected, and the backend does not silently choose a supported candidate

#### Scenario: Candidate overflow

- **WHEN** discovery finds a seventeenth VESC motor controller
- **THEN** the connection fails with `too_many_controllers`, publishes no truncated candidate list, and directs the user to configure a CAN ID when the connection is saved

#### Scenario: Valid saved selection

- **WHEN** the user selects a candidate from the current saved-mode discovery generation
- **THEN** the backend atomically merges its CAN ID into the latest Board Profile without overwriting other fields, advances the profile revision once, clears the candidates, resumes compatibility negotiation on that route, and can reach `ready`

#### Scenario: Valid guest selection

- **WHEN** the user selects a candidate from the current guest-mode discovery generation
- **THEN** the backend stores the route only for the active connection, clears the candidates, resumes compatibility negotiation, and writes no Board Profile or saved routing hint

#### Scenario: Stale or unknown selection

- **WHEN** a selection names an ID outside the current candidates or arrives after disconnect, reconnect, or another selection
- **THEN** the backend returns `invalid_state` without changing the active route or any persisted preference

#### Scenario: Malformed selection

- **WHEN** a selection contains a missing, fractional, negative, nonnumeric, or greater-than-253 CAN ID
- **THEN** the backend returns `invalid_request` without changing the active route or persisted preference

#### Scenario: Frontend recreation during selection

- **WHEN** the WebView is recreated while controller selection is required
- **THEN** `state.get` restores the same bounded candidates, tracking mode, and selection state while native discovery remains authoritative

### Requirement: Exact supported compatibility families

The backends SHALL support audited VESC firmware 6.02, 6.05, 6.06, and 7.00 through an exact compatibility catalog mapped to one `vescCoreReadOnly` profile for the VESC-level commands Float View consumes, an exact shutdown capability for 6.05, 6.06, and 7.00, and exact Refloat control subprofiles for normal firmware. A VESC controller SHALL be supported only when `COMM_FW_VERSION` contains the complete bounded identity baseline, reports `HW_TYPE_VESC`, matches an exact catalog entry, and the selected route subsequently proves compatible Refloat INFO and release-selected telemetry behavior. Both zero and nonzero test-version markers on an audited version select read/write access. Unrecognised VESC and Refloat versions SHALL remain ineligible for writes. Read operations SHALL remain allowed on compatible connections, while Refloat writes and shutdown SHALL pass a centralized capability and safety check before encoding. A bounded four-byte hardware-CRC suffix MAY be absent or present according to the audited release fixture, but its value, the hardware or firmware display name, and the controller UUID MUST NOT broaden compatibility or shutdown availability.

The backends SHALL support the wire families represented by every audited Refloat source package from 1.0.0 through 1.3.0 through explicit tested compatibility mappings. Refloat 1.0.x SHALL use its fixed legacy telemetry protocol, legacy lighting command 202, and shared package/flywheel controls and SHALL be detected and reported as wire version `1.0` because its INFO response does not contain a patch version. Refloat 1.1.x SHALL use its dynamic commands 31/32 protocol, legacy lighting command 202, and shared package/flywheel controls. Refloat 1.2.x SHALL use its dynamic commands 31/32 protocol with bounded alert/fault data, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36. Exact Refloat 1.3.0 SHALL use documented selectable command 33 without command-32 layout negotiation, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36.

The connection-scoped device mutation coordinator SHALL be the only path that may submit a Refloat write or controller shutdown and SHALL reject every mutating command while authoritative package mode is flywheel except an eligible command 22 request that disables flywheel. This mode check MUST occur immediately before scheduler submission so stale frontend state or a queued request cannot bypass it.

#### Scenario: Audited VESC core profile

- **WHEN** a direct or CAN-routed controller reports stable VESC 6.02, 6.05, 6.06, or 7.00 with the complete source-defined identity baseline
- **THEN** the backend selects `vescCoreReadOnly`, validates the route and Refloat capability, and can reach `ready` without assuming support for unconsumed VESC commands

#### Scenario: Stable identity variants

- **WHEN** VESC 6.02 omits the hardware-CRC suffix or VESC 6.05, 6.06, or 7.00 supplies the bounded suffix
- **THEN** both shapes decode through the same profile and any CRC remains non-public and is not used as a compatibility key

#### Scenario: Audited test build permits guarded writes

- **WHEN** an audited 6.02/6.05/6.06/7.00 controller reports a nonzero firmware test-version marker
- **THEN** the backend reaches `ready` after route and Refloat validation, allows read operations, and permits Refloat control operations through the same capability and safety checks as release firmware

#### Scenario: Malformed VESC identity

- **WHEN** a controller reports an unterminated name, a truncated UUID or flag baseline, or an unexpected unbounded remainder
- **THEN** discovery fails closed without requesting Refloat telemetry or creating a session

#### Scenario: Legacy Refloat session

- **WHEN** a direct or CAN-routed controller returns legacy Refloat INFO wire byte `10` from an audited source-package-1.0.x protocol family
- **THEN** the backend reports package version `1.0`, uses the fixed legacy telemetry and legacy lighting profile, and reaches `ready` without requesting dynamic layouts or claiming that the device reported patch `.0`

#### Scenario: Dynamic Refloat sessions

- **WHEN** a direct or CAN-routed controller reports an audited Refloat 1.1.x or 1.2.x release
- **THEN** the backend negotiates commands 31/32 and the exact control profile without inferring support from a broad version range

#### Scenario: Selectable Refloat 1.3.0 session

- **WHEN** a direct or CAN-routed controller reports exact stable Refloat 1.3.0
- **THEN** the backend selects command 33 and the exact stable control profile without sending command 32 or treating another 1.3 release as compatible

#### Scenario: Unsupported identity

- **WHEN** controller or package identity does not match an explicit compatibility entry, including a Refloat prerelease, Refloat 1.3.1, or VESC 7.01 while it is a test build
- **THEN** the backend reports that identity as unsupported without treating recognizable command IDs as compatible

#### Scenario: Flywheel write authorization

- **WHEN** authoritative package mode is flywheel and a caller requests a package, lighting, fatal, or other device mutation
- **THEN** centralized authorization emits no request, while an eligible flywheel-disable transition may emit only command 22

#### Scenario: Audited VESC shutdown profile

- **WHEN** a ready writable controller reports stable VESC 6.05, 6.06, or 7.00 and authoritative ride state is idle outside flywheel mode
- **THEN** the backend exposes shutdown and may send exactly `[156, 0, 0]` through the selected route without inspecting hardware name

#### Scenario: VESC 6.02 has no shutdown command

- **WHEN** an otherwise supported controller reports VESC 6.02
- **THEN** the backend keeps shutdown unavailable and sends no command 156

#### Scenario: CAN-routed shutdown

- **WHEN** shutdown targets a selected CAN controller
- **THEN** the backend wraps `[156, 0, 0]` exactly once with `COMM_FORWARD_CAN` for that controller and does not emit direct or broadcast `CAN_PACKET_SHUTDOWN`

#### Scenario: Unsafe shutdown variants

- **WHEN** any caller requests forced shutdown, restart, broadcast shutdown, retry, or shutdown while the board is not authoritatively idle
- **THEN** the backend emits no mutation and returns the stable invalid-state or invalid-request error applicable to the malformed or unsafe request

#### Scenario: Shutdown disconnect lifecycle

- **WHEN** the current selected controller or BLE transport disappears after an accepted generation-scoped shutdown request
- **THEN** the backend treats the loss as intentional and suppresses reconnect, while a connection that remains healthy past the bounded intent window continues normally

### Requirement: Route-correct telemetry lifecycle

The backend SHALL serialize discovery and telemetry requests on the VESC stream, start polling only after the selected route and compatibility family are complete, and repeat route validation and layout negotiation after reconnect before resuming samples. Saved mode SHALL feed the durable session pipeline; guest mode SHALL feed only the bounded ephemeral live pipeline.

#### Scenario: Direct saved telemetry

- **WHEN** a supported saved-mode direct controller completes Refloat negotiation
- **THEN** direct polling produces persisted live samples and an automatic durable session

#### Scenario: Direct guest telemetry

- **WHEN** a supported guest-mode direct controller completes Refloat negotiation
- **THEN** direct polling produces live samples without a durable session, location track, or catalog update

#### Scenario: CAN saved telemetry

- **WHEN** a supported saved-mode CAN controller completes Refloat negotiation
- **THEN** polling remains forwarded to the selected CAN ID and samples are attributed to that saved board

#### Scenario: CAN guest telemetry

- **WHEN** a supported guest-mode CAN controller completes Refloat negotiation
- **THEN** polling remains forwarded to the selected CAN ID and samples remain ephemeral to the active guest connection

#### Scenario: Reconnect changes discovery generation

- **WHEN** BLE disconnects during identification or telemetry and later reconnects
- **THEN** stale responses from the old generation are ignored and the new generation validates its route before polling resumes with the retained tracking mode

#### Scenario: Reconnect reaches another controller

- **WHEN** reconnect cannot establish that the selected controller is the same controller that owns an active saved session
- **THEN** the backend closes that session as interrupted, fails with `controller_changed`, and appends no sample from the replacement controller; a guest reconnect uses the same bounded validation without creating history

### Requirement: Private learned controller identity

The backends SHALL parse and persist a meaningful VESC firmware UUID for reconnect validation only for saved connections. When public `preferredCanId` is null and exactly one supported controller is automatically discovered in saved mode, the backend SHALL persist its CAN ID and UUID as a private automatic hint. A public preference SHALL always take precedence. The UUID MUST NOT appear in frontend contract state, normal logs, exports, portable archives, controller-selection labels, or errors. A meaningful board-level firmware identity may feed the versioned one-way digest used for stable `deviceHistoryId`, but the raw value remains native-private and is never the archive identity. Guest connections SHALL never persist or update a learned hint or stable board identity.

#### Scenario: Remembered saved controller still matches

- **WHEN** a remembered CAN route reports the same meaningful controller UUID after reconnect in saved mode
- **THEN** the backend confirms that route without presenting a new ambiguity prompt

#### Scenario: Remembered controller changed

- **WHEN** a private automatic hint reports a different meaningful UUID and no active session requires continuity
- **THEN** the backend invalidates that hint and returns to bounded discovery rather than recording the replacement silently

#### Scenario: Same automatic controller moved

- **WHEN** an active saved-session reconnect cannot find the privately remembered CAN ID but bounded discovery finds the same meaningful UUID at another CAN ID
- **THEN** the backend updates the private hint, re-establishes telemetry on the new route, and resumes the same session without prompting

#### Scenario: Preferred controller changed

- **WHEN** a public preferred route reports a different meaningful UUID
- **THEN** the backend fails with `controller_changed` and does not fall back to discovery

#### Scenario: Public preference edited

- **WHEN** the user changes or clears `preferredCanId` through Board Profile
- **THEN** the backend invalidates the UUID associated with the old public route and stores a new meaningful UUID only after the replacement preferred route succeeds

#### Scenario: Guest controller route

- **WHEN** a guest connection discovers or selects a controller route
- **THEN** the route is held only for that connection generation and no UUID, CAN preference, or automatic hint is persisted

#### Scenario: UUID unavailable

- **WHEN** supported firmware omits a meaningful UUID
- **THEN** identification continues using route, hardware type, and supported firmware identity without fabricating or exposing a UUID

### Requirement: Bounded initial identification recovery

After a native backend has successfully prepared BLE UART notifications, it SHALL recover from one missing response to the initial direct VESC firmware read by issuing that same idempotent read at most one additional time within the current connection and identification generation. The recovery attempt MUST remain within the bounded overall identification deadline, MUST NOT reconnect the BLE transport, and MUST NOT retry incompatible responses, configured-route failures, CAN inventory probes, or mutating commands. A late response MAY satisfy the identical retry, but an old connection generation MUST NOT mutate the new connection.

#### Scenario: First direct firmware response is lost

- **WHEN** BLE UART setup succeeds, the first direct firmware request receives no response within its attempt window, and the same identification generation remains active
- **THEN** the backend sends one additional direct firmware request without reconnecting and can continue to `ready` from its valid response

#### Scenario: Both direct firmware responses are lost

- **WHEN** neither the original nor the one recovery request receives a valid firmware response
- **THEN** identification fails within its existing overall deadline and no third firmware request or durable session is created

#### Scenario: Connection generation changes before recovery

- **WHEN** the first direct firmware request times out after disconnect, reconnect, explicit cancellation, or another identification generation has begun
- **THEN** the backend sends no recovery request and the stale callback cannot alter current state

### Requirement: Development connection diagnostics

Native backends SHALL make privacy-safe development diagnostics available for BLE setup and controller identification with stable stage, attempt, outcome, and final issue-code context. These diagnostics MUST remain bounded and MUST NOT contain Bluetooth addresses, Apple peripheral UUIDs, launch tokens, raw packet contents, private controller UUIDs, or telemetry values. Routine connection diagnostics SHALL NOT be emitted to the console by production or non-debuggable application builds.

#### Scenario: Identification timeout in a development build

- **WHEN** a development or debuggable build exhausts initial identification recovery
- **THEN** its console diagnostics identify the BLE setup completion, direct-firmware attempts and outcomes, and final bounded failure code without private identifiers

#### Scenario: Same failure in a production build

- **WHEN** an equivalent routine connection failure occurs in a production or non-debuggable build
- **THEN** the backend still publishes normal authoritative failure state but emits no routine connection diagnostic to the console

### Requirement: Recoverable connection failure

An asynchronous identification failure SHALL retain a failed active connection with a bounded stable `connectionIssue`, tracking mode, saved identity only when established in saved mode, and already verified route and firmware information. Polling and new session creation MUST remain stopped. `device.connect` for the same failed `deviceId` SHALL start a new generation using the latest Board Profile for saved mode or the retained guest intent for guest mode; a different active device SHALL remain invalid.

#### Scenario: Preferred CAN ID corrected

- **WHEN** a failed saved connection's Board Profile is updated and `device.connect` is requested again for the same device
- **THEN** the backend clears the old issue, starts a new bounded saved generation, and uses the updated preference

#### Scenario: Guest retry

- **WHEN** a failed guest connection is requested again for the same device
- **THEN** the backend clears the old issue, starts a new bounded guest generation, and does not create a saved board or read a persisted guest preference

#### Scenario: Different device while failed

- **WHEN** another device is requested while a failed active connection remains
- **THEN** the backend returns `invalid_state` and preserves the failed connection

#### Scenario: Disconnect from failure or selection

- **WHEN** the user disconnects while identification, controller selection, or failed state is active
- **THEN** the backend cancels that generation and removes the active connection through normal disconnect state changes without persisting guest state

### Requirement: Custom-configuration identity and routing

For audited controller firmware, native identity decoding SHALL retain a bounded non-public custom-configuration count and nullable hardware-configuration CRC when present without exposing either as a compatibility shortcut or public raw identity. The selected supported Refloat controller SHALL require custom configuration index `0` before configuration becomes available. Commands 92–95 SHALL be encoded and decoded by matching Swift/Kotlin VESC protocol owners, routed directly or wrapped exactly once through the selected CAN controller, and submitted only through the connection-wide request scheduler with expected command, route, chunk, operation, and connection-generation context.

#### Scenario: Direct custom configuration

- **WHEN** the selected direct controller reports at least one custom configuration
- **THEN** schema, current/default, and eligible write requests use index `0` without CAN wrapping

#### Scenario: CAN custom configuration

- **WHEN** the selected controller is at CAN ID 42
- **THEN** each command 92–95 request is wrapped exactly once for ID 42 and only its correlated response advances the operation

#### Scenario: No registered configuration

- **WHEN** a supported Refloat identity reports zero custom configurations or index `0` never answers
- **THEN** live telemetry may remain ready while Refloat configuration is unavailable and no guessed schema is used

#### Scenario: Stale chunk response

- **WHEN** a command-92 response belongs to an older route, offset, operation, or connection generation
- **THEN** the scheduler discards it without appending bytes or updating replacement configuration state

### Requirement: Centralized Refloat mutation authorization

The connection service SHALL provide one native mutation coordinator and authorization boundary for semantic Refloat controls, complete custom-configuration writes, and Tune apply. It SHALL evaluate current access mode, exact package profile, route, generation, package/ride state, and pending work immediately before scheduler submission, and no owner may retain a parallel mutation lock or direct write path. Recognised VESC test-version firmware SHALL permit the same guarded mutations as release firmware. Unsupported VESC and package versions SHALL remain ineligible for Refloat mutations. While authoritative package mode is flywheel, the coordinator SHALL reject every device mutation except the exact eligible command 22 transition that disables flywheel.

#### Scenario: Competing semantic control

- **WHEN** a configuration write is verifying readback and a package, flywheel, lighting, or fatal control is requested
- **THEN** the later mutation is rejected without a second pending write

#### Scenario: Competing configuration write

- **WHEN** a semantic Refloat control is pending and a configuration write or Tune apply is requested
- **THEN** the configuration mutation is rejected without changing its current base revision

#### Scenario: Writable test firmware

- **WHEN** audited test-version firmware opens Refloat configuration
- **THEN** schema and current/default reads may succeed and write and Tune apply use the same capability and safety checks as release firmware

#### Scenario: Flywheel mutation authorization

- **WHEN** authoritative package mode is flywheel and any semantic control, command 95 write, or Tune apply is requested
- **THEN** centralized authorization sends no mutation unless the request is the eligible exact command 22 flywheel-disable transition

#### Scenario: Flywheel current read

- **WHEN** authoritative package mode is flywheel and command 93 current values are requested or received late
- **THEN** the request is not submitted or the response is discarded without creating a current revision, while schema and default reads remain eligible
