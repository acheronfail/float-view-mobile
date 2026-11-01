## MODIFIED Requirements

### Requirement: Exact supported compatibility families

The backends SHALL retain audited VESC and Refloat profiles and permit validated public read-only Refloat fallback on supported VESC controllers. Unknown Refloat labels alone SHALL NOT reject a usable public read protocol or grant mutation authority. Existing identity, route and centralized mutation safety checks SHALL remain mandatory; stable releases SHALL NOT revoke audited beta support.

#### Scenario: Audited VESC core profile

- **WHEN** a direct or CAN-routed controller reports stable VESC 6.02, 6.05, 6.06, or 7.00 with the complete source-defined identity baseline
- **THEN** the backend selects `vescCoreReadOnly`, validates the route and Refloat capability, and can reach `ready` without assuming support for unconsumed VESC commands

#### Scenario: Stable identity variants

- **WHEN** VESC 6.02 omits the hardware-CRC suffix or VESC 6.05, 6.06, or 7.00 supplies the bounded suffix
- **THEN** both shapes decode through the same profile and any CRC remains non-public and is not used as a compatibility key

#### Scenario: Audited test build permits guarded writes

- **WHEN** an audited 6.02/6.05/6.06/7.00 controller reports a nonzero firmware test-version marker
- **THEN** the backend reaches `ready` after route and Refloat validation, allows read operations, and permits audited Refloat control operations through the same capability and safety checks as release firmware

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
- **THEN** the backend selects command 33 and the exact stable control profile without sending command 32 or granting audited mutation semantics to another 1.3 release

#### Scenario: Audited Refloat 1.3.1-beta2 session

- **WHEN** direct or CAN INFO reports numeric 1.3.1 with the complete bounded `beta2` suffix
- **THEN** the backend reports `1.3.1-beta2`, selects the existing selectable telemetry and stable control families, and preserves the full identity in session provenance

#### Scenario: Neighboring release rejected

- **WHEN** an unknown release fails the validated public read protocol or INFO has a malformed suffix
- **THEN** the backend rejects it without inferring beta2 compatibility from numeric 1.3.1 or recognizable command IDs
- **AND** valid unknown identities with a successful public read probe instead receive limited support, while audited beta3 keeps its verified profile

#### Scenario: Unsupported identity

- **WHEN** VESC identity is unsupported (including VESC 7.01 test builds), or Refloat identity/protocol cannot satisfy audited or limited admission
- **THEN** the backend reports the appropriate failure without creating a session or treating recognizable command IDs as sufficient evidence

#### Scenario: Flywheel write authorization

- **WHEN** authoritative package mode is flywheel and a caller requests a package, lighting, fatal, or other device mutation
- **THEN** centralized authorization emits no request, while an eligible flywheel-disable transition may emit only command 22

#### Scenario: Audited VESC shutdown profile

- **WHEN** a ready writable controller with audited Refloat mutation semantics reports stable VESC 6.05, 6.06, or 7.00 and authoritative ride state is idle outside flywheel mode
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

### Requirement: Actionable Refloat compatibility failures

A confirmed unusable Refloat identity or read protocol SHALL expose bounded display version metadata including its reported suffix and the exact verified source-release list. The connection SHALL remain failed without automatic retries, telemetry or mutations. Retry and disconnect SHALL remain explicit actions, and a new connection generation SHALL clear prior failure metadata.

#### Scenario: Unsupported package

- **WHEN** a controller reports Refloat 1.3.1-beta but fails public read admission
- **THEN** the current complete snapshot retains that display version and the audited verified release list without admitting the package or creating a session

#### Scenario: Missing or malformed INFO

- **WHEN** INFO times out or cannot be decoded safely
- **THEN** the connection reports a retryable identification failure without claiming a detected unsupported version

#### Scenario: New connection generation

- **WHEN** the user retries or disconnects after compatibility failure
- **THEN** old version metadata is cleared and only current-generation replies can establish the next outcome

### Requirement: Refloat compatibility recovery presentation

The shared frontend SHALL show a specific unsupported-protocol explanation naming the board and reported package, offline verified-release guidance, secondary Check again, and primary Back to boards. The header SHALL identify unsupported Refloat and the generic diagnostics prompt SHALL be absent for this confirmed outcome. Pointer and keyboard actions SHALL activate once.

#### Scenario: Unsupported version recovery

- **WHEN** an unsupported-version snapshot arrives or the frontend is recreated
- **THEN** the dialog explains how a supported package or compatible Float View build resolves the issue and View verified releases reveals the audited catalog without disconnecting or claiming it lists every connectable version

#### Scenario: Check again

- **WHEN** Check again is activated
- **THEN** one explicit connection attempt starts in the current tracking mode and a repeated failure presents current details again

#### Scenario: Return to boards

