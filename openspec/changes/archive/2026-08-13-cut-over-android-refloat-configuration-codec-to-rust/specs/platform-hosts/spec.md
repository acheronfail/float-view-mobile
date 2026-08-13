## ADDED Requirements

### Requirement: Android Rust Refloat configuration codec bridge ownership

The repository SHALL provide one temporary, stateless, source-built Android bridge that exposes the safe Rust Refloat configuration codec without moving OS or stateful application ownership.

#### Scenario: Expose exactly the narrow codec surface

- **WHEN** the JNI ABI is inspected
- **THEN** it SHALL export exactly ABI-version, schema-decode, values-decode, and values-encode methods
- **AND** it SHALL expose no Rust allocator, destructor, callback, scheduler, database, host singleton, generic dispatcher, or long-lived handle

#### Scenario: Exchange bounded canonical records

- **WHEN** Kotlin invokes a decode or encode operation
- **THEN** all inputs, outputs, and produced-length metadata SHALL use JVM-owned bounded primitive arrays
- **AND** schema and value metadata SHALL use safe-crate-owned versioned canonical records
- **AND** the bridge SHALL retain no JVM reference, pointer, record, or state after the call

#### Scenario: Publish output atomically with exact capacity

- **WHEN** an output array is empty or smaller than the valid result
- **THEN** the bridge SHALL publish only the exact bounded required length and a fixed insufficient-output status
- **AND** a repeated call with that exact capacity SHALL publish the complete deterministic result without partial bytes from either call

#### Scenario: Validate every JNI boundary

- **WHEN** the bridge receives references, arrays, capacities, schema records, value records, or result metadata
- **THEN** it SHALL validate pending exceptions, nullability, exact result length, maximum sizes, reference distinctness, record version/kind/length/count/reserved fields, UTF-8, scalar canonicality, and schema/value consistency before use
- **AND** it SHALL contain panics and return deterministic payload-free statuses

#### Scenario: Build every Android target from locked source

- **WHEN** Gradle builds host-JVM, arm64-v8a, armeabi-v7a, x86, or x86_64 Debug or Release outputs
- **THEN** the locked Rust source SHALL produce and package the exact JNI library through the typed bridge catalogue
- **AND** clean builds SHALL require no committed or generated language binding or native artifact
- **AND** source, support, catalogue, toolchain, and lockfile changes SHALL invalidate the build output

#### Scenario: Remove superseded Kotlin protocol ownership

- **WHEN** focused adapter and selection evidence passes
- **THEN** Android SHALL delete its Kotlin Qt decompressor, XML parser, schema fingerprint and wire-signature construction, value byte reader/writer, scale conversion, string framing, and CRC implementation
- **AND** source audits SHALL reject reintroduction or fallback selection of those algorithms

#### Scenario: Preserve host boundaries and Apple isolation

- **WHEN** the bridge is integrated
- **THEN** Kotlin SHALL retain the stateful configuration service and application ownership
- **AND** Android SHALL retain BLE, location, background work, files, theme, lifecycle, permissions, WebView, persistence, and scheduling ownership
- **AND** Apple's selected codec, bridge, adapter, packages, archives, and symbols SHALL remain unchanged

#### Scenario: Mark both codec bridges as temporary migration scaffolding

- **WHEN** maintainers inspect the bridge catalogues and ownership documentation
- **THEN** they SHALL identify both Android and Apple codec bridges as removable after unified Rust runtime cutover
- **AND** they SHALL NOT treat the canonical records as a generated or permanent platform API
