## ADDED Requirements

### Requirement: Bounded VESC BLE discovery and stream setup

Each native backend SHALL mirror VESC Tool's saved-device discovery rule: it MAY observe BLE advertisements without a service filter but SHALL publish a device only when the advertisement contains the Nordic UART service UUID or its exact Apple peripheral UUID or Android Bluetooth address already belongs to a saved board. Saved status SHALL bypass only the advertisement check. Every connection SHALL still boundedly discover the required UART service and characteristics, enable notifications, and segment outbound frames to the available BLE write payload. Operation at the default ATT MTU SHALL remain supported; an increased MTU MUST be an optimization rather than a connection prerequisite.

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

### Requirement: Saved identity precedes application readiness

After UART setup, the backend SHALL establish saved-device identity only after a valid local VESC firmware response and SHALL load the Board Profile before choosing a controller route. It MUST NOT create a durable session until route, supported controller identity, Refloat compatibility, and telemetry setup reach application `ready`.

#### Scenario: First valid VESC transport

- **WHEN** a first-time BLE peer supplies a valid local VESC firmware response
- **THEN** the backend creates its saved-device record, publishes `savedDeviceId`, loads the default Board Profile, and continues identification without creating a session

#### Scenario: Arbitrary UART peer

- **WHEN** a BLE peer exposes UART but never supplies a valid local VESC firmware response
- **THEN** the backend fails identification without saving it as a board or creating a session

#### Scenario: Negotiation completes

- **WHEN** controller and telemetry negotiation completes after saved identity is established
- **THEN** application state becomes `ready` and exactly one durable session is created or resumed according to lifecycle rules

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

When discovery finds more than one VESC motor controller, the backend SHALL pause before Refloat negotiation and publish at most 16 candidates sorted by CAN ID. Each candidate SHALL contain CAN ID, hardware/model name when available, firmware name when available, VESC version, and `vescSupported`. It MUST NOT expose firmware UUIDs, hide unsupported motor controllers, or silently select a candidate.

#### Scenario: Ambiguous gateway

- **WHEN** discovery identifies two compatible VESC controllers
- **THEN** connection state becomes `selecting_controller`, both display-safe candidates are published, and no controller reaches telemetry polling until a selection is accepted

#### Scenario: Supported and unsupported controllers

- **WHEN** discovery identifies more than one motor controller and at least one VESC firmware version is unsupported
- **THEN** all controllers remain visible, unsupported candidates are marked and cannot be selected, and the backend does not silently choose a supported candidate

#### Scenario: Candidate overflow

- **WHEN** discovery finds a seventeenth VESC motor controller
- **THEN** the connection fails with `too_many_controllers`, publishes no truncated candidate list, and directs the user to configure a CAN ID

#### Scenario: Valid selection

- **WHEN** the user selects a candidate from the current discovery generation
- **THEN** the backend atomically merges its CAN ID into the latest Board Profile without overwriting other fields, advances the profile revision once, clears the candidates, resumes compatibility negotiation on that route, and can reach `ready`

#### Scenario: Stale or unknown selection

- **WHEN** a selection names an ID outside the current candidates or arrives after disconnect, reconnect, or another selection
- **THEN** the backend returns `invalid_state` without changing the active route or persisted preference

#### Scenario: Malformed selection

- **WHEN** a selection contains a missing, fractional, negative, nonnumeric, or greater-than-253 CAN ID
- **THEN** the backend returns `invalid_request` without changing the active route or persisted preference

#### Scenario: Frontend recreation during selection

- **WHEN** the WebView is recreated while controller selection is required
- **THEN** `state.get` restores the same bounded candidates and selection state while native discovery remains authoritative

### Requirement: Exact supported compatibility families

The backends SHALL support audited VESC firmware 6.02, 6.05, 6.06, and 7.00 through an exact compatibility catalog mapped to one `vescCoreReadOnly` profile for the commands Float View consumes. A VESC controller SHALL be supported only when `COMM_FW_VERSION` contains the complete bounded identity baseline, reports `HW_TYPE_VESC`, matches an exact catalog entry, and the selected route subsequently proves compatible Refloat INFO and layout behavior. A zero test-version marker selects normal access; a nonzero marker on an audited version selects read-only access. Read operations SHALL remain allowed in either mode, while write operations SHALL be rejected by a centralized capability check. A bounded four-byte hardware-CRC suffix MAY be absent or present according to the audited release fixture, but its value, the hardware or firmware display name, and the controller UUID MUST NOT broaden compatibility.

The backends SHALL support the wire families represented by Refloat source packages 1.0.0, 1.1.0, and 1.2.3 through explicit tested compatibility mappings. The 1.0.0 source package SHALL use its fixed legacy telemetry protocol and SHALL be detected and reported as wire version `1.0` because its INFO response does not contain a patch version. Refloat 1.1.0 SHALL use its dynamic ID and data protocol, and Refloat 1.2.3 SHALL use its dynamic protocol with bounded alert and fault data.

#### Scenario: Audited VESC core profile

- **WHEN** a direct or CAN-routed controller reports stable VESC 6.02, 6.05, 6.06, or 7.00 with the complete source-defined identity baseline
- **THEN** the backend selects `vescCoreReadOnly`, validates the route and Refloat capability, and can reach `ready` without assuming support for unconsumed VESC commands

