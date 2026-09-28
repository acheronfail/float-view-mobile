## MODIFIED Requirements

### Requirement: Apple retains OS and contract ownership

Swift SHALL retain Bluetooth execution, clocks/timers, lifecycle/background work, permissions, location acquisition, native file presentation, theme/UI integration, WebView ownership, storage-root protection, and export/import presentation. Shared Rust SHALL own loopback transport, authentication, frontend dispatch, and serialization. The shared backend service SHALL own product/catalog persistence, archive formats and portable application policy. Swift SHALL supply typed OS facts and execute bounded OS effects without duplicating domain records, frontend dispatch or portable decisions. Runtime state and durable session data SHALL survive disposable WebView replacement and remain exposed only through the existing authenticated frontend contract.

#### Scenario: Execute an effect

- **WHEN** Rust emits a bounded host effect
- **THEN** Swift executes only the corresponding OS capability work in order and reports later facts without adding application policy

#### Scenario: Replace the WebView

- **WHEN** iOS or macOS destroys and recreates its WebView
- **THEN** the new frontend receives the same native-process runtime snapshot and authenticated monotonic state stream

#### Scenario: Acquire a location

- **WHEN** the native location provider supplies an authorized finite observation for an attached session
- **THEN** Swift supplies it to the shared service, which applies capture policy and publishes location state only after the Rust session marker advances
