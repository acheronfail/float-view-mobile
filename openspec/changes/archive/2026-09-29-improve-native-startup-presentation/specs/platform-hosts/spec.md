## ADDED Requirements

### Requirement: Native startup presentation

Each supported host SHALL use its platform's normal launch and window presentation with an application-themed canvas. It SHALL NOT introduce a second splash screen, minimum splash duration, or startup visibility gate dependent on Bluetooth, backend connection, or telemetry readiness. Native theme changes SHALL be reflected in current and subsequently recreated host surfaces.

#### Scenario: Cold launch without a connected device

- **WHEN** the application launches with Bluetooth unavailable or no device connected
- **THEN** frontend content can become visible without waiting for device discovery or connection
- **AND** the native canvas remains consistent with the selected appearance during document loading

#### Scenario: Desktop window reopen

- **WHEN** the desktop window is closed and reopened while the application remains running
- **THEN** the new window and web canvas use the current application appearance
- **AND** the existing application service remains the sole runtime owner

### Requirement: Startup presentation through frontend lifecycle

Normal frontend absence during backgrounding or reconstruction SHALL preserve an application-themed surface without displaying a startup error. Existing recovery and teardown behavior SHALL retain native service/session ownership. Presentation callbacks and observers SHALL remain bounded by their owner lifetimes and SHALL NOT accumulate across repeated recreation.

#### Scenario: Return after frontend disposal

- **WHEN** the application returns to the foreground after its frontend was disposed
- **THEN** the recreated view presents the current appearance without a transient loading/error screen
- **AND** ongoing native session and connection state is preserved according to existing lifecycle policy

#### Scenario: Retained frontend resumes

- **WHEN** the application resumes with its frontend retained and system appearance changed while backgrounded
- **THEN** system-selected appearance updates while an explicit app preference remains unchanged

#### Scenario: Existing renderer recovery

- **WHEN** an existing host recovery path replaces a terminated web renderer
- **THEN** the replacement uses the current appearance and existing service owner
- **AND** no startup mask hides the recovered content or existing failure presentation

#### Scenario: Repeated frontend recreation

- **WHEN** the frontend is recreated repeatedly
- **THEN** theme observers, presentation callbacks and native view resources do not grow with recreation count

### Requirement: Startup failures remain distinguishable and visible

Startup presentation SHALL distinguish normal frontend absence from an actual startup failure. It SHALL NOT hide existing native or frontend error presentation behind a loading surface or wait indefinitely for a new startup-ready signal.

#### Scenario: Native frontend startup fails

- **WHEN** the iOS host cannot start its loopback frontend while the app is active
- **THEN** it shows a startup error rather than normal loading presentation
- **AND** successful activation through the existing lifecycle path clears the stale error

#### Scenario: Failure while backgrounded

- **WHEN** the frontend is absent during backgrounding or foreground reconstruction without a recorded startup failure
- **THEN** the host does not display a startup error

#### Scenario: Web load or connection fails

- **WHEN** document navigation or the frontend's backend connection fails
- **THEN** startup styling does not obscure the existing platform or frontend error/recovery behavior
- **AND** presentation adds no unbounded retry or wait-for-connectivity loop
