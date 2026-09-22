## ADDED Requirements

### Requirement: Shared server migration evidence

The shared server change SHALL pass focused Rust and host-boundary tests, canonical mock/macOS/iOS/Android software conformance, clean native packaging, dependency/deletion audits, and `just verify`. Canonical assertions SHALL remain in `conformance` and cover all existing methods and routes. They SHALL cover authentication, malformed and oversized input, response ordering, multiple and slow clients, cancellation, lifecycle recovery, and archive cleanup. Component tests SHALL cover hostile handles, envelopes, generation races, and shutdown bounds.

#### Scenario: Exercise each selected host

- **WHEN** canonical software scenarios run after each host migration
- **THEN** they exercise that host's selected shared server and fail on missing prerequisites or contract drift

#### Scenario: Verify platform packaging

- **WHEN** clean Apple host/device/simulator and Android host/four-ABI Debug/Release builds run
- **THEN** the required shared server symbols and dependencies are present within the existing binding artifacts without obsolete server libraries

#### Scenario: Verify lifecycle and contention

- **WHEN** clients reconnect, consume slowly, cancel reads, or race listener stop with host completion during recording
- **THEN** the suite verifies bounded work, monotonic state, exactly-once terminal results where connected, and uninterrupted permitted persistence

### Requirement: Explicit shared server physical checkpoint

The change SHALL include a separate Android, iOS, and macOS physical checkpoint with the flashed ESP32 fixture. It SHALL exercise the selected production path for connection, telemetry, saved recording, reconnect, and frontend/lifecycle recovery. Physical availability MUST NOT weaken the software gate. Missing hardware or deferred coverage SHALL remain explicitly incomplete unless the user authorizes a narrower checkpoint.

#### Scenario: Run the physical checkpoint

- **WHEN** each physical host connects through the fixture and its frontend is recreated during recording
- **THEN** the canonical runner observes continued durable progress and current authenticated state through the shared server

#### Scenario: Hardware is unavailable

- **WHEN** a required device or fixture is unavailable
- **THEN** software verification completes independently and the missing physical result is recorded without claiming completion
