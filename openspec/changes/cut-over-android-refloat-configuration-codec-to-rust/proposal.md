## Why

Apple now selects the bounded Rust Refloat configuration codec, but Android still duplicates its Qt decompression, XML normalization, schema fingerprint/signature, typed value decoding, and write encoding in Kotlin. Cutting Android over to the same safe crate removes the remaining native protocol duplicate and gives both production backends one source of truth before stateful configuration behavior later moves behind the unified Rust runtime.

## What Changes

- Add one temporary, stateless Android JNI bridge over the existing canonical schema/value record APIs, built from locked source for the host JVM and all supported Android ABIs.
- Replace Android's Kotlin protocol implementation with a strict canonical-record adapter while preserving its existing models, service API, errors, and frontend behavior.
- Retain the exact canonical schema record with Android's generation-scoped schema so current/default decode and write encode use the same Rust-validated layout.
- Delete the superseded Kotlin Qt decompressor, SAX/XML normalization, fingerprint/signature, value reader/writer, scaling, string framing, and CRC algorithms with no runtime fallback.
- Extend focused tests, build/source/symbol audits, the complete software gate, and Android/iOS/macOS ESP32 checkpoints to prove selection and regression safety.
- Preserve the selected Apple codec and bridge unchanged; retain Android's stateful configuration service, compatibility catalogue, scheduler, persistence, loopback transport, and every OS capability.
- Treat both platform codec bridges as migration scaffolding to delete when configuration/application behavior moves into the unified Rust runtime.

Non-goals are moving stateful configuration or general application behavior, SQLite, the loopback web server, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract; changing stored Tune/configuration formats; generating bindings; or consolidating temporary bridge crates.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the existing safe Refloat configuration codec as Android's sole production owner of schema and value wire semantics while retaining stateful Kotlin ownership.
- `platform-hosts`: Define the temporary bounded Android JNI ownership, locked-source packaging, Kotlin adapter deletion, Apple isolation, and eventual bridge removal boundary.
- `backend-conformance`: Require focused ABI/adapter evidence, clean packaging, complete software regression, and all-host physical selection evidence.

## Impact

The change affects `backends/core/refloat-configuration-codec`, a new sibling `-android-jni` crate, shared bridge-build metadata, Android Gradle packaging, Android's Refloat configuration codec/service and tests, ownership documentation, root audits, and conformance evidence. It adds no frontend or persistence contract change, no runtime fallback, no generated source, and no committed native artifact.
