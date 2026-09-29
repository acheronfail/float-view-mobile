## MODIFIED Requirements

### Requirement: Software and hardware execution

The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Every change SHALL run the software gate. Extended physical suites SHALL be required only when the affected OS adapters, permissions, lifecycle/background behavior, radio timing, or firmware/wire interoperability cannot be established by software coverage. Each change SHALL record why physical testing is required or skipped; software/simulator evidence SHALL NOT be presented as physical evidence. Hardware scenarios SHALL exercise the production Bluetooth, protocol, application-service, and storage paths selected by each platform host. Component parity checks MAY exercise a candidate Rust implementation in the fast software gate but MUST NOT present that component as a supported backend or production path before host integration.

#### Scenario: Fast software gate

- **WHEN** `just verify` runs without physical hardware
- **THEN** the frontend, mock, native code, deterministic software conformance, and every retained Rust component's locked formatting, warnings-denied linting, unit tests, and parity tests are verified

#### Scenario: Rust candidate parity

- **WHEN** the VESC framing crate is present without an Apple or Android host bridge
- **THEN** the software gate compares its valid, malformed, fragmented, noisy, multiple-frame, CRC, and size-bound behavior with immutable source-derived evidence without adding a backend target adapter

#### Scenario: Physical BLE gate

- **WHEN** a hardware conformance command runs with a configured ESP32 fixture
- **THEN** the same relevant contract scenarios drive the backend through its real OS Bluetooth stack and selected production implementations

#### Scenario: Production path

- **WHEN** physical conformance launches a Debug application with a test-only endpoint seam
- **THEN** Bluetooth, VESC protocol, application-service, session, and SQLite behavior use the implementations selected for that host's production build rather than a test-only candidate

#### Scenario: Shared product or frontend change

- **WHEN** a change affects shared product policy, persistence, or frontend behavior without changing an OS or hardware interaction that needs physical evidence
- **THEN** passing the software gate and recording the scope-based physical-test skip satisfies verification

#### Scenario: Hardware-sensitive change

- **WHEN** a change affects behavior whose correctness depends on the real OS/device/firmware interaction
- **THEN** the change identifies and runs the relevant physical matrix, and reports unavailable hardware or unperformed checks explicitly
