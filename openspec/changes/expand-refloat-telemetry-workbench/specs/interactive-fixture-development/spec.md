## ADDED Requirements

### Requirement: Editable scenario workbench
`just fixture dev` SHALL provide a manual scenario workbench that composes an editable BLE endpoint, at most 16 controller candidates, at most 64 discovered CAN IDs, source-package identities, discovery behavior, notification fragmentation, and semantic telemetry state while a separately launched `just dev <platform>` application is operated by the developer. The workbench MUST NOT automatically drive the product, choose the product's active controller, assert product outcomes, or report pass/fail. Float View SHALL request telemetry from only its one selected route even when the scenario advertises multiple candidates.

#### Scenario: Reproduce a reported setup
- **WHEN** a developer configures an Express endpoint, a non-motor CAN node, two motor controllers, and a controller omitted from ping
- **THEN** the simulator answers the developer-operated application's requests according to that topology until the developer changes or powers off the scenario

#### Scenario: Manual investigation
- **WHEN** the connected product reaches a state the developer wants to inspect
- **THEN** the workbench waits for manual actions and does not advance through a scripted test or select a controller on the product's behalf

### Requirement: Bounded shareable scenario documents
The workbench SHALL import and export a versioned human-readable scenario document of at most 65,536 UTF-8 bytes containing topology, public simulated identities, discovery behavior, fragmentation, semantic telemetry baseline, coherent scene selection, and explicit overrides. Documents SHALL be strictly validated. Raw controller UUID bytes, serial paths, launch tokens, power state, and real Bluetooth identifiers MUST NOT be imported, exported, or displayed. A scenario-local controller alias of at most 64 UTF-8 bytes SHALL deterministically resolve to a private simulated UUID on the server.

#### Scenario: Share a bug reproduction
- **WHEN** a developer exports a valid configured scenario and another developer imports it
- **THEN** both workbenches expose equivalent wire identity, topology, discovery, and telemetry behavior without sharing a real controller identifier

#### Scenario: Invalid imported document
- **WHEN** a document is oversized, has an unsupported schema version, duplicate or invalid CAN IDs, excessive nodes, non-finite telemetry, or unknown privileged fields
- **THEN** the server rejects it atomically without changing the retained scenario or active simulator

### Requirement: Semantic telemetry scenes and overrides
The workbench SHALL own one version-independent semantic board-state model and project it into the exact selected Refloat wire family. It SHALL provide coherent manual scenes for idle, engaged, cruise, acceleration, braking, ATR, high current, high duty, field weakening, supported pushback states, temperatures, voltage states, wheelslip, footpad combinations, stop conditions, darkride, hand-test, flywheel, charging, and firmware fault. Developers SHALL be able to override bounded individual values, freeze or resume generated motion, and restore scene defaults.

#### Scenario: Coherent high-load scene
- **WHEN** a developer activates the hard-acceleration scene
- **THEN** requested and actual current, duty, voltage sag, pitch, ATR or torque contribution, and running state change together to a deterministic plausible baseline

#### Scenario: Expert override
- **WHEN** a developer pins controller temperature after choosing a coherent riding scene
- **THEN** subsequent generated samples retain that temperature while other unpinned values continue according to the scene

#### Scenario: Version projection
- **WHEN** the same semantic scene is selected for representative audited Refloat 1.0.x, 1.1.x, and 1.2.x releases
- **THEN** each response uses its source-faithful layout and reports unavailable values as absent rather than inventing equivalent bytes

#### Scenario: Legacy realtime compatibility
- **WHEN** a client sends Refloat 1.0.x command 1
- **THEN** the fixture returns the fixed-layout high-precision realtime response rather than encoding the compact command 10 layout under command 1

### Requirement: Manual connection and identity events
The workbench SHALL provide explicit bounded controls to disconnect and re-advertise, reconnect unchanged, move the same private controller identity to another valid CAN ID, replace a controller identity, change ping visibility, and select a supported notification-fragment bound. Identity or topology mutations that would make an in-flight response ambiguous SHALL occur only through a serialized disconnect-transition-re-advertise action or while powered off.

#### Scenario: Same controller moves
- **WHEN** a developer invokes the move-controller event during an active connection
- **THEN** the coordinator disconnects, applies the CAN-ID change while disconnected, and re-advertises with the same private UUID

#### Scenario: Replacement controller
- **WHEN** a developer invokes the replace-controller event
- **THEN** the next connection exposes a different private UUID at the configured route without exporting or logging either UUID

#### Scenario: Fragmented detailed response
- **WHEN** the fragment bound is set to 20 and a negotiated field-list response is larger than 20 bytes
- **THEN** the exact encoded response is delivered in ordered fragments of at most 20 bytes

### Requirement: Observable workbench state
The control surface SHALL show the resolved topology, semantic state, values available in the selected Refloat family, request classification and route, response kind and fragmentation, counters, and bounded diagnostics. Controls SHALL retain native button semantics and consistent primary-pointer, cancellation, keyboard, and assistive-technology behavior.

#### Scenario: Unsupported source field
- **WHEN** a semantic value is not available in the selected package family
- **THEN** the control surface marks it unavailable instead of showing a transmitted value

#### Scenario: Touch and keyboard actions
- **WHEN** a primary touch pointer or keyboard user activates a state or connection control
- **THEN** the action runs exactly once, while non-primary and cancelled pointer activation do not mutate the workbench
