## ADDED Requirements

### Requirement: Minimal bridge conformance

Verification SHALL cover the exact production bridge direction, command and event kinds, ABI/build identity, bounded caller buffers, insufficient-capacity retry, command ordering, duplicate and stale completion rejection, event bounds, stop/wake behavior, panic containment, and runtime restart on Apple and Android targets. Test-only reverse codecs MUST NOT be required to exercise these properties.

#### Scenario: Production-direction test

- **WHEN** bridge verification submits native observations and receives Rust commands
- **THEN** it uses the same focused event entry points and command decoder as production and observes no host command encoder or host event decoder in production sources

#### Scenario: Exhaustive command reachability

- **WHEN** verification compares the command vocabulary with production request sites and host dispatch branches
- **THEN** every command has at least one production requester and one bounded completion path and no unreachable command remains

### Requirement: Stable VESC UUID conformance

Canonical software and host-adapter tests SHALL prove that the same meaningful VESC firmware UUID resolves one saved-board identity across reconnect, WebView or Activity replacement, and process recreation; remains distinct from another firmware UUID; never depends on a runtime discovery handle; and never appears in public or diagnostic outputs.

#### Scenario: Saved-board process restart

- **WHEN** conformance saves and connects a VESC, recreates the native process, reconnects through its private platform association, and receives the same firmware UUID
- **THEN** the backend resolves the existing board, preserves saved tracking semantics, and creates neither a duplicate board nor a new random identity

#### Scenario: UUID privacy sweep

- **WHEN** conformance captures state, results, errors, diagnostics, logs, archives, and exports after UUID-bearing operations
- **THEN** none contains the UUID bytes or textual representation

### Requirement: Rust software-host parity

The canonical suite SHALL run the required simulator, emulator, and browser-development software scenarios through the Rust software host and SHALL verify equivalent observable behavior on Apple and Android without native software peripheral implementations.

#### Scenario: Software-host request transcript

- **WHEN** equivalent Apple and Android software scenarios discover, connect, exchange VESC requests, disconnect, and restart
- **THEN** the Rust software host records the same bounded request classification and produces equivalent contract state and results

### Requirement: Native host surface audit

The complete software gate SHALL report production Swift and Kotlin counts separately from tests, generated files, dependencies, and build outputs and SHALL fail when banned superseded paths or duplicate ownership types return. Line counts SHALL be informational; ownership violations and banned paths SHALL be normative.

#### Scenario: Superseded path reintroduced

- **WHEN** a Swift or Kotlin software VESC peripheral, general-purpose reverse bridge codec, random device-token map, or unreachable bridge command is added after cutover
- **THEN** verification fails with the offending owned path or symbol

#### Scenario: Capability inventory is incomplete

- **WHEN** verification compares iOS, macOS, and Android capability inventories with their required entry files and focused evidence
- **THEN** it fails for every missing row or file, unexplained unsupported capability, incomplete capability, or entry that does not identify its implementation owner

#### Scenario: Host audit runs on macOS

- **WHEN** `just verify` executes the host-surface audit with the repository-supported macOS shell environment
- **THEN** the audit completes without relying on unavailable shell builtins and applies the same ownership rules as other development environments

### Requirement: Native document conformance

Canonical and focused host verification SHALL cover native import and export presentation, operation ordering, cancellation, bounded copying, invalid staging paths, oversized input, Activity/window/WebView replacement, and staging cleanup without moving archive semantics into Swift or Kotlin.

#### Scenario: Native document round trip

- **WHEN** each supported native host imports a valid bounded archive and exports the resulting session through its platform surface
- **THEN** Rust produces equivalent session state and export content while the host performs only native presentation and bounded file access

#### Scenario: Native document cancellation and cleanup

- **WHEN** import or export is cancelled before, during, or after presentation replacement
- **THEN** the operation terminates exactly once, no partial import is visible, and no expired staging file remains

### Requirement: Background and location conformance

Focused and physical verification SHALL prove effective background execution, BLE continuity or restoration, location authorization propagation, bounded background coordinates, foreground-service or background-mode configuration, and durable Rust-owned logging on every platform that declares support.

#### Scenario: Background ride logging

- **WHEN** a supported mobile host begins a saved ride, backgrounds or replaces its frontend, receives BLE telemetry and location, and later returns to foreground
- **THEN** one process-owned Rust runtime persists the continuous bounded session and the host reports no fabricated eligibility or duplicated application policy

#### Scenario: Authorization changes without coordinates

- **WHEN** location authorization changes and no location sample follows
- **THEN** Rust still receives and publishes the new authorization through the production focused entry point

### Requirement: Effective appearance conformance

Each native host SHALL verify initial effective appearance, a live platform appearance change, background/foreground return, and WebView recreation using exactly its declared appearance source.

#### Scenario: Appearance source remains singular

- **WHEN** appearance conformance exercises one host
- **THEN** the frontend and native-only chrome reach the expected effective light or dark state and verification observes neither a duplicate source nor a persisted-preference feedback loop
