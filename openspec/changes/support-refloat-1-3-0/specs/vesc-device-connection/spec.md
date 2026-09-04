## MODIFIED Requirements

### Requirement: Exact supported compatibility families

The backends SHALL support audited VESC firmware 6.02, 6.05, 6.06, and 7.00 through an exact compatibility catalog mapped to one `vescCoreReadOnly` profile for the VESC-level commands Float View consumes, an exact shutdown capability for 6.05, 6.06, and 7.00, and exact Refloat control subprofiles for normal firmware. A VESC controller SHALL be supported only when `COMM_FW_VERSION` contains the complete bounded identity baseline, reports `HW_TYPE_VESC`, matches an exact catalog entry, and the selected route subsequently proves compatible Refloat INFO and release-selected telemetry behavior. A zero test-version marker selects normal read/write access; a nonzero marker on an audited version selects read-only access. Read operations SHALL remain allowed in either mode, while Refloat writes and shutdown SHALL pass a centralized capability and safety check before encoding. A bounded four-byte hardware-CRC suffix MAY be absent or present according to the audited release fixture, but its value, the hardware or firmware display name, and the controller UUID MUST NOT broaden compatibility or shutdown availability.

The backends SHALL support the wire families represented by every audited Refloat source package from 1.0.0 through 1.3.0 through explicit tested compatibility mappings. Refloat 1.0.x SHALL use its fixed legacy telemetry protocol, legacy lighting command 202, and shared package/flywheel controls and SHALL be detected and reported as wire version `1.0` because its INFO response does not contain a patch version. Refloat 1.1.x SHALL use its dynamic commands 31/32 protocol, legacy lighting command 202, and shared package/flywheel controls. Refloat 1.2.x SHALL use its dynamic commands 31/32 protocol with bounded alert/fault data, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36. Exact Refloat 1.3.0 SHALL use documented selectable command 33 without command-32 layout negotiation, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36.

The connection-scoped device mutation coordinator SHALL be the only path that may submit a Refloat write or controller shutdown and SHALL reject every mutating command while authoritative package mode is flywheel except an eligible command 22 request that disables flywheel. This mode check MUST occur immediately before scheduler submission so stale frontend state or a queued request cannot bypass it.

#### Scenario: Audited VESC core profile

- **WHEN** a direct or CAN-routed controller reports stable VESC 6.02, 6.05, 6.06, or 7.00 with the complete source-defined identity baseline
- **THEN** the backend selects `vescCoreReadOnly`, validates the route and Refloat capability, and can reach `ready` without assuming support for unconsumed VESC commands

#### Scenario: Stable identity variants

- **WHEN** VESC 6.02 omits the hardware-CRC suffix or VESC 6.05, 6.06, or 7.00 supplies the bounded suffix
- **THEN** both shapes decode through the same profile and any CRC remains non-public and is not used as a compatibility key

#### Scenario: Audited test build is read-only

- **WHEN** an audited 6.02/6.05/6.06/7.00 controller reports a nonzero firmware test-version marker
- **THEN** the backend reaches `ready` after route and Refloat validation, allows read operations, and rejects Refloat control operations with `invalid_state`

#### Scenario: Malformed VESC identity

- **WHEN** a controller reports an unterminated name, a truncated UUID or flag baseline, or an unexpected unbounded remainder
- **THEN** discovery fails closed without requesting Refloat telemetry or creating a session

#### Scenario: Legacy Refloat session

- **WHEN** a direct or CAN-routed controller returns legacy Refloat INFO wire byte `10` from an audited source-package-1.0.x protocol family
- **THEN** the backend reports package version `1.0`, uses the fixed legacy telemetry and legacy lighting profile, and reaches `ready` without requesting dynamic layouts or claiming that the device reported patch `.0`

#### Scenario: Negotiated dynamic Refloat sessions

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
