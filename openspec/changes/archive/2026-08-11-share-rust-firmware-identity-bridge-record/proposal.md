## Why

The selected Apple and Android firmware-identity bridge crates independently encode and test the same versioned identity record even though both depend on the same safe Rust decoder. Now that both host cutovers are complete, retaining two protocol-neutral encoders makes record changes harder, permits cross-host drift, and leaves the boundary crates nearly as large as the component they expose.

## What Changes

- Make the safe `vesc-firmware-identity` crate the single owner of the bounded, versioned, platform-neutral identity record layout, constants, serialization, and domain-error outcome used by both host bridges.
- Reduce the Apple FFI and Android JNI crates to their independent exported symbols, caller validation, status mapping, bounded publication, and platform-boundary tests while deleting both bridge-local record encoders and duplicate record-vector helpers/tests.
- Add exact cross-host regression evidence proving both bridges publish the same existing bytes, statuses, limits, symbols, libraries, headers, adapters, and production behavior.
- Extend audits and documentation so a future identity record change is made in the safe dependency once and bridge-local record construction cannot reappear.
- Keep this as a hand-written, dependency-free refactor: no generated bindings, `cbindgen`, procedural/declarative export macros, new runtime, committed native artifact, or ABI change.
- Non-goals: changing firmware payload decoding, Swift/Kotlin record validation or models, compatibility/routing/application behavior, any other protocol family, and OS-owned Bluetooth, background, location, lifecycle, permission, file, theme, WebView, loopback, or persistence behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Assign the portable firmware-identity bridge record and outcome to the safe component while retaining separate platform boundaries and the existing selected behavior.
- `platform-hosts`: Define one canonical identity record owner beneath the independent Apple FFI and Android JNI bridge crates without combining their artifacts or host responsibilities.
- `backend-conformance`: Require exact shared-record vectors, bridge-local encoder deletion, unchanged ABI/adapters, complete software verification, and all-host physical identity regression.

## Impact

Affected code is limited to `backends/core/vesc-firmware-identity`, its two host bridge crates, their focused Rust/C/JVM tests, relevant build/ownership audits, and core documentation. Public Apple C and Android JNI symbols, status numbers, record bytes, headers, Kotlin/Swift adapters, archive/shared-library names, dependency direction, frontend contract, SQLite data, and platform behavior remain unchanged; no dependency or compatibility migration is introduced.
