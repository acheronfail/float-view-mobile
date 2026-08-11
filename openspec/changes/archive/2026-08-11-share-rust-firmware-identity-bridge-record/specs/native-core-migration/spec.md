## ADDED Requirements

### Requirement: Shared Rust firmware identity bridge record contract

The safe `vesc-firmware-identity` component SHALL be the single owner of the existing bounded platform-neutral firmware-identity bridge record, including its version, maximum payload and record extents, field/flag layout, replacement-decoded name serialization, optional firmware-name and hardware-configuration-CRC representation, and closed decoder-to-record outcome. The component SHALL remain dependency-free and unsafe-free and MUST expose no C/JNI symbol, pointer, host status, callback, retained reference, platform framework, or generated binding.

The independent Apple FFI and Android JNI bridges SHALL consume that shared result while retaining their existing public symbols, argument shapes, caller validation, fixed status mapping, caller-owned publication, artifacts, and host isolation. Neither bridge SHALL retain a parallel record encoder, record-layout constant set, or domain-error outcome implementation.

#### Scenario: Produce the existing canonical identity record

- **WHEN** the shared record operation receives any audited motor, Express, stock-BMS, or VBMS32 firmware payload
- **THEN** it returns the exact existing bounded record bytes for every version, name, UUID, test marker, hardware type, configuration count, optional firmware name, and optional CRC field

#### Scenario: Preserve portable failure classification

- **WHEN** the operation receives a wrong command, malformed or truncated identity, or an impossible constructed identity representation
- **THEN** it returns the matching closed unsupported-command, structural, or internal outcome without a partial record or private payload diagnostic

#### Scenario: Select the same result through both bridges

- **WHEN** Apple and Android decode the same accepted or rejected identity through their compiled production bridge
- **THEN** each bridge maps the one shared outcome to its unchanged status and caller-owned result without executing bridge-local serialization or structural decoding

#### Scenario: Preserve safe component isolation

- **WHEN** repository verification inspects the safe component and both host bridges
- **THEN** the component remains dependency-free and unsafe-free, the bridges remain independent platform artifacts, and no generated binding, macro-generated export, runtime, callback, or host type crosses the dependency boundary
