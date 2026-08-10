## Context

The six selected Rust protocol families expose separate, hand-written Android JNI bridges for framing, firmware identity, custom configuration, BMS values, controller data, and Refloat control. Their exported symbols and domain records differ, but their 3,334 source lines repeatedly perform the same low-level operations: contain panics, validate the JNI function table, reject pending exceptions and null/aliased references, obtain primitive-array lengths, copy bounded Java bytes into Rust, publish bounded bytes and integers back to Java, and check for exceptions after every VM call.

The bridges are already selected in production and package six independent libraries for four Android ABIs plus host JVM tests. This change therefore sits below locked JNI contracts. Apple uses different caller-owned pointer invariants through `apple-ffi-support` and remains an unchanged regression peer.

## Goals / Non-Goals

**Goals:**

- Define one small Rust crate for reusable Android JNI boundary mechanics using the already locked `jni-sys = 0.4.1` dependency.
- Make environment, exception, array-length, reference-alias, bounded-copy/publication, and panic behavior consistent across all six bridges.
- Preserve exact exported symbols, ABI/status numbers, argument order/types, records, Kotlin adapters, shared-library names, four-ABI packages, and host-test loading.
- Delete repeated low-level implementations and duplicate primitive-only tests while retaining every bridge's domain and JNI-contract tests.
- Make changes to shared support invalidate every Android bridge Gradle task.

**Non-Goals:**

- Sharing domain record encoders, protocol error mappings, exported entry points, libraries, Kotlin adapters, or host application behavior.
- Adopting the higher-level `jni` crate, generated JNI exports, `cbindgen`, bindgen, UniFFI, declarative/procedural macros, callbacks, opaque handles, or committed generated/native artifacts.
- Combining Apple and Android support, changing Apple bridges, or moving protocol, routing, scheduling, application, persistence, BLE, background, location, file, theme, loopback, WebView, or SQLite ownership.

## Decisions

### Add a normal support crate over the existing raw JNI dependency

`android-jni-support` will be a workspace library whose only external dependency is the already pinned `jni-sys = 0.4.1`. It will re-export only the JNI primitive/reference types needed to spell explicit bridge signatures and provide ordinary functions and small per-call value types for containment and primitive-array operations. The six bridge manifests will depend on their domain crate plus support rather than each depending directly on `jni-sys`.

The higher-level `jni` crate was rejected because it would introduce a new dependency graph and broader object/string/local-frame abstractions that these primitive-array-only contracts do not need. Export macros and code generation were rejected because the six argument/result shapes differ and explicit exported names are part of the reviewed ABI. `cbindgen` does not generate Kotlin JNI declarations or remove VM-boundary logic.

### Use a non-retaining per-call environment wrapper

An unsafe constructor will validate a raw `JNIEnv` and its function table for the duration of one exported call. The resulting borrowed wrapper will expose checked operations for pending-exception detection, primitive-array length, pairwise `IsSameObject` rejection, maximum-bounded byte reads, capacity-checked byte writes, and exact integer writes. It will not create global references, retain local references or the environment, invoke host callbacks, attach threads, or cross the exported call lifetime.

All raw JNI function-table calls and post-call exception checks will live in support. Bridge entry points remain `unsafe extern "system"` because the JVM owns reference validity and declared types, but consumers will contain only explicit contract validation, domain invocation, status mapping, and calls to checked support operations.

### Keep bridge-specific policy and status mapping explicit

Support will return a small protocol-neutral boundary error set distinguishing invalid environment/reference/shape/alias/limit from a VM exception. Consumers continue to decide required arrays, exact metadata lengths, operation-specific maxima, initial output values, oversized-input status, success-only publication, unsupported-command fields, and established internal/invalid status mapping.

A configurable mega-invocation helper was rejected because opaque flags or callbacks would hide the exact order in which each ABI validates and initializes its arrays. Generic panic containment may accept a caller-provided fallback value, but support will not know any bridge ABI version or status constant.

### Preserve independent bridge crates and build products

Each Android bridge keeps its Cargo package, one domain dependency, explicit exported functions, JNI tests, Kotlin adapter, Gradle task family, shared-library name, and disjoint host/four-ABI output directory. All six task input sets will include the support manifest and source so incremental builds cannot retain stale boundary code. The shared allowlisted builder and symbol catalog remain unchanged apart from dependency/input audits.

## Risks / Trade-offs

- **[Risk] A support defect affects all Android bridges.** → Keep the API primitive-array-only, add adversarial support tests with a deterministic fake JNI table, retain bridge-level contract tests, package all four ABIs, and run all-host physical regressions.
- **[Risk] A wrapper accidentally retains a thread-local environment or reference.** → Store only a borrow tied to one unsafe constructor call, expose no owned/global reference API, forbid callbacks/thread attachment, and audit the support source and public fields.
- **[Risk] Shared helpers change established status ordering.** → Keep required/maximum/status policy at each consumer and compare focused tests before deleting helpers.
- **[Risk] Gradle misses a shared-source change.** → Add the support manifest/source to every bridge-specific host/debug/release task input and audit exact coverage.
- **[Trade-off] Explicit exported wrappers and record encoders remain substantial.** → Retain that meaningful code for ABI clarity; higher-level record-schema sharing remains a later independent decision.

## Migration Plan

1. Add the support crate and fake-JNI-table tests without changing consumers.
2. Migrate one representative single-decode bridge and the multi-operation framing bridge, compare focused host/JNI behavior, then migrate the remaining four.
3. Remove bridge-local raw JNI and containment helpers/direct `jni-sys` dependencies; update Gradle inputs, audits, and documentation.
4. Run clean host/four-ABI Debug/Release packaging and the complete `just verify` gate.
5. Run identity, direct/CAN control, BMS, and mobile background physical scenarios as applicable on Android, iOS, and macOS; defer only bounded-time location sample absence.
6. Rollback is a normal revert restoring bridge-local helpers and direct dependencies; no ABI, application state, or persistent data migration is required.

## Open Questions

None. The six consumers, primitive-array-only boundary, existing dependency version, public ABIs, build outputs, verification gates, and physical targets are fixed by the repository.
