## MODIFIED Requirements

### Requirement: Platform responsibilities remain outside the runtime

The runtime MUST express Bluetooth I/O, clocks, timers, persistence paths, cache access, routing-hint storage, and activation as caller facts or returned effects. Bluetooth adapters, location acquisition, lifecycle/background execution, permissions, native file presentation, theme, and WebViews SHALL remain platform-owned. Shared Rust transport SHALL own loopback authentication and the frontend API above the application runtime. The shared backend service SHALL satisfy portable cache, routing-hint, storage and activation effects directly, forwarding only OS effects to native adapters. The application runtime MUST remain independent of HTTP, sockets, and server executors.

#### Scenario: Execute runtime effects

- **WHEN** the runtime returns writes, timers, cache work, activation, or publication effects
- **THEN** the shared backend service executes portable work, dispatches OS work to native adapters and reports resulting facts through later runtime commands

#### Scenario: Replace a WebView

- **WHEN** a platform destroys and recreates its WebView while the native process and runtime remain alive
- **THEN** the replacement frontend observes the same process-owned authoritative state without transferring runtime ownership to the view
