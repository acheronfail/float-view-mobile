## Why

Apple and Android still duplicate the largest remaining pure Refloat protocol algorithm: bounded Qt-compressed schema decoding, normalized metadata and fingerprint construction, wire-signature calculation, and exact typed value serialization. Extracting that cohesive deterministic boundary into an unselected safe Rust component is the smallest next step that materially advances the shared runtime without changing production behavior or adding another temporary host bridge.

## What Changes

- Add one safe, host-independent `float-view-refloat-configuration-codec` candidate under `backends/core` for bounded Qt/zlib decompression, hardened `ConfigParams` parsing, normalized schema construction, SHA-256 fingerprinting, CRC32C wire-signature calculation, and exact schema-driven value decode/encode.
- Preserve the committed limits, closed field and wire types, sanitization, finite metadata rules, ordering/group validation, signed/numeric/string semantics, signature checks, complete-value requirements, and stable payload-free failures already mirrored by Swift and Kotlin.
- Prove exact parity across all nine immutable Refloat 1.0.0–1.2.3 schema/value fixture families plus adversarial compressed, XML, metadata, signature, truncation, remainder, missing/extra value, and encode-boundary cases.
- Add only the small maintained Rust dependencies needed for safe zlib, streaming XML, and SHA-256 processing, locked for offline builds and covered by dependency and unsafe-code audits.
- Extend workspace, documentation, ownership, production-isolation, bridge-symbol, build-catalogue, packaging, and fixture-parity checks without adding Apple FFI, Android JNI, generated bindings, packaged native libraries, or a selected host call path.
- Keep Swift and Kotlin as the production codec owners. Later independent Apple and Android cutover changes will add temporary bridges and delete each superseded native algorithm; later changes will move the stateful configuration service and ultimately remove all migration bridge crates with the unified Rust runtime.
- Run `just verify` and production-path physical identity, control, BMS, configuration, telemetry, ride, disconnect, and background/session regression on connected Android, iOS, and macOS hosts against the ESP32 fixture. Only bounded location-sample acquisition may remain deferred manual coverage.
- Non-goals: host cutover or native-code deletion; schema caching/catalogue admission; request scheduling/chunk assembly; configuration lifecycle, drafts, writes/readback coordination, Tune persistence, application behavior, SQLite, loopback server, BLE, location, native files, theme, permissions, UI, or frontend-contract changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the bounded model, dependency constraints, source/fixture parity, isolation, and retained native ownership of the unselected Rust Refloat configuration codec candidate.
- `platform-hosts`: Locate the candidate below both native hosts while preserving production configuration coordination, persistence, and OS capability ownership.
- `backend-conformance`: Require exhaustive source-derived and adversarial candidate evidence, the complete software gate, and all-host physical regression without claiming candidate execution.

## Impact

This affects the locked `backends/core` workspace and lockfile, one new safe Rust crate, focused fixture tests and root audits, core ownership documentation, `just verify`, and OpenSpec requirements. It adds locked safe parser/decompression/hash dependencies but no runtime-selected artifact, cross-platform API, database or persistence change, native package dependency, frontend-contract change, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
