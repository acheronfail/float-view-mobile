## MODIFIED Requirements

### Requirement: Android selects one unified Rust runtime

The Android backend SHALL select one process-owned unified Rust application runtime through one bounded callback-free JNI boundary and SHALL retain Bluetooth, execution, lifecycle/background service, permissions, location acquisition, native file presentation, theme/UI, product catalogue, and import/export presentation as Kotlin-owned OS responsibilities. Shared Rust SHALL own loopback transport, authentication, frontend dispatch, and serialization. Kotlin SHALL supply typed facts and execute bounded host operations without duplicating the frontend API.

#### Scenario: Execute an ordered runtime result

- **WHEN** Android supplies a valid platform fact or frontend operation to the runtime
- **THEN** Kotlin validates the complete result and executes its bounded effects in order without a native protocol/application/session-decision fallback
