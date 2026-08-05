## MODIFIED Requirements

### Requirement: Exact supported compatibility families

The backends SHALL support audited VESC firmware 6.02, 6.05, 6.06, and 7.00 through an exact compatibility catalog mapped to one `vescCoreReadOnly` profile for the VESC-level commands Float View consumes and exact Refloat control subprofiles for normal firmware. A VESC controller SHALL be supported only when `COMM_FW_VERSION` contains the complete bounded identity baseline, reports `HW_TYPE_VESC`, matches an exact catalog entry, and the selected route subsequently proves compatible Refloat INFO and layout behavior. A zero test-version marker selects normal read/write access; a nonzero marker on an audited version selects read-only access. Read operations SHALL remain allowed in either mode, while Refloat writes SHALL pass a centralized capability and safety check before version-specific encoding. A bounded four-byte hardware-CRC suffix MAY be absent or present according to the audited release fixture, but its value, the hardware or firmware display name, and the controller UUID MUST NOT broaden compatibility.

The backends SHALL support the wire families represented by every audited Refloat source package from 1.0.0 through 1.2.3 through explicit tested compatibility mappings. Refloat 1.0.x SHALL use its fixed legacy telemetry protocol, legacy lighting command 202, and shared package/flywheel controls and SHALL be detected and reported as wire version `1.0` because its INFO response does not contain a patch version. Refloat 1.1.x SHALL use its dynamic ID/data protocol, legacy lighting command 202, and shared package/flywheel controls. Refloat 1.2.x SHALL use its dynamic protocol with bounded alert/fault data, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36.

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

#### Scenario: Dynamic Refloat sessions

- **WHEN** a direct or CAN-routed controller reports an audited Refloat 1.1.x or 1.2.x release
- **THEN** the backend negotiates the corresponding dynamic telemetry and exact control profile without inferring support from a broad version range

#### Scenario: Unsupported identity

- **WHEN** controller or package identity does not match an explicit compatibility entry, including VESC 7.01 while it is a test build
- **THEN** the backend reports that identity as unsupported without treating recognizable command IDs as compatible

