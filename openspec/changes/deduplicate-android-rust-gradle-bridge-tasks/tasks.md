## 1. Typed Gradle bridge catalog

- [x] 1.1 Add one immutable six-entry Android Rust bridge integration catalog with exact identifiers, task suffixes, labels, crate directories, output directories, library stems, and JVM test properties.
- [x] 1.2 Add configuration invariants for the exact entry count and unique identity-bearing fields, including disjoint Android and host outputs.
- [x] 1.3 Add generic Android Debug/Release package and debug host-JNI task factories with the existing common and bridge-specific inputs, commands, outputs, groups, and descriptions.

## 2. Downstream task wiring and deletion

- [x] 2.1 Register the existing twelve package tasks and six host tasks from the catalog with byte-for-byte public task names and builder arguments.
- [x] 2.2 Derive JNI source sets, JVM-test host dependencies/library properties, Debug/Release merge dependencies, and clean ordering from the registered provider collections.
- [x] 2.3 Delete the six bridge-specific package factories, six hand-written host tasks, per-bridge output/library variables, repeated source-set entries, repeated dependency lists, and repeated clean-order declarations.

## 3. Audits and documentation

- [x] 3.1 Add a Gradle verification task and root audit assertions for the exact six entries, eighteen task names, unique outputs/properties, unchanged task graph, and absence of bridge-specific task factories/input blocks.
- [x] 3.2 Update Android and core documentation to explain the typed Gradle integration catalog, its intentional separation from the shell build catalog, and the one-entry process for later bridges.
- [x] 3.3 Confirm the Gradle file has a material line-count reduction with no generated metadata, new Gradle module/plugin, dependency, or committed native artifact.

## 4. Software verification

- [x] 4.1 Pass Gradle Kotlin DSL configuration, exact task-surface audit, and Debug/Release task dry runs.
- [x] 4.2 Pass all six linked Android host-JNI suites and structural/source-input/output audits.
- [x] 4.3 Pass clean four-ABI Android Debug/Release packaging and unchanged Apple bridge/package builds.
- [x] 4.4 Pass strict OpenSpec validation, `git diff --check`, and the complete `just verify` software gate.

## 5. Physical regression and final gate

- [x] 5.1 Run identity, direct/CAN control, BMS, and background scenarios as applicable on the connected Android device against the ESP32 fixture.
- [x] 5.2 Run the same applicable identity, direct/CAN control, BMS, and background scenarios on the connected iOS device against the ESP32 fixture.
- [x] 5.3 Run identity, direct/CAN control, and BMS scenarios on the macOS host against the ESP32 fixture.
- [x] 5.4 Confirm cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence.
- [x] 5.5 Rerun structural audits and the complete `just verify` gate after physical evidence succeeds.
