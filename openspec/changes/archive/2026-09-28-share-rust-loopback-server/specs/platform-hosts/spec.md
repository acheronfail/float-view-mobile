## MODIFIED Requirements

### Requirement: One shared Apple backend

iOS and macOS under `/backends/apple` SHALL use the same Apple adapters for Bluetooth, storage, lifecycle, and OS integration. Apple and Android SHALL select the same Rust application runtime and shared Rust loopback server. The shared server SHALL own transport, authentication, contract dispatch, and frontend serialization while platform adapters retain OS execution and catalog storage.

#### Scenario: Apple behavior change

- **WHEN** shared backend behavior changes
- **THEN** both iOS and macOS receive the same implementation and contract semantics
