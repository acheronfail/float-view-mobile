## MODIFIED Requirements

### Requirement: Allowlisted bridge build catalog

The repository SHALL define the six retained Rust bridge families exactly once for build tooling. Each catalog entry SHALL bind a stable bridge identifier to its existing Cargo package, Apple archive name, Android shared-library name, diagnostic label, JNI symbol prefix, and exact JNI application-symbol allowlist. The catalog SHALL reject unknown identifiers and SHALL NOT discover packages, libraries, or symbols from untrusted command input, generated bindings, or committed native artifacts.

#### Scenario: Select every retained bridge

- **WHEN** shared tooling selects frame, firmware identity, custom configuration, BMS values, controller data, or Refloat control
- **THEN** it resolves the existing bridge-specific Cargo package, artifact name, diagnostic label, and exact Android JNI symbols without changing any ABI or package identity

#### Scenario: Reject an unknown bridge

- **WHEN** either shared builder receives an identifier absent from the catalog
- **THEN** it exits before invoking Cargo, creating output, or accepting caller-supplied package, library, or symbol metadata

### Requirement: Shared Apple bridge source builder

One shared Apple builder SHALL build every catalogued Apple bridge and its shared Apple FFI support dependency from locked offline source for macOS, iOS device, and iOS simulator architectures. It SHALL preserve Debug/Release mapping, pinned Rust target checks, single-architecture copy and multi-architecture `lipo` behavior, exact archive names and output directories, actionable missing-toolchain/target diagnostics, and absence of committed archives. No bridge-local Apple build helper SHALL remain.

#### Scenario: Build every Apple target family

- **WHEN** root recipes or an iOS Xcode phase requests a catalogued bridge for macOS, iOS device, or iOS simulator in Debug or Release
- **THEN** the shared builder compiles the bridge and shared support dependency and produces the same named static archive with the requested architecture set at the existing output path

#### Scenario: Reject invalid Apple build input

- **WHEN** the Apple builder receives an unsupported SDK, architecture, configuration, empty architecture set, unknown bridge, or missing pinned toolchain/target
- **THEN** it fails with an actionable diagnostic and does not substitute, download, or publish another artifact

### Requirement: Build integration and regression equivalence

Root `just`, Android Gradle, and iOS Xcode SHALL invoke the shared builders with explicit catalog identifiers while preserving public recipe/task/phase names, bridge-specific source inputs, shared Apple FFI support inputs, generated output paths, runtime link names, and clean/incremental rebuild behavior. Repository verification SHALL require exactly the two shared executable builders and one non-executable catalog, reject bridge-local helper reintroduction, and demonstrate that all twelve existing bridge artifacts, ABIs, symbols, packages, and production call paths remain unchanged.

#### Scenario: Rebuild after shared tooling changes

- **WHEN** the shared catalog, relevant shared builder, or shared Apple FFI support manifest/source changes
- **THEN** every affected Gradle or Xcode bridge task treats the applicable shared file as an input and rebuilds its existing output without altering unrelated source ownership

#### Scenario: Verify software packaging equivalence

- **WHEN** clean Apple and Android Debug/Release packaging, focused ABI/JNI checks, audits, and `just verify` run after migration
- **THEN** all existing bridge archives and four-ABI JNI libraries retain their exact names, symbols, architectures, dependency graphs, and selected host behavior while bridge-local build helpers and Apple FFI safety primitive copies are absent

#### Scenario: Verify physical bridge regression

- **WHEN** focused control and BMS scenarios run against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host, with mobile background continuity also exercised
- **THEN** production BLE, framing, firmware identity, custom configuration, BMS values, controller data, Refloat control, application state, session work, and cleanup remain successful through the artifacts built by the shared tooling

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, state, session, background, and cleanup failures remain blocking
