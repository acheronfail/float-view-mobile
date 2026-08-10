## Why

Eight bridge-local Apple and Android build scripts repeat the same pinned-toolchain, target, packaging, architecture, and symbol-audit logic across roughly one thousand lines, so adding or updating a bridge requires copying security-sensitive tooling and invites silent drift. Centralizing only this mechanical build layer is a low-risk way to reduce maintenance cost before moving further native behavior into Rust.

## What Changes

- Add one source-controlled allowlisted catalog for the four existing Rust bridge families and their exact Cargo packages, library names, diagnostics, and Android JNI symbols.
- Add one shared Apple static-library builder and one shared Android host/four-ABI JNI builder that consume only catalogued bridge identifiers and preserve the pinned offline toolchains, target mappings, output names, architecture checks, and failure behavior.
- Rewire root `just` recipes, Android Gradle inputs/commands, and iOS Xcode build-phase inputs/commands to the shared tooling while preserving task names, output directories, link names, and incremental rebuild dependencies.
- Delete all eight superseded bridge-local build scripts and add repository audits that reject reintroduced per-bridge helpers, unknown catalog identifiers, missing pinned toolchains/targets/NDKs, symbol drift, wrong architectures, generated bindings, or committed native artifacts.
- Verify clean Debug/Release packaging, the complete software gate, and focused ESP32 physical regressions on Android, iOS, and macOS. Only bounded-time location sample absence remains deferred.
- Non-goals: changing any Rust bridge implementation, ABI version, exported symbol, Cargo dependency, Swift/Kotlin adapter, protocol behavior, runtime ownership, frontend contract, generated binding policy, or introducing `cbindgen`, macros, an `xtask`, a general native runtime, callbacks, or opaque handles.

## Capabilities

### New Capabilities

- `rust-host-bridge-build-tooling`: Defines the shared allowlisted Apple and Android source-build tooling, retained bridge-specific identities, exact outputs/audits, deletion of copied helpers, and regression evidence.

### Modified Capabilities

None.

## Impact

- Affected paths: `backends/core` shared tooling and deleted bridge-local scripts, Android Gradle Rust build tasks, iOS Xcode Rust build phases, root `just` recipes/audits, and backend/core documentation.
- Runtime and cross-platform APIs are unchanged: all eight bridge crates, four Apple C modules, four Android JNI loaders, symbols, records, statuses, and production call paths remain byte-for-byte compatible.
- No dependency, persistence, Bluetooth, backgrounding, location, file, theme, WebView, application-service, SQLite, loopback, or frontend behavior changes.
