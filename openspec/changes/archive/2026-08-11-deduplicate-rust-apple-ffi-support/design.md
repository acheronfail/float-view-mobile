## Context

The six selected Apple Rust bridge crates contain 3,202 lines and deliberately expose separate, small C ABIs for VESC framing, firmware identity, custom configuration, BMS values, controller data, and Refloat control. Their domain mappings and result records differ, but each independently implements the same low-level boundary invariants: contain Rust panics, model pointer ranges without overflow, reject null/misaligned/overlapping regions, borrow validated input bytes, copy bounded output bytes, and publish scalar metadata.

The bridge ABIs are already selected in production on iOS and macOS, so this is a refactor beneath a locked interface. The support component must make unsafe behavior easier to audit without hiding exported symbols or coupling protocol crates. Android JNI has different VM/reference/array invariants and remains outside this step.

## Goals / Non-Goals

**Goals:**

- Define one small dependency-free Rust crate for reusable Apple C-boundary mechanics.
- Make invalid-region, alias, address-overflow, capacity, and panic behavior identical across all six Apple bridges.
- Preserve exact exported symbols, status numbers, records, headers, archives, Swift adapters, and production behavior.
- Delete repeated low-level implementations and duplicate primitive tests while retaining each bridge's domain/ABI tests.
- Ensure changes to shared support invalidate every Apple bridge Xcode build phase.

**Non-Goals:**

- Sharing domain record encoders, protocol error mappings, exported entry points, C headers, archives, or Swift adapters.
- Refactoring Android JNI, adopting `jni`, UniFFI, `cbindgen`, `bindgen`, a procedural macro, or generated source.
- Changing protocol behavior, host ownership, frontend contracts, BLE, background work, location, persistence, files, theme, WebViews, loopback, or SQLite.

## Decisions

### Add a normal library crate, not generated code or an export macro

`apple-ffi-support` will be a workspace library with no dependencies. It will expose ordinary, reviewable functions and small value types for panic containment, checked regions/disjointness, alignment checks, validated byte slices, and bounded copies/scalar writes. Each bridge continues to spell its exported `extern "C"` functions explicitly.

`cbindgen` was rejected because it generates headers but does not remove Rust pointer/JNI plumbing, adds tool/version/reproducibility machinery, and would make checked-in public headers derived artifacts. Export-generating declarative/procedural macros were rejected for this step because they obscure exact symbols and cannot naturally express the six different argument/result shapes. UniFFI was rejected because it changes the ABI and adds a runtime.

### Keep domain statuses and validation policy in each bridge

The support crate will return a small protocol-neutral boundary error or a contained-panic result. Consumers map those outcomes to their already published status constants and continue to decide operation-specific limits, required metadata pointers, optional regions, default output initialization, and domain error mappings.

This prevents the support crate from learning VESC/Refloat concepts or ABI version/status values. A single configurable mega-validator was rejected because arrays of opaque flags would save call-site lines at the cost of reviewability.

### Centralize raw memory operations after explicit region validation

The support crate will own the implementations of address-range construction, pairwise disjointness, `slice::from_raw_parts`, bounded byte copying, and unaligned/typed scalar publication as applicable. Its unsafe functions will have documented caller invariants and explicit unsafe blocks; workspace audits will reject these primitives in Apple bridge consumers.

Bridge entry points remain `unsafe extern "C"` because C cannot express Rust lifetime guarantees, but their unsafe work becomes a short call into the audited support component after validation. Tests will cover zero-length null inputs, required null outputs, misalignment, wraparound, aliasing, insufficient capacity, exact capacity, successful publication, and panic containment.

### Preserve independent bridge crates and build products

Each Apple bridge keeps its own Cargo package, dependency on exactly one domain crate plus the support crate, static archive, C ABI tests, header module, Xcode phase, and Swift adapter. The six Cargo manifests gain the same path dependency; no combined Apple archive is introduced.

Every Xcode Rust phase will list the support manifest and source as inputs so incremental builds cannot retain stale embedded helpers. Root/shared builders already use Cargo dependency resolution; audits will verify catalog and phase completeness for all six families and correct the canonical four/eight count to six/twelve.

## Risks / Trade-offs

- **[Risk] A support bug affects all Apple bridges.** → Keep the API small, preserve bridge-level adversarial/ABI tests, add exhaustive support-unit tests, build every macOS/iOS target, and run all-host physical regressions.
- **[Risk] Generic helpers weaken bridge-specific pointer checks.** → Keep required/optional pointer and maximum-capacity policy at each call site and compare the migrated behavior against existing tests before deleting helpers.
- **[Risk] Unsafe lifetime or alias claims become too broad.** → Construct slices only after checked non-overlapping regions, document caller invariants, expose no stored/global pointers, and forbid callbacks or retained borrows.
- **[Risk] Shared source changes do not trigger Xcode phases.** → Add both support manifest and source to every phase input list and audit exact coverage.
- **[Trade-off] Explicit exported wrappers and domain encoders still contain substantial code.** → Retain that meaningful duplication for ABI clarity; evaluate Android support and higher-level record schemas only in later changes.

## Migration Plan

1. Add the support crate and exhaustive boundary tests without changing consumers.
2. Migrate one representative decode bridge and one multi-operation bridge, compare focused ABI/adversarial tests, then migrate the remaining four.
3. Delete all superseded primitives, update manifests/Xcode inputs/audits/documentation, and run all Apple target/package plus complete repository verification.
4. Run identity, direct/CAN control, BMS, and mobile background physical scenarios as applicable on Android, iOS, and macOS. Defer only bounded-time location sample absence.
5. Rollback is a normal revert restoring bridge-local helpers; public ABI, persistent state, and runtime data require no migration.

## Open Questions

None. The six consumers, public ABIs, support boundary, non-goals, verification gates, and physical targets are fixed by the current repository.
