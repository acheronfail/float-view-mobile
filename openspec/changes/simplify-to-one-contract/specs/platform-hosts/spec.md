## ADDED Requirements

### Requirement: Lowercase ownership layout
The repository SHALL place the frontend under `/frontend`, the shared Apple implementation and its iOS/macOS hosts under `/backends/apple`, the Android implementation and host under `/backends/android`, shared backend SQL and immutable protocol fixtures under `/backends`, canonical acceptance under top-level `/conformance`, and supporting firmware under `/tools/vesc-test-device`.

#### Scenario: Repository navigation
- **WHEN** a developer inspects the repository root
- **THEN** lowercase top-level directories identify frontend, backend, conformance, tool, and planning ownership without generic package or shared-code layers

### Requirement: One shared Apple backend
iOS and macOS under `/backends/apple` SHALL use the same Apple backend implementation for application state, Bluetooth, VESC protocol, storage, loopback transport, and contract dispatch.

#### Scenario: Apple behavior change
- **WHEN** shared backend behavior changes
- **THEN** both iOS and macOS receive the same implementation and contract semantics

### Requirement: Thin Apple hosts
The iOS and macOS hosts SHALL contain only application entry, lifecycle, WebView, permission presentation, export presentation, and platform resource wiring not owned by the shared backend.

#### Scenario: WebView replacement
- **WHEN** an Apple host replaces its WebView
- **THEN** it reconnects to the same native backend state without reimplementing backend behavior

### Requirement: One Android application module
Android under `/backends/android` SHALL use one Gradle application module. Backend boundaries SHALL be represented by focused Kotlin packages and source files rather than separate `api-models`, `core`, `protocol`, `logging`, and `bluetooth` Gradle modules.

#### Scenario: Android build
- **WHEN** the Android application is assembled
- **THEN** all required backend and host code is built from the one application module

### Requirement: Native backend ownership
Each native process SHALL own Bluetooth state, application state, background work, durable logging, and the loopback server independently of the WebView. The WebView SHALL own no durable or Bluetooth state.

#### Scenario: Frontend termination
- **WHEN** the WebView process or view is terminated
- **THEN** native backend state remains authoritative and recoverable by a replacement frontend

### Requirement: Minimal dependencies
Platform builds SHALL remove dependencies used only by deleted generation, module boundaries, native UI scaffolding, or duplicate tests, while retaining focused maintained dependencies that replace substantial protocol or server code.

#### Scenario: Dependency audit
- **WHEN** implementation is complete
- **THEN** every remaining direct dependency is used by current build, runtime, or verification code

### Requirement: No generic shared-code directory
Cross-language shared content under `/backends` SHALL be limited to immutable SQL and protocol fixtures. Swift and Kotlin implementation code MUST remain owned by their backend.

#### Scenario: Shared backend resource
- **WHEN** both backends consume the same current SQL schema or protocol fixture
- **THEN** the immutable resource may reside under `/backends` without introducing a cross-language shared-code layer
