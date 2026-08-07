## ADDED Requirements

### Requirement: Reproducible portable Rust toolchains

Repository setup SHALL install or validate the pinned Rust toolchain and target components, Cargo cross-compilation/packaging tools, Android NDK compatibility, application JDK, checked-in `Cargo.lock`, exact Rust dependency resolution, pinned `cbindgen` identity, checked-in generated C-header hash, and native packaging configuration required by the checked-in build definitions. Development commands MUST fail early with actionable expected/actual diagnostics rather than downloading an unpinned tool, changing the lockfile, reading a mutable developer-local repository, regenerating the header implicitly, or falling back to a system dependency during application compilation.

#### Scenario: Rust prerequisites ready

- **WHEN** a developer completes repository setup and starts a core or host build
- **THEN** orchestration validates the pinned Rust toolchain, required target components, locked dependency graph, generated header identity, and target-specific linker/packaging prerequisites before compilation

#### Scenario: Android prerequisites ready

- **WHEN** a developer completes repository setup and runs the Android build
- **THEN** Gradle locates the pinned compatible NDK, application JDK, Rust targets, selected locked Cargo/NDK integration tool, checked-in JNI wrapper, and matching C-ABI identity without generator execution or manual shell preparation

#### Scenario: Apple prerequisites ready

- **WHEN** a developer builds macOS or iOS
- **THEN** Xcode or the root recipe locates the pinned Rust target components, checked-in header/module map, selected static-library packaging path, and matching ABI build identity before linking the host

#### Scenario: Explicit C-header regeneration

- **WHEN** a maintainer runs `just generate core-header`
- **THEN** it uses the Cargo-locked `cbindgen` version and canonical Rust ABI sources to reproduce the checked-in C header and hash without changing unrelated dependencies or requiring a mutable global installation

#### Scenario: Rust dependency resolution mismatch

- **WHEN** Cargo would change the checked-in lockfile or resolve a dependency outside its recorded identity
- **THEN** setup or verification stops with the dependency, expected identity, actual condition, and corrective command

#### Scenario: Toolchain mismatch

- **WHEN** the installed Rust toolchain, target component, NDK, JDK, `cbindgen`, or native packaging tool does not match the repository requirement
- **THEN** setup or build stops before host packaging with expected and actual versions plus the corrective command

### Requirement: One portable backend development build

The root development and verification commands SHALL build the Rust backend through its Cargo workspace and locked dependency graph and reuse that source/configuration through host-specific packaging. Normal application builds SHALL consume the checked-in C header and hand-written host wrappers and MUST NOT require network access, header generation, mutable local package repositories, committed native binaries, separate Apple-backend or Android-Kotlin-backend source configurations after cutover, or system-SQLite compile configurations.

#### Scenario: Shared source edit

- **WHEN** a developer edits Rust protocol, application, server, storage, or bridge code and starts a native development target
- **THEN** the selected host rebuilds and runs the same Cargo products and features used by its production build

#### Scenario: Host-only edit

- **WHEN** a developer edits only a Swift/Kotlin platform adapter, host presentation seam, or wrapper
- **THEN** the host rebuild does not require a translated application-behavior change in another language or regeneration of the C header when the ABI is unchanged

#### Scenario: Offline normal build

- **WHEN** setup has already fetched the locked Rust sources and a developer runs a normal host build without network access
- **THEN** the core and host build from local locked inputs without contacting a registry, running `cbindgen`, or consulting a developer-local repository

## MODIFIED Requirements

### Requirement: Platform-selected native development

The repository SHALL provide `just dev mock|macos|ios|android [target]` as the public hot-reload entry point. A native selection MUST run the shared frontend in a development-host browser against exactly one selected real native process using the Rust backend, while `mock` MUST run the deterministic mock backend. Android orchestration SHALL build/package the Rust library and JNI wrapper, and Apple orchestration SHALL build/link the Rust static product and C-ABI wrapper, without exposing those steps as a second public workflow.

#### Scenario: macOS frontend iteration

- **WHEN** a developer runs `just dev macos`
- **THEN** Vite hot module replacement serves the shared frontend in the host browser while the macOS process owns Apple adapters/native presentation and the Rust runtime owns application, storage, and server paths

#### Scenario: Selected mobile target

- **WHEN** a developer runs `just dev ios <target>` or `just dev android <target>`
- **THEN** the selected installed application owns its platform adapters and process Rust runtime while the host browser uses that backend through the existing forwarding transport

#### Scenario: Deterministic mock

- **WHEN** a developer runs `just dev mock`
- **THEN** Vite starts with the deterministic mock backend and no native application or Rust core library is built or launched
