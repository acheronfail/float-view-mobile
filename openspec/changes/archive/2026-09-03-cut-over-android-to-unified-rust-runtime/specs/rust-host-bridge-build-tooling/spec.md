## ADDED Requirements

### Requirement: Coarse runtime build tooling replaces protocol matrix

The repository SHALL have one source-build definition for each thin host runtime binding and SHALL NOT retain a protocol bridge catalog, per-family host build helper, per-family Gradle/Just task, test property, source set, or generated artifact directory.

#### Scenario: Audit final build topology

- **WHEN** normal verification inspects Apple and Android runtime builds
- **THEN** Apple produces one runtime archive, Android produces one runtime library per ABI/profile, and zero protocol bridge build paths or artifacts are selected

## REMOVED Requirements

### Requirement: Allowlisted bridge build catalog

**Reason:** The protocol-family catalog is temporary migration scaffolding and no protocol-family host bridges remain.

**Migration:** Each host has one closed runtime build definition with fixed package, artifact, architecture, and symbol inputs.

### Requirement: Shared Apple bridge source builder

**Reason:** Apple now builds one coarse runtime archive rather than six catalogued protocol archives.

**Migration:** The retained Apple runtime builder preserves locked/offline target and architecture validation for the single application-runtime archive.

### Requirement: Shared Android bridge source builder

**Reason:** Android now builds one coarse runtime JNI library rather than the protocol bridge matrix.

**Migration:** The retained Android runtime builder preserves locked/offline host and four-ABI architecture/symbol validation for the single runtime library.

### Requirement: Build integration and regression equivalence

**Reason:** Preserving twelve temporary bridge artifacts contradicts final convergence.

**Migration:** Root recipes, Gradle, and Xcode select the two coarse host runtime bindings and audits reject every retired protocol artifact and task.

### Requirement: Typed Android Gradle bridge registration

**Reason:** The six-family Gradle registration existed only to manage temporary JNI bridges.

**Migration:** One typed runtime definition owns host-test and Debug/Release four-ABI Android tasks and inputs.
