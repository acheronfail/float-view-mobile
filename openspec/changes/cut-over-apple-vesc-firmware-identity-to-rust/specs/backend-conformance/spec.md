## ADDED Requirements

### Requirement: Apple Rust firmware identity cutover evidence

Apple firmware identity ownership SHALL NOT be considered transferred until focused safe-Rust, C-ABI, Swift-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical identity runs on iOS and macOS exercise the Rust decoder without a Swift fallback. Evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, version/name/private UUID/test/type/configuration/CRC fields, replacement UTF-8, wrong commands, structural truncation and tail failures, ABI mismatch, invalid pointer/capacity/overlap/result regions, output bounds, panic containment, and private-data-safe diagnostics.

Android SHALL pass the same focused physical identity regression through its unchanged Kotlin decoder and SHALL remain outside the Apple bridge and packaging graph. A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, identity, or cleanup failure.

#### Scenario: Verify compiled Apple bridge parity

- **WHEN** focused Rust, C, and Swift tests exercise source-derived and malformed identities through the compiled Apple ABI
- **THEN** fixed version/status/record behavior, field mapping, established Swift errors, bounds, redaction, and panic containment agree with the safe decoder

#### Scenario: Verify Apple software production selection

- **WHEN** clean macOS, iOS simulator, and unsigned device packaging plus `just verify` run after cutover
- **THEN** shared Apple identity tests execute Rust through the linked two-symbol bridge and no Swift structural parser, fallback, generated binding, or extra symbol is present

#### Scenario: Verify Apple physical identity selection

- **WHEN** focused physical identity conformance runs on the connected iOS device and macOS host against fragmented ESP32 fixture responses
- **THEN** both shared Apple hosts reach ready, expose the expected direct/CAN identity and route, and clean up through the production Rust identity decoder

#### Scenario: Preserve Android regression

- **WHEN** the same focused physical identity scenario runs on the connected Android device
- **THEN** Android passes through its unchanged Kotlin identity decoder without linking or calling the Apple bridge

#### Scenario: Delete superseded Swift parser

- **WHEN** focused, packaging, canonical, and physical checks pass
- **THEN** the Swift common-prefix, terminated-name, hardware-tail, and CRC-tail implementation is absent and the complete software gate remains green
