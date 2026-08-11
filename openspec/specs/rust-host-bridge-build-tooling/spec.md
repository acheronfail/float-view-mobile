# Rust Host Bridge Build Tooling Specification

## Purpose

Define the shared, allowlisted source-build mechanics for the retained Apple FFI and Android JNI bridge crates without changing their independent artifacts or ABIs.

## Requirements

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

### Requirement: Typed Android Gradle bridge registration

Android Gradle SHALL define exactly one closed typed integration entry for each of the six retained Rust bridge families and SHALL use generic factories to register each existing Debug/Release four-ABI package task and debug host-JNI task. The entries SHALL be the only Gradle source for bridge identifiers, public task suffixes, domain/JNI source inputs, generated output directories, host library stems, and JVM test properties. Configuration and repository audits SHALL reject duplicate or missing identities, reintroduced bridge-specific factories, and any task-name, input, output, profile, dependency, or property drift.

#### Scenario: Register the exact task surface

- **WHEN** Gradle configures the Android application
- **THEN** it registers exactly the existing twelve package tasks and six host-JNI tasks with unchanged public names, groups, builder arguments, profiles, inputs, outputs, and clean ordering

#### Scenario: Wire every downstream consumer

- **WHEN** Android source sets, JVM tests, or Debug/Release JNI merge tasks consume Rust bridge artifacts
- **THEN** they derive all six disjoint directories, host dependencies, absolute library properties, and profile task dependencies from the same typed entries without a bridge-specific parallel list

#### Scenario: Reject catalog drift

- **WHEN** a Gradle bridge entry is missing or duplicates an identifier, suffix, output, library stem, or test property, or a bridge-specific registration path reappears
- **THEN** configuration or structural audit fails before packaging can silently omit, overwrite, or load the wrong artifact

#### Scenario: Preserve all-host behavior

- **WHEN** linked host tests, clean Android four-ABI Debug/Release packaging, complete software verification, and applicable Android/iOS/macOS ESP32 fixture scenarios run after task deduplication
- **THEN** every bridge ABI, JNI symbol, artifact, production call path, BLE/protocol/control/BMS/background assertion, session behavior, and cleanup remains unchanged, with only bounded-time location sample absence eligible for manual deferral
