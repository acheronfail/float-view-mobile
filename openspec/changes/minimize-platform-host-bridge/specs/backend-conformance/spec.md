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
