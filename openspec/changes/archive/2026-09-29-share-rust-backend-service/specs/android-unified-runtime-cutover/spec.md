## MODIFIED Requirements

### Requirement: Android selects one unified Rust runtime

The Android backend SHALL select one process-owned unified Rust application runtime through one bounded callback-free JNI boundary and SHALL retain Bluetooth, execution, lifecycle/background service, permissions, location acquisition, native file presentation, theme/UI, protected storage-root setup, and import/export presentation as Kotlin-owned OS responsibilities. Shared Rust SHALL own loopback transport, authentication, frontend dispatch, and serialization. The shared backend service SHALL own product/catalog persistence, archive formats and portable application policy. Kotlin SHALL supply typed OS facts and execute bounded OS effects without duplicating domain records, frontend dispatch or portable decisions.

#### Scenario: Execute an ordered runtime result

- **WHEN** Android supplies a valid platform fact or frontend operation to the runtime
- **THEN** Kotlin validates the complete result and executes its bounded effects in order without a native protocol/application/session-decision fallback
