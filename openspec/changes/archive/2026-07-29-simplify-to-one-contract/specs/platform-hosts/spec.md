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

### Requirement: Physical conformance host seams
Debug physical-conformance builds MAY provide only the endpoint and lifecycle control needed by the host TypeScript runner. Those seams MUST preserve loopback-only transport and MUST use the same production Bluetooth, VESC protocol, application-service, and storage implementations as the application.

#### Scenario: Android physical launch
- **WHEN** conformance installs a Debug Android build on a selected ADB device
- **THEN** a fixed Debug loopback port can be forwarded while the application uses `AndroidBluetoothAdapter` and normal process-owned backend state

#### Scenario: Android physical scenarios reuse one authorized application
- **WHEN** multiple physical conformance scenarios run in one Android suite
- **THEN** the Debug application is built and installed once, permissions and loopback forwarding are prepared once, and each launch resets only app-owned Float View storage before normal backend initialization

#### Scenario: iOS physical launch
- **WHEN** conformance installs and launches a signed Debug iOS build on a selected device
- **THEN** a launch-configured loopback port can be tunneled while the application uses `CoreBluetoothAdapter` and normal native backend state

#### Scenario: iOS physical scenarios reuse one authorized application
- **WHEN** multiple physical conformance scenarios run in one iOS suite
- **THEN** the signed Debug application is installed once, each launch resets only app-owned test storage before backend initialization, and the existing OS Bluetooth authorization is preserved

#### Scenario: iOS physical launch does not own the developer terminal
- **WHEN** conformance launches or terminates the signed Debug iOS application
- **THEN** it uses structured process identifiers without attaching the application console to the invoking TTY, and loopback proxy failures are bounded and reported at the point of failure

#### Scenario: iOS physical build output
- **WHEN** the signed Debug application and lifecycle driver build, install, or execute successfully
- **THEN** Xcode plumbing output remains hidden behind concise timed substep progress, while a failure returns bounded captured diagnostics and cleanup remains independently bounded

#### Scenario: iOS device is still preparing
- **WHEN** Xcode reports that the selected physical device is temporarily busy preparing for development
- **THEN** destination discovery and the total retry count remain bounded, retry progress is visible, and unrelated build, signing, or installation failures are not retried

#### Scenario: iOS lifecycle control
- **WHEN** deterministic physical background coverage requires OS Home and foreground actions
- **THEN** a minimal lifecycle-only XCUITest driver may perform those actions serially without containing contract assertions, VESC behavior, or persistence inspection, and each action completes before conformance proceeds

#### Scenario: macOS physical launch
- **WHEN** conformance launches the macOS test host without software-Bluetooth or seeded-state configuration
- **THEN** readiness is exposed while CoreBluetooth and isolated production backend paths are used
