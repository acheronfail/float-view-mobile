## Why

Apple and Android each implement the same bounded historical-series reduction and irreversible prune-row selection above their SQLite readers, leaving roughly 740 lines of platform-neutral application behavior duplicated immediately before the persistence migration. Extracting that kernel now gives the future Rust SQLite/session runtime one direct safe owner while keeping this increment independent of filesystem, database, server, and host-cutover risk.

## What Changes

- Add one safe `backends/core/session-history-kernel` crate that owns bounded ordered M4 series reduction, continuity generations, and deterministic protected-row pruning over platform-neutral session observations.
- Preserve the canonical 16-field, 1,024-bucket, 65,536-observation and 1–300-second prune bounds, exact elapsed/sequence ordering, first/minimum/maximum/last role deduplication, explicit/automatic gap behavior, late and null field discontinuities, protected endpoints/semantic sources, and the eight-additional-row-per-bucket prune cap.
- Add committed shared source-derived series/prune vectors and adversarial tests for ties, gaps, phase cadence, retained resolution, invalid/non-finite input, ordering, arithmetic/counter bounds, dense extrema, and repeated pruning.
- Add audits proving the candidate owns no copied native/TypeScript runtime, SQLite, persistence port, cursor serialization, local server, host ABI, callback, thread, filesystem, network, or OS capability, and is not selected or packaged by Apple or Android.
- Keep the selected Swift and Kotlin reducers, SQLite stores, cursor encoders, loopback handlers, frontend contract/reference, and every durable format unchanged. A later Rust persistence/runtime change will call this crate directly and host cutovers will delete both native reducers without introducing `session-history-apple-ffi` or `session-history-android-jni` crates.
- Require the complete software gate and unchanged Android, iOS, and macOS ESP32 behavior; bounded location acquisition remains deferred to later manual verification.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Require one safe bounded Rust session-history kernel below future SQLite/session and loopback runtime ownership.
- `platform-hosts`: Record the unselected Rust candidate and retained selected native persistence/application ownership until later direct runtime cutovers.
- `backend-conformance`: Require shared-vector, adversarial, ownership-isolation, software, and all-host physical evidence for the extraction.

## Impact

The change affects the safe Rust workspace, shared session-history fixtures, focused tests/audits, root verification recipes, migration documentation, and OpenSpec requirements. It adds no production library input, exported symbol, dependency outside the Rust standard library, native source change, frontend API change, SQLite/schema/storage change, bridge crate, or cross-platform behavior change.
