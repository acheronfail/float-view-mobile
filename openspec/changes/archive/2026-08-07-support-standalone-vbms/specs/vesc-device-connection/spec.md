## ADDED Requirements

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
