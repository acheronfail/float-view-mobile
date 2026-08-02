# Platform Hosts Specification

## Purpose

Define repository ownership and the minimal Android, iOS, and macOS host responsibilities around the shared frontend and native backends.

## Requirements

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

### Requirement: Stable native loopback port registry

Supported native backends SHALL bind only to loopback and SHALL use stable default ports: macOS `11010`, iOS `11011`, and Android `11012`. A newly supported backend MUST receive the next unassigned port beginning at `11013`.

#### Scenario: Supported backend starts

- **WHEN** a macOS, iOS, or Android backend starts
- **THEN** its loopback server listens on its assigned platform port

#### Scenario: Assigned port unavailable

- **WHEN** the assigned loopback address and port cannot be bound
- **THEN** the native host fails startup with an actionable diagnostic and does not silently select another port

#### Scenario: New backend allocation

- **WHEN** another native backend becomes supported
- **THEN** it receives one documented unused port from the consecutive registry beginning at `11013`

### Requirement: Browser-development native host seams

Each native platform SHALL provide only the launch, readiness, forwarding, process lifetime, and native-presentation seams required for host-browser development. Those seams MUST use the production Bluetooth, VESC protocol, application-service, storage, permission, lifecycle, and export implementations.

#### Scenario: macOS browser host

- **WHEN** macOS browser development starts
- **THEN** a native process listens on `127.0.0.1:11010` without creating a WKWebView and retains CoreBluetooth, SQLite, permission, and save-panel ownership

#### Scenario: iOS browser host

- **WHEN** iOS browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11011` and a monitored `iproxy` exposes host loopback port `11011`

#### Scenario: Android browser host

- **WHEN** Android browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11012` and ADB exposes host loopback port `11012`

#### Scenario: Production host behavior

- **WHEN** a native application runs outside browser-development orchestration
- **THEN** it continues serving the embedded frontend and owning native state with the same backend contract

### Requirement: Mirrored native connection ownership

The Apple and Android backends SHALL use matching ownership boundaries for application coordination, controller discovery, telemetry polling, Bluetooth stream setup, VESC framing and commands, firmware parsing, exact compatibility mapping, and Refloat protocol families. Apple SHALL place application owners under `backends/apple/Sources/FloatViewBackend/application`, VESC protocol owners under `protocol/vesc`, and Refloat owners under `protocol/refloat`. Android SHALL retain its established application owners under `backends/android/app/src/main/java/fail/acheron/floatview/backend` and place corresponding VESC and Refloat owners under `protocol/vesc` and `protocol/refloat`. Swift and Kotlin implementations MUST remain backend-owned, dependencies MUST point downward from application coordination to Bluetooth and protocol components, and superseded monolithic paths MUST be deleted after extraction.

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend has one clearly named controller-discovery owner rather than route decisions spread through transport, polling, and view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the matching Swift and Kotlin file is found under the corresponding `protocol/vesc` or `protocol/refloat` subtree rather than a version-named application-service branch

#### Scenario: Add a compatible firmware version

- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** each backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** each backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the mirrored Swift/Kotlin locations, dependency direction, privacy boundary, and version-to-family extension rule

### Requirement: Mirrored detailed telemetry ownership

Apple and Android SHALL retain mirrored owners for Refloat family decoding, telemetry scheduling, semantic normalization, application state, durable persistence, session retrieval, and serialization. VESC command routing SHALL remain below Refloat polling, and neither protocol owner SHALL depend on Bluetooth, storage, loopback transport, or frontend code.

#### Scenario: Add a known telemetry field

- **WHEN** support for an existing negotiated Refloat field is added
- **THEN** the matching Swift and Kotlin decoder, normalized model, persistence mapping, serialization, fixtures, and tests are updated without adding platform-specific frontend logic

#### Scenario: Route ownership

- **WHEN** a detailed Refloat request targets a CAN controller
- **THEN** the Refloat owner supplies the command and the VESC routing owner applies CAN forwarding exactly once

### Requirement: Equivalent native detailed persistence

The Apple and Android current schemas SHALL durably store the same detailed typed values in dedicated nullable stable columns, unknown negotiated values only in bounded `extension_json`, write-time field presence in `session_numeric_field`, and discrete transitions in `session_event` with an independent monotonic event sequence, source-sample sequence, observation time, elapsed time, and single-session ownership. Multiple transitions from one sample SHALL be inserted in stable catalog order. An authoritative commit marker SHALL publish sample/event boundaries, recorded/stored counts, elapsed coverage, history revision, and summary source evidence before catalog metadata is exposed. Incompatible test-stage databases SHALL be deleted and recreated from the current schema rather than migrated.

#### Scenario: Equivalent stored sample

- **WHEN** both native backends accept equivalent normalized telemetry
- **THEN** session retrieval and export produce semantically equivalent numeric records and transition events despite native storage implementation differences

#### Scenario: Old disposable schema

- **WHEN** a development database has the previous incompatible telemetry schema
- **THEN** normal disposable-schema handling recreates it without a migration framework or partial column fallback

### Requirement: Native lifecycle owns detailed polling

Detailed realtime polling, the one-second compact due time, normalization, and durable commits SHALL remain native when the WebView is absent or recreated. Each host SHALL retain one in-flight request and no separate telemetry work queue. Background and reconnect behavior SHALL follow the existing platform lifecycle rules and prevent stale responses or a replacement controller from contributing detailed samples or events to an active session.

#### Scenario: WebView replacement

- **WHEN** the frontend disappears while native detailed telemetry continues
- **THEN** accepted samples remain durably ordered and a replacement frontend recovers the latest committed sample

#### Scenario: Controller changes

- **WHEN** reconnect validation reports a replacement controller
- **THEN** detailed polling stops, the old session ends, route-scoped compact state is cleared, and no replacement-controller fields or events are appended to the old session
