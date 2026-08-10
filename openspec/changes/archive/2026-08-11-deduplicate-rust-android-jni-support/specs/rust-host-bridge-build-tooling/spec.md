## MODIFIED Requirements

### Requirement: Shared Android bridge source builder

One shared Android builder SHALL build every catalogued JNI bridge and its shared Android JNI support dependency from locked offline source for host JVM tests and `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` Android packages in Debug and Release. It SHALL preserve Rust 1.93.0, NDK 28.2.13676358, API 29, target/linker mapping, exact library and output names, architecture checks, and the exact per-bridge JNI symbol allowlist including rejection of `JNI_OnLoad` or extra application symbols. No bridge-local Android build helper SHALL remain.

#### Scenario: Build host and Android artifacts

- **WHEN** Gradle or root verification requests a catalogued bridge for a supported host profile or Android ABI/profile
- **THEN** the shared builder compiles the bridge and shared support dependency and produces, installs, architecture-checks, and symbol-audits the same existing library in the same disjoint output directory

#### Scenario: Reject invalid Android build input

- **WHEN** the Android builder receives an unknown bridge, mode, ABI, profile, missing pinned Rust target, missing or mismatched pinned NDK, unavailable linker, wrong architecture, or symbol drift
- **THEN** it fails closed without committing, packaging, or silently substituting a native library

### Requirement: Build integration and regression equivalence

Root `just`, Android Gradle, and iOS Xcode SHALL invoke the shared builders with explicit catalog identifiers while preserving public recipe/task/phase names, bridge-specific source inputs, shared Apple FFI support inputs, shared Android JNI support inputs, generated output paths, runtime link names, and clean/incremental rebuild behavior. Repository verification SHALL require exactly the two shared executable builders and one non-executable catalog, reject bridge-local helper reintroduction, and demonstrate that all twelve existing bridge artifacts, ABIs, symbols, packages, and production call paths remain unchanged.

#### Scenario: Rebuild after shared tooling changes

- **WHEN** the shared catalog, relevant shared builder, shared Apple FFI support manifest/source, or shared Android JNI support manifest/source changes
- **THEN** every affected Gradle or Xcode bridge task treats the shared file as an input and rebuilds its existing output without altering unrelated source ownership

#### Scenario: Verify software packaging equivalence

- **WHEN** clean Apple and Android Debug/Release packaging, focused ABI/JNI checks, audits, and `just verify` run after migration
- **THEN** all existing bridge archives and four-ABI JNI libraries retain their exact names, symbols, architectures, dependency graphs, and selected host behavior while bridge-local build helpers, Apple FFI safety primitive copies, and Android JNI safety primitive copies are absent

#### Scenario: Verify physical bridge regression

- **WHEN** focused control and BMS scenarios run against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host, with mobile background continuity also exercised
- **THEN** production BLE, framing, firmware identity, custom configuration, BMS values, controller data, Refloat control, application state, session work, and cleanup remain successful through the artifacts built by the shared tooling

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, state, session, background, and cleanup failures remain blocking
