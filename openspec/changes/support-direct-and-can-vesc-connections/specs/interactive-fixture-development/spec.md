## MODIFIED Requirements

### Requirement: Selectable direct and VESC Express topology
The controller SHALL expose each composed controller/package combination either as a directly connected BLE VESC or behind a VESC Express 6.05 BLE transport with a bounded configuration of one or more CAN-connected motor controllers. Controller firmware SHALL be selected from audited stable 6.02, 6.05, 6.06, and 7.00 variants that share one source-faithful `vescCoreReadOnly` identity implementation with version-applicable optional tails. Each Express controller SHALL have one unique canonical decimal CAN ID in the inclusive range `0` through `253`, with the common single-controller configuration defaulting to `1`. Interactive fixture development MAY present the common single-controller editor while conformance composes multiple controllers through the same simulator types.

#### Scenario: Direct controller discovery
- **WHEN** direct topology is selected and the native backend requests firmware
- **THEN** the BLE peer reports the selected controller firmware with VESC hardware type and accepts applicable Refloat requests without CAN forwarding

#### Scenario: Express controller discovery
- **WHEN** Express topology is selected with one controller on CAN ID `42`
- **THEN** the BLE peer first reports VESC Express 6.05 transport identity, CAN discovery returns `42`, and the selected controller identity and applicable Refloat responses are available through CAN-forward command `34` targeting `42`

#### Scenario: Multiple Express controllers
- **WHEN** conformance composes controllers at CAN IDs `17` and `42`
- **THEN** CAN discovery and forwarded firmware probes expose both independent source-faithful identities and route later responses only to the addressed controller

#### Scenario: Forward to another CAN ID
- **WHEN** Express topology is selected and a forwarded request targets an ID other than any configured controller
- **THEN** the simulator returns no controller response and records only a bounded high-level diagnostic

#### Scenario: Invalid CAN ID
- **WHEN** a developer submits `254`, `255`, a negative or fractional number, padded text, a non-decimal representation, nonnumeric input, or a duplicate ID
- **THEN** the server rejects the value without changing retained controller configuration or simulator state

#### Scenario: Direct topology retains CAN input
- **WHEN** direct topology is selected
- **THEN** the CAN-ID input is disabled, no CAN device is advertised, and the last valid Express CAN configuration remains visible and available for a later Express selection

#### Scenario: Topology configuration saved while powered off
- **WHEN** a developer includes changed topology or valid CAN configuration in the complete configuration saved while powered off and disconnected
- **THEN** the coordinator applies it with the firmware and Refloat selections and atomically resets topology-scoped framing, discovery, layout, telemetry, and counters before the next advertisement

#### Scenario: Topology configuration saved while active
- **WHEN** a developer attempts to save changed topology or CAN configuration while advertising or connected
- **THEN** the server rejects the complete save without changing the identity or forwarding path visible to the BLE peer

#### Scenario: Complete topology matrix
- **WHEN** software verification exercises every runtime-composed controller/package pair through direct and single-controller Express topology
- **THEN** table-driven Cartesian tests cover all twenty-four base configurations, with additional multi-controller and invalid/test-identity cases composed from the same controller types rather than separate simulator implementations

#### Scenario: Version-applicable identity suffix
- **WHEN** the fixture encodes audited VESC 6.02, 6.05, 6.06, or 7.00 identity
- **THEN** it emits the complete common identity baseline and the source-applicable bounded hardware-CRC suffix without using the CRC to choose protocol behavior

### Requirement: Version-specific Refloat protocol
Each Refloat codec SHALL implement the source-derived INFO and read-only telemetry protocol of exact package identity 1.0.0, 1.1.0, or 1.2.3. The native compatibility catalog SHALL treat all three exact packages as supported through their matching legacy or dynamic wire family.

#### Scenario: Refloat 1.0.0 INFO
- **WHEN** a client requests Refloat INFO from the 1.0.0 codec
- **THEN** the codec returns the legacy version-1 short INFO response encoding major/minor as `10` for Refloat 1.0.0 even when the request asks for INFO version 2

#### Scenario: Refloat 1.0.0 telemetry
- **WHEN** a client sends an allowlisted legacy realtime or compact all-data request to the 1.0.0 codec
- **THEN** the response uses the corresponding fixed legacy layout and the codec does not advertise dynamic commands 31 or 32

#### Scenario: Refloat 1.1.0 protocol
- **WHEN** a client requests INFO, realtime IDs, or realtime data from the 1.1.0 codec
- **THEN** the codec returns INFO version 2 with package version 1.1.0 and uses the 1.1 dynamic ID and float16 data layouts without the 1.2 alert tail

#### Scenario: Refloat 1.2.3 protocol
- **WHEN** a client requests INFO, realtime IDs, or realtime data from the 1.2.3 codec
- **THEN** the codec returns INFO version 2 with package version 1.2.3 and uses the 1.2 dynamic layout, mask, and bounded alert/fault tail

#### Scenario: Native compatibility matches fixture packages
- **WHEN** either native backend connects to any selected 1.0.0, 1.1.0, or 1.2.3 package
- **THEN** it selects the matching protocol family, reports source package 1.0.0 using its detectable wire version `1.0`, reaches `ready`, and requests only packets supported by that source-faithful codec

#### Scenario: Direct telemetry is supported
- **WHEN** either native backend connects directly to a supported THOR300 and Refloat combination
- **THEN** the fixture answers direct identity, layout when applicable, and telemetry requests and the backend produces frontend riding telemetry without CAN forwarding
