## ADDED Requirements

### Requirement: Typed Android Gradle bridge registration

Android Gradle SHALL define exactly one closed typed integration entry for each of the six retained Rust bridge families and SHALL use generic factories to register each existing Debug/Release four-ABI package task and debug host-JNI task. The entries SHALL be the only Gradle source for bridge identifiers, public task suffixes, domain/JNI source inputs, generated output directories, host library stems, and JVM test properties. Configuration and repository audits SHALL reject duplicate or missing identities, reintroduced bridge-specific factories, and any task-name, input, output, profile, dependency, or property drift.

#### Scenario: Register the exact task surface

- **WHEN** Gradle configures the Android application
- **THEN** it registers exactly the existing twelve package tasks and six host-JNI tasks with unchanged public names, groups, builder arguments, profiles, inputs, outputs, and clean ordering

#### Scenario: Wire every downstream consumer

- **WHEN** Android source sets, JVM tests, or Debug/Release JNI merge tasks consume Rust bridge artifacts
- **THEN** they derive all six disjoint directories, host dependencies, absolute library properties, and profile task dependencies from the same typed entries without a bridge-specific parallel list

#### Scenario: Reject catalog drift

- **WHEN** a Gradle bridge entry is missing or duplicates an identifier, suffix, output, library stem, or test property, or a bridge-specific registration path reappears
- **THEN** configuration or structural audit fails before packaging can silently omit, overwrite, or load the wrong artifact

#### Scenario: Preserve all-host behavior

- **WHEN** linked host tests, clean Android four-ABI Debug/Release packaging, complete software verification, and applicable Android/iOS/macOS ESP32 fixture scenarios run after task deduplication
- **THEN** every bridge ABI, JNI symbol, artifact, production call path, BLE/protocol/control/BMS/background assertion, session behavior, and cleanup remains unchanged, with only bounded-time location sample absence eligible for manual deferral
