## ADDED Requirements

### Requirement: Coarse runtime build tooling replaces protocol matrix

The repository SHALL have one source-build definition for each thin host runtime binding and SHALL NOT retain a protocol bridge catalog, per-family host build helper, per-family Gradle/Just task, test property, source set, or generated artifact directory.

#### Scenario: Audit final build topology

- **WHEN** normal verification inspects Apple and Android runtime builds
- **THEN** Apple produces one runtime archive, Android produces one runtime library per ABI/profile, and zero protocol bridge build paths or artifacts are selected
