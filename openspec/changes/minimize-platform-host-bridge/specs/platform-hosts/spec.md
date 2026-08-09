## ADDED Requirements

### Requirement: Auditable minimal host surface

Production Swift and Kotlin SHALL be limited to seven native capability families: Bluetooth transport, background execution, location, native documents, system appearance, lifecycle/WebView ownership, and the minimal runtime bridge, together with required application entry, permissions, bounded packaged-resource access, and privacy-safe diagnostics. Intermediate application coordination, randomized device-token maps, portable bootstrap services, software peripherals, mirrored bridge-domain layers, and unreachable compatibility surface MUST be absent.

#### Scenario: Host ownership audit

- **WHEN** implementation is complete
- **THEN** a repository audit counts production source separately from tests, generated artifacts, and build checkouts and maps every remaining Swift and Kotlin production type to an allowed host responsibility

#### Scenario: Shared behavior change

- **WHEN** connection policy, software VESC behavior, bootstrap policy, or bridge validation changes
- **THEN** one Rust owner changes without matching behavioral translations in Swift and Kotlin

### Requirement: Uniform native capability inventory

iOS, macOS, and Android SHALL each record every native capability family as `implemented`, `unsupported`, or `incomplete`. Every family SHALL have one predictable platform entry file and one row in both the cross-host capability matrix and the platform README. `Implemented` SHALL identify focused verification, `unsupported` SHALL compile as a small documented stable-unavailable stub, and `incomplete` SHALL fail the final ownership/conformance gate.

#### Scenario: Reader locates a host capability

- **WHEN** a maintainer looks for Bluetooth, background execution, location, documents, appearance, lifecycle/WebView, or runtime-bridge integration on a supported host
- **THEN** the host README and predictably named entry file identify its support status, concrete implementation owner, and focused verification without requiring a repository-wide search

#### Scenario: Platform does not support a capability

- **WHEN** an OS cannot provide a declared capability
- **THEN** that host retains the matching entry file as a compiling stub that explains the limitation, returns the stable unavailable result, and prevents the UI from advertising successful support

#### Scenario: Capability remains incomplete

- **WHEN** a capability matrix row or entry file is marked incomplete at the final gate
- **THEN** verification fails with the host, capability, entry file, and missing acceptance evidence

### Requirement: Predictable native host layout

Capability entry files SHALL use the names `FloatViewBluetooth`, `FloatViewBackgroundExecution`, `FloatViewLocation`, `FloatViewDocuments`, `FloatViewAppearance`, `FloatViewLifecycle`, `FloatViewWebView`, and `FloatViewRuntimeBridge` with the host language extension. iOS and macOS SHALL be sibling directories beneath `backends/apple/`; genuinely shared Apple implementation SHALL live in an explicit common directory. Android SHALL expose the matching entry files from its host package. Supporting private files MAY use focused subdirectories, but generic miscellaneous ownership buckets MUST NOT obscure the capability owner.

#### Scenario: Apple capability uses shared implementation

- **WHEN** iOS and macOS share an implementation
- **THEN** each sibling host retains its predictable entry file and that file points directly to the concrete owner beneath the Apple common directory without duplicating behavior

#### Scenario: macOS host layout is inspected

- **WHEN** a maintainer opens the Apple backend tree
- **THEN** macOS appears as a sibling of iOS with the same capability entry shape rather than as an unrelated package source path

### Requirement: Native Bluetooth adapters remain OS-only

Apple and Android Bluetooth adapters SHALL own OS discovery callbacks, permission and radio mapping, GATT service and characteristic setup, negotiated write sizing, OS write backpressure, callback ordering before bridge entry, private platform transport association resolution, and raw notification forwarding. They MUST NOT own saved-versus-guest policy, public discovered-device collections, VESC firmware UUID parsing, board identity derivation, pending application connection policy, protocol framing, telemetry interpretation, or durable state.

#### Scenario: GATT connection becomes ready

- **WHEN** an OS adapter completes the required VESC service, write-characteristic, and notification setup
- **THEN** it submits the private transport association and bounded transport state while Rust completes firmware identification, saved-board identity resolution, application connection coordination, and authoritative state publication

#### Scenario: Stale native callback

- **WHEN** a callback belongs to a replaced native connection or runtime generation
- **THEN** the adapter or Rust generation boundary rejects it before it can complete a current operation or contribute bytes

### Requirement: Native background execution remains mechanism-only

Each host SHALL expose one background-execution capability entry that owns only its OS mechanism and effective eligibility. iOS SHALL own declared background modes, CoreBluetooth restoration, and background-location configuration; Android SHALL own foreground-service lifecycle, service types, and notification requirements; macOS SHALL document its process behavior and return stable unavailable results for mobile-only requests. Rust SHALL own durable logging intent and policy.

