## Context

The four selected Rust protocol components each have an Apple C bridge and Android JNI bridge. Their eight bridge-local build scripts total 1,037 lines even though the Apple scripts differ only by Cargo package/archive name and the Android scripts differ only by Cargo package/library, diagnostic text, and exact JNI symbols. The duplicated logic owns pinned toolchains, target mapping, NDK discovery, linking, architecture validation, copying, `lipo`, and Android symbol auditing, so drift is both maintenance cost and a packaging risk.

This change is deliberately below the runtime boundary. It must preserve the independent bridge crates and their exact symbols while making build identity declarative and build mechanics shared. The root `justfile` remains the public development and verification interface; Gradle and Xcode must retain their existing named tasks/phases and outputs.

## Goals / Non-Goals

**Goals:**

- Replace eight copied helpers with one allowlisted bridge catalog, one shared Apple builder, and one shared Android builder.
- Keep bridge selection explicit, bounded, source-owned, offline, and independently symbol-audited.
- Preserve every Cargo package, ABI, library name, output location, target architecture, Gradle task, Xcode phase, and host runtime path.
- Make adding a later bridge require one catalog entry plus explicit host integration rather than copying hundreds of shell lines.
- Delete superseded scripts and add exact structural/audit checks that prevent duplication from returning.

**Non-Goals:**

- Changing bridge Rust, Swift, Kotlin, C headers, record layouts, status mappings, symbols, dependencies, or production selection.
- Sharing Apple and Android artifacts, combining bridge crates, moving protocol/application behavior, or changing BLE, background, location, files, theme, SQLite, loopback, or frontend behavior.
- Introducing code generation, `cbindgen`, `bindgen`, UniFFI, macros, a Rust `xtask`, callbacks, opaque handles, generated source, or committed native binaries.
- Extracting common FFI/JNI runtime plumbing or record encoders; those remain possible later incremental changes.

## Decisions

### Use a closed shell catalog rather than free-form arguments

Add a non-executable shared catalog sourced by both builders. A selection function accepts only `frame`, `firmware-identity`, `custom-configuration`, or `bms-values` and assigns fixed package names, artifact stems, diagnostic labels, JNI prefix, and sorted expected JNI symbols. Unknown identifiers fail before Cargo or filesystem output.

Free-form package/library/symbol arguments were rejected because they would make build invocations part of the trust boundary and let call sites weaken symbol checks. Per-bridge descriptor files were rejected because eight files would preserve unnecessary distribution and make global completeness harder to audit. JSON/TOML plus a parser and a Rust `xtask` were rejected as disproportionate new machinery for four static entries.

### Keep one platform-specific builder per toolchain

The Apple builder owns SDK/architecture-to-Rust-target mapping, configuration mapping, pinned target availability, locked offline Cargo invocation, and copy-or-`lipo` publication. The Android builder owns host builds, ABI/compiler/architecture mapping, pinned NDK resolution, locked offline Cargo invocation, installation, file architecture checks, and exact catalog-provided JNI symbol comparison.

A single cross-platform mega-script was rejected because Apple and Android have different artifact forms, inputs, validation, and callers. Keeping two scripts preserves platform-local diagnostics without repeating them per protocol.

### Rewire existing callers without renaming their public surfaces

Root recipes pass a bridge identifier before the existing Apple/Android build arguments. Gradle uses one helper/catalog pair while retaining its current bridge-specific tasks, source inputs, outputs, and library properties. Each Xcode phase invokes the shared Apple helper with its bridge identifier and lists both shared files plus its bridge-specific Cargo/source files as inputs.

Merging Gradle tasks or Xcode phases was rejected because that would broaden incremental rebuild scope and make one bridge failure obscure which artifact failed. Output directories remain disjoint so packaging and host-test loaders do not change.

### Audit catalog completeness and deletion

Repository audits require the catalog to contain exactly the four retained identifiers and exact package/artifact/symbol identities; require only the two executable shared builders; reject every bridge-local `build-apple.sh`/`build-android.sh`; exercise unknown identifiers and missing pinned NDK failure; and retain existing ABI, dependency, architecture, package-content, and symbol audits. Documentation reports that bridges remain separate while build mechanics are shared.

Code generation was rejected because this change targets repeated build mechanics, not C/JNI interface declarations. The explicit symbol catalog remains small and reviewable.

## Risks / Trade-offs

- **[Risk] A shared helper regression affects all bridges.** → Preserve bridge-specific tasks and outputs, add catalog/unit-style failure checks, and run clean Debug/Release packaging plus the full software gate.
- **[Risk] Incremental builds miss a shared-tool change.** → Declare both the helper and catalog as inputs to every Gradle task and Xcode phase.
- **[Risk] Catalog drift weakens JNI symbol auditing.** → Keep exact sorted symbols in the closed catalog and audit catalog completeness against retained bridge source/package identities.
- **[Risk] Shell sourcing makes variable leakage possible.** → Use readonly global configuration, one selection function with explicit assignments, strict mode in callers, and reset/validate all selected fields before use.
- **[Trade-off] Bridge-specific call sites still repeat identifiers and outputs.** → Retain that small explicit duplication because it documents packaging ownership and preserves incremental isolation.

## Migration Plan

1. Add and audit the closed catalog plus shared Apple and Android helpers while existing scripts still provide comparison evidence.
2. Rewire root recipes, Gradle tasks, and Xcode phases to shared tooling and verify focused host builds, exact symbols, architectures, and invalid-input failures.
3. Delete the eight bridge-local helpers, update structural audits/documentation, and run clean packaging and `just verify`.
4. Run `control-direct`, `bms-express`, and mobile `background` physical scenarios as applicable on Android, iOS, and macOS; defer only bounded-time location sample absence.
5. Rollback is a normal revert restoring the bridge-local helpers and old call sites; no runtime data or ABI migration is involved.

## Open Questions

None. The retained bridge set, toolchain versions, identifiers, artifacts, symbols, call sites, and physical targets are already fixed by the repository.
