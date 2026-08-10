## ADDED Requirements

### Requirement: Allowlisted bridge build catalog

The repository SHALL define the four retained Rust bridge families exactly once for build tooling. Each catalog entry SHALL bind a stable bridge identifier to its existing Cargo package, Apple archive name, Android shared-library name, diagnostic label, JNI symbol prefix, and exact JNI application-symbol allowlist. The catalog SHALL reject unknown identifiers and SHALL NOT discover packages, libraries, or symbols from untrusted command input, generated bindings, or committed native artifacts.

#### Scenario: Select every retained bridge

- **WHEN** shared tooling selects frame, firmware identity, custom configuration, or BMS values
- **THEN** it resolves the existing bridge-specific Cargo package, artifact name, diagnostic label, and exact Android JNI symbols without changing any ABI or package identity

#### Scenario: Reject an unknown bridge

- **WHEN** either shared builder receives an identifier absent from the catalog
- **THEN** it exits before invoking Cargo, creating output, or accepting caller-supplied package, library, or symbol metadata

### Requirement: Shared Apple bridge source builder

One shared Apple builder SHALL build every catalogued Apple bridge from locked offline source for macOS, iOS device, and iOS simulator architectures. It SHALL preserve Debug/Release mapping, pinned Rust target checks, single-architecture copy and multi-architecture `lipo` behavior, exact archive names and output directories, actionable missing-toolchain/target diagnostics, and absence of committed archives. No bridge-local Apple build helper SHALL remain.

#### Scenario: Build every Apple target family

- **WHEN** root recipes or an iOS Xcode phase requests a catalogued bridge for macOS, iOS device, or iOS simulator in Debug or Release
- **THEN** the shared builder produces the same named static archive with the requested architecture set at the existing output path

#### Scenario: Reject invalid Apple build input

- **WHEN** the Apple builder receives an unsupported SDK, architecture, configuration, empty architecture set, unknown bridge, or missing pinned toolchain/target
- **THEN** it fails with an actionable diagnostic and does not substitute, download, or publish another artifact

### Requirement: Shared Android bridge source builder

One shared Android builder SHALL build every catalogued JNI bridge from locked offline source for host JVM tests and `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` Android packages in Debug and Release. It SHALL preserve Rust 1.93.0, NDK 28.2.13676358, API 29, target/linker mapping, exact library and output names, architecture checks, and the exact per-bridge JNI symbol allowlist including rejection of `JNI_OnLoad` or extra application symbols. No bridge-local Android build helper SHALL remain.

#### Scenario: Build host and Android artifacts

- **WHEN** Gradle or root verification requests a catalogued bridge for a supported host profile or Android ABI/profile
- **THEN** the shared builder produces, installs, architecture-checks, and symbol-audits the same existing library in the same disjoint output directory

#### Scenario: Reject invalid Android build input

- **WHEN** the Android builder receives an unknown bridge, mode, ABI, profile, missing pinned Rust target, missing or mismatched pinned NDK, unavailable linker, wrong architecture, or symbol drift
- **THEN** it fails closed without committing, packaging, or silently substituting a native library

### Requirement: Build integration and regression equivalence

Root `just`, Android Gradle, and iOS Xcode SHALL invoke the shared builders with explicit catalog identifiers while preserving public recipe/task/phase names, bridge-specific source inputs, generated output paths, runtime link names, and clean/incremental rebuild behavior. Repository verification SHALL require exactly the two shared executable builders and one non-executable catalog, reject bridge-local helper reintroduction, and demonstrate that all eight existing bridge artifacts, ABIs, symbols, packages, and production call paths remain unchanged.

#### Scenario: Rebuild after shared tooling changes

- **WHEN** the shared catalog or relevant shared builder changes
- **THEN** every affected Gradle or Xcode bridge task treats the shared file as an input and rebuilds its existing output without altering unrelated source ownership

#### Scenario: Verify software packaging equivalence

- **WHEN** clean Apple and Android Debug/Release packaging, focused ABI/JNI checks, audits, and `just verify` run after migration
- **THEN** all existing bridge archives and four-ABI JNI libraries retain their exact names, symbols, architectures, dependency graphs, and selected host behavior while all eight copied helpers are absent

#### Scenario: Verify physical bridge regression

- **WHEN** focused control and BMS scenarios run against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host, with mobile background continuity also exercised
- **THEN** production BLE, framing, firmware identity, custom configuration, BMS values, application state, session work, and cleanup remain successful through the artifacts built by the shared tooling

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, state, session, background, and cleanup failures remain blocking
