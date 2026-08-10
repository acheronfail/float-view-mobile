## ADDED Requirements

### Requirement: Android Rust firmware identity cutover evidence

Android firmware identity ownership SHALL NOT be considered transferred until focused safe-Rust, JNI, Kotlin-adapter, dependency, symbol, source-deletion, and clean packaging checks; the complete `just verify` software gate; and focused ESP32 physical identity runs on Android, iOS, and macOS exercise Android's Rust decoder without a Kotlin fallback and preserve Apple's established Rust path. Evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, version/name/private UUID/test/type/configuration/CRC fields, replacement UTF-8, wrong commands, structural truncation and tail failures, ABI mismatch, null/alias/array-shape/output-bound failures, pending exceptions, panic containment, invalid result records, and private-data-safe diagnostics.

A missing location observation during a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal a permission, capability, monotonic-boundary, BLE, session, identity, route, or cleanup failure.

#### Scenario: Verify compiled Android bridge parity

- **WHEN** focused Rust and JVM tests exercise source-derived and malformed identities through the compiled JNI ABI
- **THEN** fixed version/status/record behavior, field mapping, established Kotlin errors, bounds, redaction, exception handling, and panic containment agree with the safe decoder

#### Scenario: Verify Android software production selection

- **WHEN** clean host tests, four-ABI debug and release packaging, canonical checks, and `just verify` run after cutover
- **THEN** Android identity tests execute Rust through the linked two-symbol bridge and no Kotlin structural parser, fallback, generated binding, committed binary, or extra identity symbol is present

#### Scenario: Verify Android physical identity selection

- **WHEN** focused physical identity conformance runs on the connected Android device against fragmented ESP32 fixture responses
- **THEN** Android reaches ready, exposes the expected direct or CAN identity and route, and cleans up through the production Rust identity decoder

#### Scenario: Preserve Apple physical identity regression

- **WHEN** the same focused physical identity scenario runs on the connected iOS device and macOS host
- **THEN** both Apple hosts pass through their unchanged Rust identity decoder and Apple bridge without linking or calling the Android JNI artifact

#### Scenario: Delete superseded Kotlin parser

- **WHEN** focused, packaging, canonical, and all-host physical checks pass
- **THEN** the Kotlin common-prefix, terminated-name, hardware-tail, and CRC-tail implementation is absent and the complete software gate remains green
