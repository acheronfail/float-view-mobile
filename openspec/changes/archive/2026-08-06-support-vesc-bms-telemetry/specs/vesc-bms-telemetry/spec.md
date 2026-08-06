## ADDED Requirements

### Requirement: Audited auxiliary-node compatibility discovery

The native backend SHALL classify every structurally valid local and probed CAN firmware identity and retain BMS candidates only when they match the exact audited compatibility catalog: stock `HW_TYPE_VESC_BMS` version 6.00, or hardware name `VBMS32`, `HW_TYPE_CUSTOM_MODULE`, version 6.06. Arbitrary custom modules and unaudited versions MUST NOT become BMS candidates. One generation SHALL remain bounded to 64 normalized CAN IDs and 16 BMS candidates. BMS discovery or failure MUST NOT delay controller readiness, prevent session creation, interrupt controller polling, or change controller continuity decisions.

#### Scenario: Direct motor controller with CAN BMS

- **WHEN** the BLE endpoint identifies as `HW_TYPE_VESC` and its CAN network contains one exact supported stock VESC BMS or VBMS32 node
- **THEN** the controller can become ready directly while a bounded auxiliary pass discovers the CAN-routed VBMS independently

#### Scenario: Direct VBMS gateway

- **WHEN** the BLE endpoint identifies as an exact supported stock VESC BMS or VBMS32 and its CAN network contains one supported motor controller
- **THEN** the backend retains a direct BMS candidate and continues controller discovery through CAN without treating the VBMS as the motor controller

#### Scenario: Reused complete inventory

- **WHEN** controller discovery already probed every normalized CAN ID returned by the current generation's CAN ping
- **THEN** auxiliary discovery reuses those classified identities and does not repeat the complete ping and firmware pass

#### Scenario: Incomplete controller path

- **WHEN** a direct, preferred, or privately hinted controller route reaches readiness without a complete CAN inventory
- **THEN** auxiliary discovery performs its own low-priority bounded ping and sequential firmware pass

#### Scenario: Auxiliary discovery overflow

- **WHEN** normalized CAN IDs or identified VBMS nodes exceed their respective bounds
- **THEN** BMS state becomes unavailable with a bounded diagnostic, no partial BMS route is selected, and the controller connection continues unchanged

### Requirement: Deterministic BMS route selection

The backend SHALL select a directly connected local VBMS, an exact configured CAN route, or the sole automatically discovered CAN VBMS. Without a direct local VBMS or `preferredBmsCanId`, zero candidates SHALL produce `not_found`, one SHALL be selected, and more than one SHALL produce `ambiguous` with no values polling. The backend MUST NOT automatically select by CAN ordering or controller-cached BMS data.

#### Scenario: Unique CAN BMS

- **WHEN** automatic auxiliary discovery identifies exactly one valid CAN-routed VBMS
- **THEN** the backend selects that route and directly requests its values

#### Scenario: Multiple unconfigured BMS nodes

- **WHEN** automatic auxiliary discovery identifies two or more valid VBMS nodes and `preferredBmsCanId` is null
- **THEN** state becomes `ambiguous`, bounded display-safe candidates are published, and no BMS values request is sent to any candidate

#### Scenario: Preferred BMS omitted by ping

- **WHEN** `preferredBmsCanId` is `42`, the local endpoint is not a VBMS, and CAN ping is empty or omits ID `42`
- **THEN** the backend probes CAN route `42` directly and uses it only if it reports a valid VBMS identity

#### Scenario: Preferred BMS unavailable

- **WHEN** the configured BMS route times out, returns malformed firmware, or does not match an exact supported BMS compatibility entry
- **THEN** BMS state becomes `unavailable` and the backend does not fall back to another discovered BMS or affect controller readiness

#### Scenario: Active preference change

- **WHEN** the current Board Profile changes `preferredBmsCanId` while the controller connection is active
- **THEN** the backend invalidates only the auxiliary generation, probes the new exact route or resumes automatic discovery when cleared, and leaves the controller route and active session intact

### Requirement: Hardware-specific firmware identity parsing

The firmware decoder SHALL parse one bounded common identity prefix and apply exact wire-layout tails. A motor-controller or VESC Express custom-module identity SHALL retain its audited flags, firmware-name, and optional-CRC requirements; a supported stock VESC BMS identity SHALL end after its common prefix and custom-configuration count; and a malformed or unexpected remainder MUST NOT be classified as a valid BMS. Auxiliary discovery SHALL apply the exact compatibility catalog after structural decoding.

#### Scenario: Source-faithful VBMS identity

- **WHEN** firmware version 6.00 reports a bounded hardware name, 12-byte UUID, pairing marker, test version, `HW_TYPE_VESC_BMS`, and custom-configuration count with no remainder
- **THEN** the decoder classifies it as a VBMS without requiring motor-only flags or a firmware name

#### Scenario: Source-faithful VBMS32 identity

- **WHEN** firmware version 6.06 reports hardware name `VBMS32`, `HW_TYPE_CUSTOM_MODULE`, the VESC Express capability bytes, terminated firmware name, and hardware CRC tail
- **THEN** the decoder structurally accepts the custom-module identity and auxiliary discovery classifies only that exact audited tuple as a BMS

