## ADDED Requirements

### Requirement: Explain configuration operation failures

Configuration and control diagnostics SHALL retain admission, completion and rejection context: failure layer/category, authoritative write blocker, readiness, pending work, revision and identity match facts, and transport/application correlation when available. Context SHALL be captured at the operation boundary without raw errors, identity contents, values or payloads. Public error behavior and mutation authorization SHALL remain unchanged.

#### Scenario: Rejected configuration write

- **WHEN** a write is rejected by readiness, safety, pending work, identity or revision checks
- **THEN** diagnostics distinguish those conditions using closed codes and structural facts from that attempt, including missing or unknown facts

#### Scenario: Accepted operation fails later

- **WHEN** admitted configuration work fails during scheduling, acknowledgement, readback, timeout or connection replacement
- **THEN** diagnostics retain its bounded correlation, available failure category and completion outcome so rejection can be distinguished from a failed accepted operation

#### Scenario: Coalesced late rejection

- **WHEN** a rejection joins an incident after its subsequent-event window closes
- **THEN** its failure context remains in the exported incident within existing event and byte bounds, without reopening the window

#### Scenario: Private request inputs

- **WHEN** identity strings, field names, values or payloads contain private text
- **THEN** failure records and exported ZIPs contain only allowlisted codes, counts and match flags, retaining existing privacy exclusions

#### Scenario: Audited prerelease context

- **WHEN** a connected controller reports an audited stable, beta2 or beta3 Refloat release
- **THEN** diagnostic compatibility context retains numeric version components and its closed release suffix without accepting arbitrary firmware text
