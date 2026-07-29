# Programmable VESC Fixture Specification

## Purpose

Define the minimal ESP-IDF BLE byte-bridge firmware used by canonical physical conformance.

## Requirements

### Requirement: Pure ESP-IDF firmware project
`/tools/vesc-test-device` SHALL be a conventional PlatformIO-managed ESP-IDF firmware project containing only the files required to build, flash, document, and test the BLE bridge. It MUST NOT contain a TypeScript package, VESC simulator, board definitions, scenarios, platform launchers, evidence system, or conformance assertions.

#### Scenario: Firmware build
- **WHEN** a developer runs `just fixture build`
- **THEN** the fixture builds without installing Node.js dependencies

#### Scenario: Firmware workflow
- **WHEN** a developer uses `just fixture setup`, `ports`, `flash`, or `monitor`
- **THEN** the command delegates directly to PlatformIO without a duplicate fixture wrapper or host package

### Requirement: Real BLE byte bridge
The ESP32 firmware SHALL advertise the canonical VESC UART GATT service and exchange exact bytes through a real BLE stack used by production Android, iOS, and macOS Bluetooth implementations. It MUST NOT interpret VESC or Refloat payloads.

#### Scenario: Backend discovery
- **WHEN** the fixture advertises and a native backend scans
- **THEN** the backend discovers it through the production OS Bluetooth path

#### Scenario: GATT write forwarding
- **WHEN** the connected application writes bytes to the VESC characteristic
- **THEN** the firmware reports the exact bytes as `WRITE <HEX>` without interpreting them

### Requirement: Minimal serial input protocol
The firmware SHALL accept only `RESET`, `ADVERTISE`, `NOTIFY <HEX>`, `DISCONNECT`, and `LED <RRGGBB>`.

#### Scenario: Notification command
- **WHEN** conformance sends a valid `NOTIFY <HEX>` command
- **THEN** the firmware transmits the exact bytes as a BLE notification

#### Scenario: Invalid control command
- **WHEN** the firmware receives malformed or unsupported serial input
- **THEN** it reports an error without changing BLE or LED state

### Requirement: Minimal serial output protocol
The firmware SHALL emit only bounded `READY`, `CONNECTED`, `DISCONNECTED`, `WRITE <HEX>`, and `ERROR <CODE>` events required by canonical conformance.

#### Scenario: Connection lifecycle
- **WHEN** a BLE client connects and later disconnects
- **THEN** the firmware emits `CONNECTED` followed by `DISCONNECTED`

### Requirement: Controllable diagnostic LED
`LED <RRGGBB>` SHALL accept exactly six hexadecimal digits and set the onboard RGB LED to that value; `LED 000000` SHALL turn it off.

#### Scenario: Failure indication
- **WHEN** conformance sends `LED FF0000`
- **THEN** the onboard LED becomes red

### Requirement: Bounded firmware behavior
Serial input, serial output, GATT writes, notification payloads, command lengths, hex decoding, and BLE queues SHALL be bounded.

#### Scenario: Oversized input
- **WHEN** serial or GATT input exceeds its fixed bound
- **THEN** the firmware rejects it, reports an error where possible, and continues without unbounded allocation
