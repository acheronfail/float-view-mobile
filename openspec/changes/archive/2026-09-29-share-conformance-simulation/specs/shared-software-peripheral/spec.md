## Purpose

Provide one independent software peripheral for every backend's conformance run, with explicit test selection and bounded isolated simulation sessions.

## ADDED Requirements

### Requirement: One independent software simulation owner

Every software host SHALL obtain protocol responses and simulated device behavior from conformance-owned models. Host adapters SHALL relay byte/event operations without implementing VESC decoding, configuration state, response encoding or product assertions. Expected protocol bytes MUST remain independently checked against immutable evidence.

#### Scenario: Equivalent host transcript

- **WHEN** direct Rust, macOS, iOS simulator and Android emulator execute the same configured request transcript
- **THEN** each observes equivalent ordered responses and device state without host-specific protocol implementations

#### Scenario: Fragmented or malformed traffic

- **WHEN** writes contain partial frames, several frames, noise or invalid CRC/length data
- **THEN** the shared simulator applies the canonical bounded framing behavior and never relies on the production decoder for its expected result

### Requirement: Explicit isolated test selection

Simulation SHALL require an authorized conformance build and explicit run credentials. Ordinary and physical builds MUST NOT expose the test transport or activate simulation from device type, user frontend input or stale configuration. Relay access SHALL be loopback-only and authenticated with a per-run credential separate from the application launch token.

#### Scenario: Production or physical launch

- **WHEN** an ordinary or physical build receives simulation configuration
- **THEN** simulation is unavailable and the real OS Bluetooth path remains selected

#### Scenario: Missing or unauthorized harness

- **WHEN** a software test cannot authenticate or reach its configured fixture
- **THEN** it fails boundedly without falling back to canned responses or real Bluetooth

### Requirement: Bounded ordered simulation lifecycle

The harness SHALL isolate at most four concurrent host sessions with at most 64 pending commands, 128 queued events and 65,536 queued payload bytes each. An encoded message SHALL be at most 65,536 bytes and an individual byte payload at most 8,192 bytes. Commands and teardown SHALL finish or fail within five seconds; polls SHALL return within one second. Admission checks SHALL precede allocation, and close/cancel SHALL remain available under saturation.

#### Scenario: Capacity or deadline exceeded

- **WHEN** input exceeds an exact limit or a command cannot complete before its deadline
- **THEN** the affected session fails explicitly without partial mutation, unbounded buffering or silent loss, and other sessions remain isolated

#### Scenario: Reconnect and delayed events

- **WHEN** a connection is replaced or a duplicate operation or old event arrives
- **THEN** session/generation/correlation checks prevent replay or delivery to the new connection while retaining scenario-defined device state

#### Scenario: View replacement and new test run

- **WHEN** a frontend is recreated during a run and a separate new test run is subsequently launched
- **THEN** view recreation preserves the current simulated connection, while the new run starts with reset buffers, configuration and credentials

#### Scenario: Harness exits during recording

- **WHEN** the relay terminates with commands in flight
- **THEN** the software host reports connection loss and the harness reports failure without claiming physical BLE coverage or replaying accepted writes
