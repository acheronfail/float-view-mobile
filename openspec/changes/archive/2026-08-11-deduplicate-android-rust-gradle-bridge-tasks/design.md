## Context

The shared shell catalog and builders already reduce twelve Rust host bridges to six closed bridge identifiers and two source builders. Android Gradle nevertheless encodes each bridge again through separate output variables, host-library variables, package-task factories, host tasks, source-set entries, test dependencies/properties, merge dependencies, and clean ordering. The repeated blocks differ only in stable metadata such as task suffix, crate directory, output directory, library stem, and JVM property.

This change is deliberately limited to Gradle Kotlin DSL registration. It must preserve every externally used task and file identity because root recipes, Android packaging, JVM tests, conformance, and developer workflows already depend on them.

## Goals / Non-Goals

**Goals:**

- Describe each of the six retained Android Rust bridges once in a typed Gradle value.
- Register two profile package tasks and one debug host task per entry through generic factories.
- Derive source-set directories, JVM-test dependencies/properties, merge dependencies, and clean ordering from those same entries.
- Fail configuration/audit on duplicate identifiers, task suffixes, output directories, library stems, or test properties.
- Preserve exact task names, builder arguments, source inputs, outputs, packages, ABIs, JNI symbols, runtime loading, and incremental behavior.

**Non-Goals:**

- Replacing or generating the existing shell build catalog.
- Parsing or sourcing shell from Gradle, or creating a cross-language metadata generator.
- Changing Xcode phases, root public recipes, Rust crate structure, JNI ABIs, or native libraries.
- Combining package tasks, host libraries, or bridge crates.
- Moving protocol, application, BLE, background, location, storage, file, theme, lifecycle, or WebView behavior.

## Decisions

### Keep a small typed Gradle integration catalog

Define a private immutable Gradle data class and six literal entries containing only Gradle-specific integration metadata: shell bridge identifier, public task suffix, descriptive label, domain crate directory, Android output directory, host output directory, JNI library stem, and JVM test property. Derive the Android JNI crate directory from the domain directory.

This is preferred over code generation because the shell builder metadata and Gradle integration metadata have different consumers and lifecycles. A generator would add an executable build step, generated-file policy, and another failure mode to remove a few literals. Parsing the shell catalog from Kotlin would couple Gradle configuration to shell syntax and weaken type checking. A convention plugin or `buildSrc` module would add more files and Gradle machinery than the local catalog replaces.

The small intentional duplication of the six stable identifiers between shell and Gradle remains protected by audits. No package/library/symbol metadata used by the shell builder is copied into Gradle beyond the library stem Gradle needs to locate host test artifacts.

### Use two generic factories and collections of task providers

One factory registers `android-all` Debug/Release tasks, and one registers debug host tasks. Each factory receives a typed bridge entry, declares the common workspace/support/tooling inputs plus the entry's domain/JNI manifests and source trees, and declares the existing entry-specific output.

Task names are computed to exactly reproduce the current API: the frame entry has an empty task suffix (`buildAndroidRustDebug`, `buildAndroidRustHost`), while the other entries retain `Identity`, `CustomConfiguration`, `BmsValues`, `ControllerData`, and `RefloatControl`. Registration results are stored by profile and bridge identifier so later configuration uses collections rather than eighteen variables.

### Drive all downstream Gradle wiring from the same entries

For each bridge entry, register its Debug/Release JNI source directories, debug host task dependency, and host-library system property. Debug and Release JNI merge tasks depend on the corresponding six package task providers. Every generated package/host provider retains `mustRunAfter(clean)`.

This removes synchronized lists while keeping the same task graph: it does not collapse tasks or broaden one task's outputs, so Gradle's incremental invalidation and parallelism remain unchanged.

### Add configuration-time invariants and repository audit evidence

Gradle will require exactly six entries and uniqueness of every identity-bearing field. A lightweight verification task will compare the exact expected eighteen public Rust build task names with the registered task set and validate disjoint package/host outputs. Root audits will invoke that task and reject the old bridge-specific factory names and repeated input registration patterns. Existing linked JNI tests, clean four-ABI packaging, `just verify`, and physical fixture matrices remain the behavioral evidence.

## Risks / Trade-offs

- **[Risk] A computed task name or output differs by one character.** → Encode the existing suffixes and paths literally in the six entries, assert the exact eighteen-name set, and run all current Gradle consumers and packaging checks.
- **[Risk] Collection iteration changes task ordering.** → Use a fixed immutable list in the established bridge order; task dependencies remain declarative and behavior does not rely on execution order between independent bridges.
- **[Risk] File-tree inputs become too broad or omit a bridge.** → The generic factory derives exactly the same two manifests and two source trees from each entry and retains common support/workspace/tooling inputs.
- **[Trade-off] Six identifiers remain in both shell and Gradle.** → This is smaller and safer than cross-language generation; audits require the exact same closed identifier set.
- **[Risk] Refactoring build configuration masks runtime regression.** → Preserve the shell builders and artifacts unchanged, then run focused Gradle/JNI gates, complete verification, and Android/iOS/macOS ESP32 physical scenarios.

## Migration Plan

1. Introduce the typed entries, invariants, and generic task factories while preserving all current task names.
2. Switch source sets, JVM tests, merge dependencies, and clean ordering to the provider collections; delete the repeated blocks.
3. Add structural/audit documentation and run focused Gradle task, host-JNI, and clean packaging checks.
4. Run `just verify` and the three-host physical fixture matrix. Rollback is a single implementation commit because no persisted data, public contract, or artifact format changes.

## Open Questions

None. Apple/Xcode phase deduplication and further root-`just` consolidation remain separate incremental changes.
