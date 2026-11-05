## ADDED Requirements

### Requirement: Portable recording identity survives import

A supported session archive's recording UUID SHALL survive import and subsequent re-export. Import of an existing recording ID on the same board SHALL retain the destination recording in full, including its pruning state, and reuse its ID. An existing ID belonging to another board SHALL fail without reassignment. Existing supported stored sessions SHALL remain intact; earlier duplicates with unrelated IDs SHALL NOT be inferred or merged by timestamps or sample similarity.

#### Scenario: Re-import an individual archive

- **WHEN** a session archive is imported twice or exported again after import
- **THEN** its recording ID remains stable and importing it again creates no second session