#### Scenario: Unlisted custom module or version

- **WHEN** a structurally valid custom module has another hardware name or version, including VBMS32 version 7.00
- **THEN** it remains usable only for generic gateway inventory and is not selected or polled as a BMS

#### Scenario: Motor identity remains strict

- **WHEN** a motor-controller identity omits its required flags or terminated firmware name
- **THEN** the decoder rejects it even though its common prefix is parseable

#### Scenario: Malformed BMS tail

- **WHEN** a purported VBMS identity contains a truncated common prefix or unexpected trailing bytes
- **THEN** it is not retained as a BMS candidate and a bounded diagnostic is recorded

#### Scenario: Private UUID boundary

- **WHEN** the backend decodes a VBMS UUID
- **THEN** the UUID remains native-private and is absent from frontend state, ordinary logs, errors, diagnostics, and exports

### Requirement: Bounded direct BMS values decoding

The backend SHALL request `COMM_BMS_GET_VALUES = 96` on the selected BMS route and publish a snapshot only after decoding one complete source-defined legacy or extended response. It SHALL bound cell and temperature counts to 50 each, balancing flags to booleans, status content to 40 bytes, all arrays and remainders exactly, and every numeric result to a finite value.

#### Scenario: Direct request

- **WHEN** the selected VBMS is the local BLE endpoint
- **THEN** the backend sends payload `[96]` in the normal VESC frame

#### Scenario: CAN-forwarded request

- **WHEN** the selected VBMS is on CAN ID `42`
- **THEN** the backend sends payload `[34, 42, 96]` in the normal VESC frame and accepts only a response reporting CAN ID `42`

#### Scenario: Complete legacy response

- **WHEN** a command-96 response contains `c` cells, `t` temperature sensors, and exactly `58 + 3c + 2t` bytes including the command
- **THEN** the backend publishes all fixed values with nullable data version and status

#### Scenario: Complete extended response

- **WHEN** the fixed response is followed by one data-version byte and one bounded NUL-terminated status with no remainder
- **THEN** the backend publishes both optional fields with the same complete snapshot

#### Scenario: Malformed variable data

- **WHEN** a response reports more than 50 cells or temperatures, lacks bytes implied by a count, contains a balancing byte other than zero or one, has a non-finite decoded value, mismatches its selected CAN route, has an unterminated or oversized status, or leaves another remainder
- **THEN** no partial snapshot is published and the failure contributes to bounded BMS unavailability

### Requirement: Serialized subordinate BMS polling

All discovery and polling commands SHALL use one connection-wide bounded VESC request scheduler with at most one request in flight and explicit expected-response context. Controller identification and due Refloat telemetry SHALL have priority over auxiliary discovery and approximately one-hertz BMS polling.

#### Scenario: Riding telemetry is due

- **WHEN** a BMS poll and riding Refloat telemetry are both eligible
- **THEN** controller telemetry is sent first and the BMS poll waits for a later free request slot

#### Scenario: Firmware probe attribution

- **WHEN** sequential controller or BMS firmware probes traverse multiple CAN routes
- **THEN** each response is attributed only to the route recorded by its matching in-flight request and never to a stale global CAN ID

#### Scenario: BMS request timeout

- **WHEN** a selected BMS values request reaches its deadline
- **THEN** the request completes as failed, the serialized queue can advance, and controller polling continues

#### Scenario: BMS recovery

- **WHEN** three consecutive BMS polls time out or return malformed responses and a later poll returns a valid complete response
- **THEN** status first becomes `unavailable` while retaining the last snapshot and then recovers to `attached` with the newer snapshot

### Requirement: Independent BMS lifecycle and read-only safety

BMS status SHALL be one of `discovering`, `not_found`, `attached`, `unavailable`, or `ambiguous`. A valid direct response in the current connection generation SHALL establish `attached`; identity alone MUST NOT. Reconnect SHALL revalidate BMS identity and liveness without controlling session continuity, and Float View MUST send no BMS mutation command.

#### Scenario: Identity found before values

- **WHEN** a unique VBMS identity is detected but its first direct values request has not succeeded
- **THEN** BMS state remains non-attached and controller operation continues

#### Scenario: No BMS present

- **WHEN** bounded discovery completes without a VBMS identity
- **THEN** BMS status is `not_found` and the connection remains ready if controller requirements are satisfied

#### Scenario: Reconnect with missing BMS

- **WHEN** an active ride reconnects to the same controller but the prior BMS cannot be revalidated
- **THEN** the ride session resumes according to controller continuity while BMS status becomes unavailable and its last snapshot remains explicitly stale

#### Scenario: Active connection removed

- **WHEN** the active BLE device is explicitly disconnected, removed, or replaced
- **THEN** all live BMS candidates, route, status, and snapshot for that device are cleared

#### Scenario: Unsafe BMS request

- **WHEN** Float View is connected to a VBMS
- **THEN** it sends only identity, discovery, forwarding, and values-read commands required by this capability and never sends balance, reset, charge-control, configuration, bootloader, or firmware-write commands
