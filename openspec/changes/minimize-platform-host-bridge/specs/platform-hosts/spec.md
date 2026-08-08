## ADDED Requirements

### Requirement: Auditable minimal host surface
Production Swift and Kotlin SHALL be limited to application entry, WebView and lifecycle ownership, OS Bluetooth and location adapters, background execution, permissions, bounded packaged-resource and document access, native settings/theme/export presentation, privacy-safe diagnostics, and the minimal versioned bridge worker. Intermediate application coordination, randomized device-token maps, portable bootstrap services, software peripherals, mirrored bridge-domain layers, and unreachable compatibility surface MUST be absent.

#### Scenario: Host ownership audit
- **WHEN** implementation is complete
- **THEN** a repository audit counts production source separately from tests, generated artifacts, and build checkouts and maps every remaining Swift and Kotlin production type to an allowed host responsibility

#### Scenario: Shared behavior change
- **WHEN** connection policy, software VESC behavior, bootstrap policy, or bridge validation changes
- **THEN** one Rust owner changes without matching behavioral translations in Swift and Kotlin

### Requirement: Native Bluetooth adapters remain OS-only
Apple and Android Bluetooth adapters SHALL own OS discovery callbacks, permission and radio mapping, GATT service and characteristic setup, negotiated write sizing, OS write backpressure, callback ordering before bridge entry, private platform transport association resolution, and raw notification forwarding. They MUST NOT own saved-versus-guest policy, public discovered-device collections, VESC firmware UUID parsing, board identity derivation, pending application connection policy, protocol framing, telemetry interpretation, or durable state.

#### Scenario: GATT connection becomes ready
- **WHEN** an OS adapter completes the required VESC service, write-characteristic, and notification setup
- **THEN** it submits the private transport association and bounded transport state while Rust completes firmware identification, saved-board identity resolution, application connection coordination, and authoritative state publication

#### Scenario: Stale native callback
- **WHEN** a callback belongs to a replaced native connection or runtime generation
- **THEN** the adapter or Rust generation boundary rejects it before it can complete a current operation or contribute bytes

### Requirement: Superseded host paths are deleted
After Apple and Android pass their respective bridge, lifecycle, software-host, canonical, and physical BLE gates, the repository SHALL delete the old software peripheral/adapters, random-token maps, unused saved-device configuration APIs, reverse codec paths, unreachable commands, obsolete tests, and dependencies used only by those paths.

#### Scenario: Cutover completion
- **WHEN** all software gates and the explicit physical BLE checkpoint pass for a host family
- **THEN** only the minimized host path remains selectable in its production and development builds