#### Scenario: Stable identity variants

- **WHEN** VESC 6.02 omits the hardware-CRC suffix or VESC 6.05, 6.06, or 7.00 supplies the bounded suffix
- **THEN** both shapes decode through the same profile and any CRC remains non-public and is not used as a compatibility key

#### Scenario: Audited test build is read-only

- **WHEN** an audited 6.02/6.05/6.06/7.00 controller reports a nonzero firmware test-version marker
- **THEN** the backend reaches `ready` after route and Refloat validation, allows read operations, and rejects write operations with `unsupported_operation`

#### Scenario: Malformed VESC identity

- **WHEN** a controller reports an unterminated name, a truncated UUID or flag baseline, or an unexpected unbounded remainder
- **THEN** discovery fails closed without requesting Refloat telemetry or creating a session

#### Scenario: Legacy Refloat session

- **WHEN** a direct or CAN-routed controller returns legacy Refloat INFO wire byte `10` from the source-package-1.0.0 protocol family
- **THEN** the backend reports package version `1.0`, uses the fixed legacy packets, and reaches `ready` without requesting dynamic layouts or claiming that the device reported patch `.0`

#### Scenario: Dynamic Refloat sessions

- **WHEN** a direct or CAN-routed controller reports Refloat 1.1.0 or 1.2.3
- **THEN** the backend negotiates the corresponding dynamic layout and decodes its source-faithful response shape

#### Scenario: Unsupported identity

- **WHEN** controller or package identity does not match an explicit compatibility entry, including VESC 7.01 while it is a test build
- **THEN** the backend reports that identity as unsupported without treating a broad version range as compatible

### Requirement: Route-correct telemetry lifecycle

The backend SHALL serialize discovery and telemetry requests on the VESC stream, start polling only after the selected route and compatibility family are complete, and repeat route validation and layout negotiation after reconnect before resuming samples.

#### Scenario: Direct telemetry

- **WHEN** a supported direct controller completes Refloat negotiation
- **THEN** direct polling produces persisted live samples and an automatic durable session

#### Scenario: CAN telemetry

- **WHEN** a supported CAN controller completes Refloat negotiation
- **THEN** polling remains forwarded to the selected CAN ID and samples are attributed to that saved board

#### Scenario: Reconnect changes discovery generation

- **WHEN** BLE disconnects during identification or telemetry and later reconnects
- **THEN** stale responses from the old generation are ignored and the new generation validates its route before polling resumes

#### Scenario: Reconnect reaches another controller

- **WHEN** reconnect cannot establish that the selected controller is the same controller that owns the active session
- **THEN** the backend closes that session as interrupted, fails with `controller_changed`, and appends no sample from the replacement controller

### Requirement: Private learned controller identity

The backends SHALL parse and persist a meaningful VESC firmware UUID for reconnect validation. When public `preferredCanId` is null and exactly one supported controller is automatically discovered, the backend SHALL persist its CAN ID and UUID as a private automatic hint. A public preference SHALL always take precedence. The UUID MUST NOT appear in frontend contract state, normal logs, exports, controller-selection labels, or errors.

#### Scenario: Remembered controller still matches

- **WHEN** a remembered CAN route reports the same meaningful controller UUID after reconnect
- **THEN** the backend confirms that route without presenting a new ambiguity prompt

#### Scenario: Remembered controller changed

- **WHEN** a private automatic hint reports a different meaningful UUID and no active session requires continuity
- **THEN** the backend invalidates that hint and returns to bounded discovery rather than recording the replacement silently

#### Scenario: Same automatic controller moved

- **WHEN** an active-session reconnect cannot find the privately remembered CAN ID but bounded discovery finds the same meaningful UUID at another CAN ID
- **THEN** the backend updates the private hint, re-establishes telemetry on the new route, and resumes the same session without prompting

#### Scenario: Preferred controller changed

- **WHEN** a public preferred route reports a different meaningful UUID
- **THEN** the backend fails with `controller_changed` and does not fall back to discovery

#### Scenario: Public preference edited

- **WHEN** the user changes or clears `preferredCanId` through Board Profile
- **THEN** the backend invalidates the UUID associated with the old public route and stores a new meaningful UUID only after the replacement preferred route succeeds

#### Scenario: UUID unavailable

- **WHEN** supported firmware omits a meaningful UUID
- **THEN** identification continues using route, hardware type, and supported firmware identity without fabricating or exposing a UUID

### Requirement: Recoverable connection failure

An asynchronous identification failure SHALL retain a failed active connection with a bounded stable `connectionIssue`, saved identity when established, and already verified route and firmware information. Polling and new session creation MUST remain stopped. `device.connect` for the same failed `deviceId` SHALL start a new generation using the latest Board Profile; a different active device SHALL remain invalid.

#### Scenario: Preferred CAN ID corrected

- **WHEN** a failed connection's Board Profile is updated and `device.connect` is requested again for the same device
- **THEN** the backend clears the old issue, starts a new bounded generation, and uses the updated preference

#### Scenario: Different device while failed

- **WHEN** another device is requested while a failed active connection remains
- **THEN** the backend returns `invalid_state` and preserves the failed connection

#### Scenario: Disconnect from failure or selection

- **WHEN** the user disconnects while identification, controller selection, or failed state is active
- **THEN** the backend cancels that generation and removes the active connection through normal disconnect state changes
