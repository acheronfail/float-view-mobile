## ADDED Requirements

### Requirement: Authoritative flywheel transition confirmation ordering

The native application service SHALL apply each current-generation authoritative telemetry sample to the sole mutation coordinator before enforcing exclusions implied by the sample's package mode. A pending flywheel enable SHALL complete successfully when telemetry first reports flywheel mode. A pending flywheel disable SHALL remain pending across repeated flywheel-mode telemetry and SHALL complete only when later authoritative telemetry reports non-flywheel mode. Every other pending mutation SHALL fail when newly authoritative flywheel mode prohibits it, and no late cancellation or timeout callback may replace a completed result.

#### Scenario: Confirm flywheel enable

- **WHEN** command 22 enable is pending and current-generation telemetry reports flywheel package mode
- **THEN** the action completes successfully with flywheel enabled before flywheel-mode exclusions are applied

#### Scenario: Preserve flywheel disable

- **WHEN** command 22 disable is pending and another current-generation sample still reports flywheel mode
- **THEN** the action remains pending and eligible until normal-mode telemetry confirms the transition or the existing bounded deadline expires

#### Scenario: Confirm flywheel disable

- **WHEN** an eligible pending flywheel disable later receives current-generation normal-mode telemetry
- **THEN** the action completes successfully with flywheel disabled and no timeout diagnostic

#### Scenario: Fail prohibited pending mutation

- **WHEN** authoritative telemetry enters flywheel mode while a non-flywheel mutation is still pending
- **THEN** that mutation fails once, its marker is cleared, and no additional prohibited command is transmitted

#### Scenario: Late failure callback

- **WHEN** a cancellation or timeout callback arrives after the matching Refloat action already completed or when none is pending
- **THEN** it is a no-op that publishes no false timeout and does not alter current control state

