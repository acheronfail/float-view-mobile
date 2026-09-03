## ADDED Requirements

### Requirement: Both mobile hosts complete unified runtime selection

Apple and Android SHALL select the same safe application runtime through thin platform bindings, with no protocol-specific bridge or native application/protocol/session-decision fallback remaining.

#### Scenario: Complete the migration

- **WHEN** Android passes its deletion, packaging, software, convergence, and requested iOS physical-regression gates
- **THEN** both mobile hosts share the unified runtime and the temporary protocol bridge/build matrix is fully retired
