## ADDED Requirements

### Requirement: Mirrored native location ownership

Apple and Android SHALL implement equivalent application-service location ownership, permission mapping, session-clock assignment, durable append, bounded reads, export, and lifecycle behavior. Location adapters SHALL not depend on WebSocket, Svelte, or browser geolocation code.

#### Scenario: Native ownership

- **WHEN** the frontend WebView is destroyed or replaced
- **THEN** the native application service remains the sole owner of location capture and the active session's recorder state

#### Scenario: Cross-platform result

- **WHEN** equivalent permission and provider fixtures are supplied to Apple and Android
- **THEN** both backends expose the same contract status, row shape, boundary semantics, and stable errors

### Requirement: Android background location service

Android SHALL extend the existing active-session foreground logging service with the required location foreground-service declaration, notification behavior, runtime permission flow, and settings fallback. Location capture SHALL remain tied to the authoritative active session and SHALL not create an independent logging service.

#### Scenario: Android background ride

- **WHEN** an authorized location-enabled board in `riding` is backgrounded
- **THEN** the existing foreground service remains eligible to receive and persist location updates and the next frontend connection recovers its state

#### Scenario: Android missing background authorization

- **WHEN** foreground location is allowed but background capability is not granted
- **THEN** the backend reports the bounded capability state and offers the Android settings path without stopping board telemetry logging

### Requirement: Apple Core Location lifecycle

The Apple backend SHALL use Core Location through the shared application service, declare the required iOS usage/background configuration, and map iOS/macOS authorization changes into the common contract. The iOS UI-server lifecycle SHALL not stop the native recorder when the app is backgrounded.

#### Scenario: iOS background capture

- **WHEN** an authorized location-enabled iOS ride enters the background
- **THEN** Core Location remains configured for the permitted background mode and native persistence continues independently of the WebView server

#### Scenario: iOS foreground-only grant

- **WHEN** an iOS ride has only foreground location authorization and enters the background
- **THEN** the Apple backend reports background capture unavailable while board logging continues

#### Scenario: macOS authorization

- **WHEN** macOS reports a location authorization change
- **THEN** the shared Apple service publishes the corresponding bounded status and starts/stops capture according to settings and session state

### Requirement: Non-browser mock seam

Mock and browser-development hosts SHALL expose deterministic location permission/status fixtures and session-data rows through the native contract seam. They SHALL not request browser geolocation or create platform-specific frontend permission branches.

#### Scenario: Deterministic mock

- **WHEN** conformance runs against the mock backend with location fixtures enabled
- **THEN** it receives repeatable permission transitions, location rows, gaps, and boundaries through the same contract as native targets
