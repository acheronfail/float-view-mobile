# Android Shared Runtime Integration

## Purpose

Define Android’s current shared-service selection, native OS boundary and runtime packaging.

Original cutover requirements and evidence remain in [archived changes](../../changes/archive/).
Ongoing checks follow the [verification policy](../../../AGENTS.md#verification).
The capability identifier remains stable for historical references.

## Requirements

### Requirement: Android selects one unified Rust service

Android SHALL use one process-owned shared Rust service through the bounded callback-free JNI boundary specified by [rust-android-jni-support](../rust-android-jni-support/spec.md). Kotlin SHALL execute OS capabilities without protocol, product, persistence, dispatch or snapshot fallback implementations.

#### Scenario: Execute an ordered result

- **WHEN** Android supplies a platform fact or frontend operation
- **THEN** shared code owns loopback authentication, dispatch, serialization, product/catalog storage, archives and portable policy
- **AND** Kotlin validates complete results and executes ordered Bluetooth, clock/timer, lifecycle/background, permission, location, protected-root, external-file and theme/UI effects without duplicating domain records

#### Scenario: Inspect production and build inputs

- **WHEN** Kotlin/Rust, Gradle, root recipes, symbols and packaged libraries are audited
- **THEN** no retired native decision owner, protocol JNI adapter/crate, support crate, fallback import, per-protocol task/property/source set, bridge catalog/helper or generated protocol artifact remains

### Requirement: Android packages one coarse runtime library

Android Debug and Release builds SHALL source-build and package exactly one unified runtime JNI library for each supported ABI, and JVM tests SHALL load exactly one host runtime library through the same closed build definition.

#### Scenario: Build clean Android packages

- **WHEN** clean host-test and four-ABI Debug/Release packaging runs
- **THEN** each expected output contains the runtime library with the required architecture/symbols and contains no protocol-specific Rust library