#### Scenario: Android logging continues in background

- **WHEN** Rust requests durable background logging and required Android permissions are available
- **THEN** Android starts the correctly typed foreground service, retains the process-owned runtime, reports effective lifecycle eligibility, and leaves session persistence and logging policy in Rust

#### Scenario: Apple receives background BLE or location work

- **WHEN** iOS is backgrounded after Rust begins an eligible logging session
- **THEN** the Apple host uses declared background modes, CoreBluetooth restoration, and configured location delivery to forward bounded observations to the same process-owned runtime without requiring a WebView

### Requirement: Native location reports authorization changes

Each location capability SHALL submit authorization and effective background-availability changes when its OS delegate or permission-result callback changes them. It MUST NOT rely solely on a later coordinate observation or an external poll. Hosts SHALL report only OS authorization, availability, coordinates, accuracy, and required ordering; Rust SHALL own recording policy, persistence, export inclusion, messages, and public state normalization.

#### Scenario: Apple permission changes without a location sample

- **WHEN** the Apple location delegate reports a new authorization state before any coordinate arrives
- **THEN** the host submits the authorization change immediately and Rust publishes the resulting monotonic location state

#### Scenario: Location is unsupported

- **WHEN** a host cannot provide location
- **THEN** its `FloatViewLocation` stub reports stable unavailable authorization and emits no fabricated coordinates

### Requirement: Native documents remain presentation-only

Native document capability entries SHALL own only picker, save, share, security-scoped URL, content-URI, and bounded-copy mechanics. They SHALL accept only Rust-allocated staging paths within approved roots and MUST NOT parse, validate, name semantically, persist, or merge session archives.

#### Scenario: Platform presents an export

- **WHEN** Rust supplies a valid staged export, display name, and content type
- **THEN** macOS presents its save surface, iOS presents its share or document-export surface, and Android presents its content-URI share/save surface through their matching capability entry points

### Requirement: Host appearance has one owner

Every host SHALL document its single effective-appearance source in `FloatViewAppearance` and its capability inventory. Verified WebView media-query observation SHALL be preferred; a focused native fallback SHALL exist only for a host that fails that verification. Native-only chrome MAY apply platform appearance locally without turning effective appearance into persisted application preference.

#### Scenario: System appearance changes

- **WHEN** the platform switches between light and dark while the application is active, backgrounded and resumed, or recreating its WebView
- **THEN** exactly one source updates the frontend effective theme and native-only chrome remains visually consistent without duplicate bridge events

### Requirement: Superseded host paths are deleted

After Apple and Android pass their respective bridge, lifecycle, software-host, canonical, and physical BLE gates, the repository SHALL delete the old software peripheral/adapters, random-token maps, unused saved-device configuration APIs, reverse codec paths, unreachable commands, obsolete layout paths, obsolete tests, and dependencies used only by those paths. Reachable native document, background, location, and presentation support MUST NOT be deleted as unreachable merely because its contract or bridge wiring is incomplete.

#### Scenario: Cutover completion

- **WHEN** all software gates and the explicit physical BLE checkpoint pass for a host family
- **THEN** only the minimized host path remains selectable in its production and development builds

## MODIFIED Requirements

### Requirement: Native lifecycle and privacy boundary

Native services SHALL continue auxiliary discovery and BMS polling while the frontend is disposable and while background execution is permitted, cancel every BMS generation and queued request on connection replacement, and exclude full BMS telemetry from default logs and durable files. VESC firmware/controller/BMS UUIDs MAY be used in state, persistence, diagnostics, logs, archives, and exports. Apple peripheral UUIDs and Android BLE addresses SHALL remain private transport associations and MUST NOT enter those surfaces.

#### Scenario: WebView disposal

- **WHEN** the frontend is destroyed while a selected BMS remains attached
- **THEN** native polling continues where the OS permits and a replacement frontend recovers the latest authoritative BMS state

#### Scenario: Connection generation changes

- **WHEN** disconnect, reconnect, preference change, or device replacement invalidates auxiliary work
- **THEN** late callbacks and queued work from the older generation cannot select a route or publish a snapshot

#### Scenario: Default diagnostics

- **WHEN** BMS discovery, polling, timeout, or malformed input is diagnosed normally
- **THEN** logs may contain canonical VESC BMS UUIDs but contain only bounded telemetry context and no full cell arrays, complete raw telemetry frames, Apple peripheral UUIDs, or Android BLE addresses
