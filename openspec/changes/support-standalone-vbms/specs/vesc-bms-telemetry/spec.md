## MODIFIED Requirements

### Requirement: Audited auxiliary-node compatibility discovery

The native backend SHALL classify every structurally valid local and probed CAN firmware identity and retain BMS candidates only when they match the exact audited compatibility catalog: stock `HW_TYPE_VESC_BMS` version 6.00, or hardware name `VBMS32`, `HW_TYPE_CUSTOM_MODULE`, version 6.06. An exact local BMS SHALL become the primary BMS-only device immediately and MUST NOT enter auxiliary controller discovery. Arbitrary custom modules and unaudited versions MUST NOT become BMS candidates. Auxiliary generations SHALL remain bounded to 64 normalized CAN IDs and 16 BMS candidates, and auxiliary BMS discovery or failure MUST NOT delay controller readiness, prevent session creation, interrupt controller polling, or change controller continuity decisions.

#### Scenario: Direct motor controller with CAN BMS

- **WHEN** the BLE endpoint identifies as `HW_TYPE_VESC` and its CAN network contains one exact supported stock VESC BMS or VBMS32 node
- **THEN** the controller can become ready directly while a bounded auxiliary pass discovers the CAN-routed VBMS independently

#### Scenario: Direct standalone VBMS

- **WHEN** the BLE endpoint identifies as an exact supported stock VESC BMS or VBMS32
- **THEN** the backend selects it as the direct BMS-only device without searching CAN for a motor controller

#### Scenario: Reused complete inventory

- **WHEN** controller discovery already probed every normalized CAN ID returned by the current generation's CAN ping
- **THEN** auxiliary discovery reuses those classified identities and does not repeat the complete ping and firmware pass

#### Scenario: Incomplete controller path

- **WHEN** a direct, preferred, or privately hinted controller route reaches readiness without a complete CAN inventory
- **THEN** auxiliary discovery performs its own low-priority bounded ping and sequential firmware pass

#### Scenario: Auxiliary discovery overflow

- **WHEN** normalized CAN IDs or identified VBMS nodes exceed their respective bounds
- **THEN** BMS state becomes unavailable with a bounded diagnostic, no partial BMS route is selected, and the controller connection continues unchanged

## ADDED Requirements

### Requirement: Standalone direct BMS polling lifecycle

A BMS-only connection SHALL select the direct BMS route, request bounded read-only command-96 values after reaching ready, replace its live snapshot on valid responses, and retain the existing bounded unavailable/recovery behavior. It MUST NOT persist BMS values or create, resume, or interrupt a ride session.

#### Scenario: Direct values arrive

- **WHEN** a ready BMS-only device returns a valid direct `COMM_BMS_GET_VALUES` response
- **THEN** authoritative state becomes attached with the decoded live snapshot and schedules the next bounded direct poll

#### Scenario: Direct values are unavailable

- **WHEN** the standalone BMS misses the configured bounded response threshold while BLE remains connected
- **THEN** BMS state becomes unavailable while the active BMS connection remains ready and eligible to recover on a later valid response

#### Scenario: Saved BMS-only connection

- **WHEN** an exact direct VBMS is connected with saved tracking intent
- **THEN** the device association may remain saved but no durable session, location track, controller identity, or historical BMS row is created
