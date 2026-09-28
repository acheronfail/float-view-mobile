## MODIFIED Requirements

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
