## Why

Rust now owns canonical per-session SQLite writes and the platform-neutral history reduction kernel, but Apple and Android still duplicate every SQLite scan that connects those components. Adding the read-only half now creates one complete per-session persistence package before catalog/runtime composition, while the candidate remains unselected and therefore cheap to reshape without production risk.

## What Changes

- **BREAKING (unselected Rust workspace only):** rename `backends/core/session-sqlite-writer` and its Rust package to durable `session-sqlite`, retaining the existing `SessionWriter` API while removing the superseded writer-only path, recipe names, and ownership wording.
- Add an independent read-only `SessionReader` that validates and snapshots one canonical session database, exposes metadata/commit-marker/numeric-field facts, and reads bounded published sample, event, and location rows without mutating the file.
- Compose ordered sample scans directly through `session-history-kernel` for deterministic bounded series reduction, including stable and extension fields, riding/idle cadence, gap generations, retained-resolution behavior, and published-sequence boundaries.
- Add literal database/query fixtures and adversarial tests for active-WAL snapshots, pagination primitives, malformed values/schema, unsupported fields, bounds, cancellation, concurrent writer progress, and payload-free diagnostics.
- Extend root verification to require one locked SQLite package and reject the removed writer-only path, reader FFI/JNI crates, host package selection, generated bindings, copied native code, and production Apple/Android/frontend/SQL changes.
- Keep native catalog lookup, opaque cursor encoding, gzip/HTTP serialization, read-lease/executor policy, pruning mutation, recovery, imports/exports, deletion, host selection, and all OS capabilities unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Rename the writer-only candidate to one Rust per-session SQLite package and add its bounded read-only query and reduction behavior.
- `platform-hosts`: Document the consolidated candidate location and preserve selected Swift/Kotlin stores until later unified-runtime cutovers.
- `backend-conformance`: Require exact reader fixtures, adversarial/concurrency evidence, production-isolation audits, the complete software gate, and unchanged all-host physical behavior.

## Impact

- Affects the unselected Rust workspace, its fixtures, root verification recipes/audits, and OpenSpec ownership documentation.
- Reuses the existing locked `rusqlite`, `serde_json`, and `session-history-kernel` graph; no new runtime, networking, code-generation, bridge, or platform dependency is introduced.
- Removes the writer-only crate/path/recipe vocabulary in favor of `session-sqlite`; no frontend contract, canonical schema, native package, durable production database, or selected cross-platform API changes in this step.
