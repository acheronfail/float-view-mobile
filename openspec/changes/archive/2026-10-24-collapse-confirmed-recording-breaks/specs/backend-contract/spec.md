## ADDED Requirements

### Requirement: Confirmed recording break metadata

`session.get` SHALL include `recordingBreaks`, a chronologically ordered array of at most 1,000 nonoverlapping `[startElapsedMs, endElapsedMs]` pairs containing integer elapsed milliseconds. Each interval SHALL identify the last recorded sample before an explicit recording boundary and its first resumed sample, within the published sample and event boundaries. Ordinary sampling delays and missing field values SHALL NOT create entries. Apple and Android SHALL use the same semantics. Unsupported or malformed boundary references and excessive collections SHALL fail the read without modifying stored data.

#### Scenario: Resume and published snapshot
- **WHEN** a recording resumes after an interruption and the resumed sample and boundary are committed
- **THEN** metadata contains the exact last-before and first-after sample times
- **AND** an earlier published boundary excludes the later interruption

#### Scenario: Archive and pruning
- **WHEN** a supported archive with recording boundaries is imported or its recording is pruned
- **THEN** the same confirmed break endpoints remain available
- **AND** recordings without boundary events return an empty array

#### Scenario: Invalid or excessive boundaries
- **WHEN** a boundary references an absent or unpublished sample, has invalid ordering, or exceeds the collection bound
- **THEN** the read fails without truncation or data modification