- **WHEN** Back to boards, Escape, native Back, or the backdrop dismisses the dialog
- **THEN** the app disconnects and returns to the board list, preserving visible feedback if disconnect fails

#### Scenario: Ready limited connection

- **WHEN** an unknown release passes public read admission
- **THEN** no failed-connection dialog or mandatory update prompt appears; a persistent nonmodal notice describes available readings/recording and unavailable board changes
- **AND** configuration and alert availability are presented from authoritative state without repeated confirmation on each connect

## ADDED Requirements

### Requirement: Preserved audited compatibility semantics

Audited profiles SHALL retain their existing VESC identity, telemetry, control and submission semantics independently of the additional limited read path.

#### Scenario: Audited VESC semantics

- **WHEN** the backend selects an audited compatibility profile
- **THEN** the backends support audited VESC firmware 6.02, 6.05, 6.06, and 7.00 through an exact compatibility catalog mapped to one `vescCoreReadOnly` profile for the VESC-level commands Float View consumes, an exact shutdown capability for 6.05, 6.06, and 7.00, and exact Refloat control subprofiles for normal firmware. A VESC controller SHALL be supported only when `COMM_FW_VERSION` contains the complete bounded identity baseline, reports `HW_TYPE_VESC`, matches an exact catalog entry, and the selected route subsequently proves compatible Refloat INFO and release-selected telemetry behavior. Both zero and nonzero test-version markers on an audited version select read/write access. Unrecognised VESC and Refloat versions SHALL remain ineligible for writes. Validated limited Refloat support does not broaden VESC admission. Read operations SHALL remain allowed on compatible connections, while Refloat writes and shutdown SHALL pass a centralized capability and safety check before encoding. A bounded four-byte hardware-CRC suffix MAY be absent or present according to the audited release fixture, but its value, the hardware or firmware display name, and the controller UUID MUST NOT broaden compatibility or shutdown availability.

#### Scenario: Audited Refloat semantics

- **WHEN** the backend selects an audited compatibility profile
- **THEN** the backends support the wire families represented by every audited Refloat source package from 1.0.0 through 1.3.0 plus exact 1.3.1-beta2 and 1.3.1-beta3 through explicit tested compatibility mappings. Refloat 1.0.x SHALL use its fixed legacy telemetry protocol, legacy lighting command 202, and shared package/flywheel controls and SHALL be detected and reported as wire version `1.0` because its INFO response does not contain a patch version. Refloat 1.1.x SHALL use its dynamic commands 31/32 protocol, legacy lighting command 202, and shared package/flywheel controls. Refloat 1.2.x SHALL use its dynamic commands 31/32 protocol with bounded alert/fault data, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36. Exact Refloat 1.3.0 and audited 1.3.1-beta2/beta3 SHALL use documented selectable command 33 without command-32 layout negotiation, stable lighting command 20, shared package/flywheel controls, and clear-fatal command 36. Beta2 and beta3 SHALL additionally recognize the IMU-timeout alert without broadening other releases.

#### Scenario: Audited mutation submission

- **WHEN** the backend selects an audited compatibility profile
- **THEN** the connection-scoped device mutation coordinator is the only path that may submit a Refloat write or controller shutdown and SHALL reject every mutating command while authoritative package mode is flywheel except an eligible command 22 request that disables flywheel. This mode check MUST occur immediately before scheduler submission so stale frontend state or a queued request cannot bypass it.

### Requirement: Bounded unknown Refloat read admission

An unknown Refloat release SHALL receive limited support only after bounded valid INFO v2 and a successful public core telemetry probe on an already verified direct or CAN route. Admission SHALL preserve its observed version and precede readiness, saved session activation and recording. Existing VESC admission SHALL remain unchanged.

#### Scenario: Future version uses an existing public read interface

- **WHEN** a supported VESC controller reports a valid unknown Refloat version such as beta4, stable 1.3.1 or 1.4.0 and passes the public core probe
- **THEN** it reaches ready with limited support and its actual bounded version/suffix, without a release registration or an alias to an audited version
- **AND** connecting and app-side recording are available while board mutations remain unavailable

#### Scenario: Candidate is not yet a usable connection

- **WHEN** valid INFO is received but the core probe is pending, malformed or times out
- **THEN** no ready state, saved session activation or durable sample is created before successful validation, and terminal failure uses the existing bounded recovery path

#### Scenario: Route or generation changes

- **WHEN** the selected route changes or a connection reconnects during or after limited admission
- **THEN** prior replies and capabilities cannot authorize the new generation and its own bounded probe is required

#### Scenario: Stable release does not retire working betas

- **WHEN** stable 1.3.1 becomes available or gains an audited profile
- **THEN** existing beta2/beta3 mappings retain their audited capabilities and exact provenance; shared implementation/evidence cleanup does not remove those boards' access
