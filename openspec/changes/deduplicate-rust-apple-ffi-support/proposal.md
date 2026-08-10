## Why

The six Apple Rust bridge crates repeat the same panic containment, raw-pointer region validation, alias rejection, input slicing, and bounded output publication across thousands of lines of otherwise domain-specific code. Extracting those security-sensitive mechanics once reduces drift and makes each later Rust protocol cutover smaller without changing any host ABI or application behavior.

## What Changes

- Add one dependency-free `backends/core/apple-ffi-support` crate for the invariant Apple C-boundary status containment, checked memory-region model, disjointness validation, borrowed input construction, and bounded output publication used by existing bridges.
- Rewire all six `-apple-ffi` crates to use the shared support API while retaining their exported symbols, status values, record encoders, domain mappings, tests, C headers, archive names, and Swift call sites.
- Delete the superseded bridge-local copies and add audits/tests that prove the support crate remains protocol-agnostic, dependency-free, bounded, panic-containing, and the only owner of those primitives.
- Make every Apple bridge build/Xcode phase track the shared support source, and correct the build-tooling contract from its original four/eight bridge count to the six/twelve bridge families/artifacts already present after later migrations.
- Run focused ABI/build checks, the complete software gate, and ESP32 physical regressions on Android, iOS, and macOS; only bounded-time location sample absence remains deferred.
- Non-goals: changing Android JNI runtime code, generating headers or bindings, changing any C/JNI symbol or record, merging bridge crates or archives, moving protocol logic, or changing BLE, background, location, files, theme, SQLite, loopback, frontend, or application behavior.

## Capabilities

### New Capabilities

- `rust-apple-ffi-support`: Defines the shared, protocol-agnostic Apple C-boundary safety support and the unchanged ownership/ABI requirements for its six consumers.

### Modified Capabilities

- `rust-host-bridge-build-tooling`: Requires all six current bridge families and twelve artifacts to be represented and makes shared Apple FFI support source an incremental build input.
- `native-core-migration`: Defines the narrow shared-support boundary while preserving domain bridge and host ownership.
- `backend-conformance`: Adds focused software and all-host physical regression evidence for the Apple-only runtime refactor.

## Impact

- Affected code: one new Rust support crate, six Apple FFI crates, Apple build/Xcode inputs, core documentation, workspace/dependency audits, and conformance requirements.
- Removed code: repeated bridge-local pointer/region, panic-containment, slice, and output-copy helpers and their duplicate unit tests.
- Public C headers, exported symbols, static archive names, Swift adapters, Android JNI libraries, frontend contract, persistence, and runtime behavior remain unchanged.
- No external dependency, generated source, database migration, or cross-platform API change is introduced.
