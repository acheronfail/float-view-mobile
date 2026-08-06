## ADDED Requirements

### Requirement: Composable interactive VBMS topology

The fixture development coordinator SHALL extend its existing controller and transport composition with zero, one, or multiple independently routed source-faithful VBMS32 6.06 nodes, including a local direct-BMS topology where the motor controller is behind CAN. Canonical fixtures SHALL retain stock VESC BMS 6.00 separately. It MUST compose node behavior from typed identities and values rather than introduce a registry of complete board combinations.

#### Scenario: One CAN VBMS

- **WHEN** a developer configures one VBMS at CAN ID `42` with a direct controller or Express topology
- **THEN** the next powered-on connection advertises the configured controller topology and answers BMS identity and values only on route `42`

#### Scenario: Direct VBMS transport

- **WHEN** a developer selects the direct-VBMS topology and configures a motor-controller CAN ID
- **THEN** local firmware reports the VBMS identity, CAN discovery exposes the controller, direct command 96 returns BMS values, and controller requests require their CAN route

#### Scenario: Multiple VBMS nodes

- **WHEN** a developer configures multiple distinct valid VBMS CAN IDs
- **THEN** the coordinator exposes every identity deterministically and lets the native backend demonstrate ambiguity or configured selection

#### Scenario: Invalid or duplicate BMS IDs

- **WHEN** configuration supplies an out-of-range, duplicate, fractional, padded, or nondecimal BMS CAN ID or conflicts with another configured node
- **THEN** the complete save is rejected without changing retained simulator state

### Requirement: Plausible deterministic BMS values

The interactive simulator SHALL generate bounded seeded BMS values with coherent total voltage, per-cell voltages, balancing flags, input current, state of charge and health, temperatures, environmental readings, and monotonic counters. Values SHALL advance only when a matching request is received and SHALL support selectable legacy and extended response tails.

#### Scenario: Idle BMS request sequence

- **WHEN** the same saved configuration and seed receive the same sequence of BMS values requests
- **THEN** they return the same smoothly varying values and counters within source-derived bounds

#### Scenario: Coherent pack values

- **WHEN** the simulator returns a BMS snapshot
- **THEN** total voltage is coherent with bounded per-cell voltages, balancing flags match configured cells, and charge, health, temperature, humidity, and pressure remain plausible

#### Scenario: Legacy and extended variants

- **WHEN** a developer selects either supported response variant
- **THEN** the coordinator emits the exact fixed response or exact data-version-plus-status extension without changing shared normalized values

#### Scenario: Timeout and malformed modes

- **WHEN** the developer selects a bounded BMS timeout or malformed-response mode
- **THEN** only the matching configured BMS request is affected and controller identity and Refloat telemetry remain available

### Requirement: Observable BMS fixture controls

The fixture control surface SHALL expose bounded atomic BMS topology, CAN-ID, response-variant, values, and failure controls alongside existing controller configuration. It SHALL show BMS request counters and high-level route diagnostics without retaining full telemetry, UUIDs, or unbounded packet data.

#### Scenario: Active configuration lock

- **WHEN** the fixture is advertising or connected
- **THEN** BMS topology and value configuration controls are visibly disabled and a save cannot change active wire behavior

#### Scenario: Atomic powered-off save

- **WHEN** a developer saves valid controller and BMS configuration while powered off
- **THEN** the coordinator validates and applies the entire composition once, resets topology-scoped counters and framing, and publishes one newer complete fixture state

#### Scenario: Accessible BMS controls

- **WHEN** BMS controls are operated by primary pointer or keyboard
- **THEN** native control semantics are retained and each enabled action occurs exactly once
