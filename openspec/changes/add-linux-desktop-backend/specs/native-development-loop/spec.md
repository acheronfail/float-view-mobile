# Spec Delta

## MODIFIED Requirements

### Requirement: Platform-selected native development

The repository SHALL provide `just dev mock|macos|ios|android|linux [target]` as the public hot-reload entry point. A native selection MUST run the shared frontend in a development-host browser against exactly one selected real native backend, while `mock` MUST run the deterministic mock backend.

#### Scenario: macOS frontend iteration

- **WHEN** a developer runs `just dev macos`
- **THEN** Vite hot module replacement serves the shared frontend in the host browser while the macOS process owns the production CoreBluetooth, application-service, storage, permission, and export paths

#### Scenario: Selected mobile target

- **WHEN** a developer runs `just dev ios <target>` or `just dev android <target>`
- **THEN** the selected installed application owns its native backend while the host browser uses that backend through the platform forwarding transport

#### Scenario: Deterministic mock

- **WHEN** a developer runs `just dev mock`
- **THEN** Vite starts with the deterministic mock backend and no native application is built or launched

#### Scenario: Linux frontend iteration

- **WHEN** a developer runs `just dev linux` in a supported Linux desktop session
- **THEN** Vite serves the shared frontend through the existing authenticated loopback proxy while one Linux process on port `11013` owns production Bluetooth, persistence and native presentation without an embedded WebView
- **AND** readiness, interruption and child cleanup use the same bounded orchestration policy as other native targets
