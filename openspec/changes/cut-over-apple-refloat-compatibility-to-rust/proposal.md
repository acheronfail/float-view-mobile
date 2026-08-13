## Why

Apple still duplicates Refloat INFO identity decoding, the exact supported-release catalogue, telemetry/control profile selection, and dynamic-layout admission in Swift even though the bounded Rust candidate now has source-derived parity and all-host regression evidence. Selecting that candidate on the shared Apple backend is the smallest next production ownership transfer and removes one complete native protocol-policy slice from both iOS and macOS.

## What Changes

- Extend the safe Rust component with one canonical bounded compatibility record so the temporary host boundary transports a complete INFO/profile decision without duplicating catalogue or serialization policy.
- Add one stateless, caller-owned-buffer Apple FFI crate with exact ABI/version and compatibility-decode operations, fixed statuses, bounded disjoint regions, deterministic output initialization, and panic containment.
- Add the hand-written Apple C module, locked source build/catalogue entry, SwiftPM/Xcode linkage, and a strict Swift record adapter shared by iOS and macOS.
- Select Rust as Apple’s sole production owner of Refloat INFO decoding, exact compatibility/profile decisions, and dynamic-layout admission; delete superseded Swift INFO, catalogue, profile, and required-layout algorithms with no fallback.
- Preserve Apple controller-discovery effects, direct/CAN routing, request scheduling, connection generations, telemetry decoding, controls, application behavior, persistence, SQLite, loopback server, BLE, background/location, lifecycle, files, theme, permissions, WebViews, and frontend contract.
- Keep Android on its unchanged Kotlin compatibility path; its JNI catalogue, Gradle task surface, packaged artifacts, and production sources remain unchanged until a separate Android cutover.
- Run focused Rust/C/Swift checks, clean Apple and Android packaging, strict OpenSpec validation, `just verify`, and physical ESP32 regression on connected Android and iOS devices plus macOS. Only bounded absence of a location sample may be deferred to later manual verification.
- Non-goals: Android compatibility selection, controller-discovery migration, bridge consolidation/code generation, application/runtime unification, SQLite or local-webserver migration, or removal of unrelated temporary protocol bridges.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Require Apple to select the Rust Refloat compatibility decision through a narrow temporary bridge and delete the superseded Swift algorithms.
- `platform-hosts`: Transfer only Apple INFO/catalogue/layout admission ownership while retaining application and OS-capability ownership and Android isolation.
- `backend-conformance`: Require focused boundary, packaging, source-deletion, complete software, and all-host physical evidence for the Apple cutover.

## Impact

This affects `backends/core/refloat-compatibility`, one new `refloat-compatibility-apple-ffi` crate, the shared Rust bridge catalogue/build recipes and audits, Apple’s hand-written C module and package/Xcode linkage, the shared Swift Refloat adapter/service/tests, ownership documentation, and OpenSpec requirements. It adds one temporary migration artifact but no new third-party dependency, generated binding, persisted-format change, frontend-contract change, Android artifact, or user-visible behavior. The temporary bridge is explicitly removed when unified Rust application behavior replaces per-protocol host calls.
