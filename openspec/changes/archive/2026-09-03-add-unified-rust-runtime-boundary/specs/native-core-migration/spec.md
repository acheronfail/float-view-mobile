## MODIFIED Requirements

### Requirement: Bounded incremental Rust extraction

Each OpenSpec change that moves backend behavior into Rust SHALL name one bounded behavioral slice, its existing production owner, its parity evidence, its integration state, and the checkpoint at which superseded production code may be deleted. A change MUST NOT combine unproven protocol, application, persistence, server, Bluetooth, or platform-lifecycle behavior. After independently proven protocol, coordination, and persistence candidates exist, one bounded composition change MAY introduce the safe coarse application-runtime component required to select those existing owners without adding another host or protocol bridge. Following that composition change, further migration work MUST be a deletion-bearing host cutover or final bridge retirement rather than another sideways candidate extraction.

#### Scenario: Start a Rust extraction

- **WHEN** a backend behavior is first implemented under `backends/core`
- **THEN** the change identifies the exact behavior and existing owner and supplies focused parity checks without claiming unrelated ownership

#### Scenario: Reject an umbrella migration

- **WHEN** a proposed step combines multiple unproven backend layers or any OS integration
- **THEN** it is divided into separately verifiable OpenSpec changes before implementation

#### Scenario: Compose proven candidates

- **WHEN** framing, protocol, coordination, and session components have independent parity evidence and the migration requires one production-selectable seam
- **THEN** one safe runtime candidate may compose them through closed commands and effects while adding no host binding or OS owner

#### Scenario: Freeze sideways extraction after composition

- **WHEN** the coarse runtime candidate passes its complete software and isolation gates
- **THEN** the next migration changes select and delete one host at a time and no new protocol-specific or application-candidate bridge is added

## ADDED Requirements

### Requirement: Native core migration must converge

The native-core migration SHALL measure its production source and bridge/build overhead before runtime composition. Apple and Android cutovers MUST each remove the superseded selected native decision owners in the same verified change, and completion MUST remove every temporary per-protocol bridge and build entry after both hosts select one coarse runtime boundary.

#### Scenario: Record the convergence baseline

- **WHEN** the application-runtime candidate is proposed
- **THEN** the repository records reproducible production LOC, workspace-member, bridge-crate, selected-family, and build-artifact counts plus exact future deletion paths

#### Scenario: Cut over one host

- **WHEN** a host passes focused, packaging, canonical, and required physical gates through the coarse runtime
- **THEN** its superseded native state machines and protocol/persistence adapters are deleted without a production fallback and the measured total does not increase

#### Scenario: Retire migration scaffolding

- **WHEN** both production hosts select the coarse runtime
- **THEN** all temporary protocol bridge crates, loaders, C modules, headers, build-catalog entries, and obsolete audits are deleted while OS-owned host responsibilities remain native
