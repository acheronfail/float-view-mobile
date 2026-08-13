## Why

The bounded Rust Refloat configuration codec now has exhaustive source, adversarial, software, and all-host regression evidence, while the shared Apple backend still duplicates its complete schema and value wire algorithms in Swift. Selecting Rust for sibling iOS and macOS is the next smallest production ownership transfer and leaves Android independently testable for its later cutover.

## What Changes

- Extend `refloat-configuration-codec` with canonical bounded versioned schema and value records whose serializers and parsers remain owned by the safe crate.
- Add a temporary stateless Apple C bridge with exactly ABI-version, schema-decode, values-decode, and values-encode symbols, caller-owned buffers, fixed statuses, checked disjoint regions, deterministic outputs, and panic containment.
- Build and link the bridge from locked source for macOS, iOS device, and iOS simulator without generated bindings or committed native archives.
- Replace Apple’s Swift schema parser, decompressor, hashing/signature reconstruction, and configuration value wire codec with a strict mechanical canonical-record adapter.
- Retain the normalized schema record only in the existing configuration-service connection generation and use it for current/default decode and write encode; delete all superseded Swift schema/value wire algorithms and fallback paths after selection.
- Preserve the existing Swift configuration service, compatibility catalogue, chunk assembly, cache, revisions, conflict and flywheel rules, scheduler, readback verification, Tune persistence, application publication, frontend contract, SQLite formats, and every OS capability.
- Keep Android’s selected Kotlin configuration codec, Gradle bridge set, packages, and artifacts unchanged.
- Prove safe-record, C-ABI, Swift-adapter, clean packaging, source-ownership, complete software, and Android/iOS/macOS ESP32 regression evidence. Only bounded location-sample absence may be deferred.
- Treat the Apple bridge and record adapter as migration scaffolding to be deleted when protocol and application ownership move behind the unified Rust runtime.
- Non-goals: cutting Android configuration over; moving the stateful configuration service, loopback server, application service, SQLite, BLE, location, background execution, native files, theme, lifecycle, permissions, WebViews, routing, scheduling, persistence formats, or the frontend contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Refloat configuration codec as Apple’s sole schema/value wire implementation while retaining native orchestration and an independent Android cutover.
- `platform-hosts`: Define the temporary four-symbol Apple bridge, canonical record ownership, sibling-host packaging, Android isolation, and deletion of superseded Swift codec algorithms.
- `backend-conformance`: Require focused record/ABI/adapter, clean packaging, no-fallback, complete software, and all-host ESP32 evidence for the ownership transfer.

## Impact

This affects the locked `backends/core` workspace, the safe configuration-codec crate, one new Apple bridge crate and hand-written C module, shared bridge build metadata, SwiftPM/Xcode linkage, Apple’s shared Refloat configuration model/adapter and focused tests, audits, and OpenSpec requirements. Android production sources and artifacts, frontend APIs, SQLite schemas/data, Tune storage formats, and user-visible behavior remain unchanged.
