## ADDED Requirements

### Requirement: Source-faithful lighting status queries

For every audited Refloat lighting wire family, a route-correct, shape-valid lighting request whose complete mutation mask is zero SHALL be treated as a read-only status query. The current LED and headlight state SHALL be returned without changing either value. Query flags MUST NOT be interpreted as desired state, and query availability MUST remain independent of idle, riding, or flywheel package mode. Any nonzero supported mask SHALL remain a mutation subject to all existing access and flywheel exclusions.

#### Scenario: Legacy zero-mask query

- **WHEN** an audited Refloat 1.0.x or 1.1.x peer sends exact command 202 with mask zero
- **THEN** current lighting state is returned and simulated lighting state remains unchanged

#### Scenario: Stable zero-mask query

- **WHEN** an audited Refloat 1.2.x peer sends exact command 20 with its complete four-byte mask equal to zero
- **THEN** current lighting state is returned and simulated lighting state remains unchanged

#### Scenario: Query while flywheel is active

- **WHEN** a bounded lighting status refresh with mask zero occurs while authoritative package mode is flywheel
- **THEN** the read succeeds without mutation while every nonzero-mask lighting request remains rejected

