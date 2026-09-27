## MODIFIED Requirements

### Requirement: Apple retains OS and contract ownership

Swift SHALL retain Bluetooth execution, clocks/timers, lifecycle/background work, permissions, location acquisition, native file presentation, theme/UI integration, WebView ownership, product catalogue metadata, and export/import presentation. Shared Rust SHALL own loopback transport, authentication, frontend dispatch, and serialization. Swift SHALL supply typed facts and execute bounded host operations without duplicating the frontend API. Runtime state and durable session data SHALL survive disposable WebView replacement and remain exposed only through the existing authenticated frontend contract.

#### Scenario: Execute an effect

- **WHEN** Rust emits a bounded host effect
- **THEN** Swift executes only the corresponding OS/cache/catalogue/publication work in order and reports later facts without adding application policy

#### Scenario: Replace the WebView

- **WHEN** iOS or macOS destroys and recreates its WebView
- **THEN** the new frontend receives the same native-process runtime snapshot and authenticated monotonic state stream

#### Scenario: Acquire a location

- **WHEN** native location policy provides an authorized finite observation for an attached session
- **THEN** Swift supplies it to Rust and publishes location state only after the Rust session marker advances
