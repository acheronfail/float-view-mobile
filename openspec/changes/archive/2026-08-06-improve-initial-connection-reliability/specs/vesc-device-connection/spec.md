## ADDED Requirements

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

Native backends SHALL make bounded development diagnostics available for BLE setup and controller identification with stable stage, attempt, outcome, final issue-code, and optional canonical VESC UUID context. These diagnostics MUST NOT contain Bluetooth addresses, Apple peripheral UUIDs, launch tokens, raw packet contents, or telemetry values. Routine connection diagnostics SHALL NOT be emitted to the console by production or non-debuggable application builds.

#### Scenario: Identification timeout in a development build

- **WHEN** a development or debuggable build exhausts initial identification recovery
- **THEN** its console diagnostics identify the BLE setup completion, direct-firmware attempts and outcomes, and final bounded failure code without private identifiers

#### Scenario: Same failure in a production build

- **WHEN** an equivalent routine connection failure occurs in a production or non-debuggable build
- **THEN** the backend still publishes normal authoritative failure state but emits no routine connection diagnostic to the console
