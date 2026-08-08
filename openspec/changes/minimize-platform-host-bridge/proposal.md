## Why

The shared Rust backend has removed the former protocol, storage, application, and loopback duplication, but the Apple and Android hosts still contain roughly 7,100 lines of production Swift and Kotlin, including duplicated software-VESC implementations, nearly 1,000 lines of bridge codecs, and host-side coordination that is not inherently platform-specific. A smaller one-way bridge and stable VESC identity boundary will reduce maintenance and fix saved-board identity behavior without moving CoreBluetooth, Android Bluetooth, location, lifecycle, or presentation APIs out of their native owners.

## What Changes

- Replace the Swift and Kotlin software VESC peripherals/adapters with the existing Rust software host for simulators, emulators, browser development, and software conformance.
- **BREAKING** revise the private atomically shipped C/JNI bridge so command metadata is represented once, command kinds use the outer header, and hosts implement only command decoding plus event submission required in production.
- Add focused bounded platform-event submissions so Swift and Kotlin no longer maintain mirrored general-purpose event codecs or test-only reverse codecs.
- Move portable bootstrap work—secure randomness, clocks, runtime coordination, and filesystem operations available uniformly in Rust—into the core; pass genuinely platform-selected roots and packaged resources as bounded startup inputs.
- Collapse host-side Bluetooth tokenization, connection-operation policy, public state normalization, and saved-device policy into Rust while retaining OS GATT discovery, permission handling, write backpressure, callback ordering, and raw byte transport in the native adapters.
- Use the meaningful VESC firmware device UUID itself as the stable saved-board identity for now. Remove the per-process random device-token layer; Apple peripheral UUIDs and Android BLE addresses remain only private local transport associations used to rediscover and connect before the firmware UUID is available, and none of these identifiers may be logged or exported.
- Delete bridge commands, resource namespaces, host models, compatibility paths, and tests that have no reachable production owner; required document/import/export presentation commands remain only when exercised from the contract path.
- Preserve one process-owned runtime and one off-UI-thread pull worker, caller-owned bounded buffers, no Rust-to-host callbacks, exact ABI/build identity checks, loopback authentication, and physical BLE behavior.
- Treat host line-count reduction as an audit outcome rather than a functional requirement: production Swift and Kotlin are counted separately from tests, generated files, and build checkouts, and every remaining production type must map to an allowed host responsibility.

### Non-goals

- Reimplementing CoreBluetooth, Android Bluetooth/GATT, Core Location, Android location, foreground services, permissions, WebViews, lifecycle, document URI/security-scoped access, or native presentation in Rust.
- Exposing platform Bluetooth addresses, Apple peripheral UUIDs other than the selected VESC device UUID identity, launch tokens, coordinates, or telemetry through logs, diagnostics, portable archives, or frontend state.
- Introducing generated Swift/Kotlin API models, UniFFI, native-to-host callbacks, a second runtime, or long-term ABI compatibility shims.
- Changing the frontend request/result contract, saved-versus-guest tracking semantics, VESC wire protocols, or durable session formats.

## Capabilities

### New Capabilities

- `native-platform-bridge`: Defines the minimal one-way C/JNI command/event bridge, startup inputs, software-host selection, bounds, identity checks, and deletion criteria for unused bridge surface.

### Modified Capabilities

- `platform-hosts`: Tightens Apple and Android ownership to OS adapters and presentation seams and requires removal of duplicated software peripherals, portable bootstrap services, and intermediate coordination models.
- `vesc-device-connection`: Uses the VESC device UUID as the stable local device identifier across scans, reconnects, WebView recreation, and process recreation, without per-process randomized aliases.
- `native-development-loop`: Routes simulator, emulator, browser-host, and software development through the Rust software host rather than native software-Bluetooth implementations.
- `backend-conformance`: Adds bridge-direction, stable-UUID, process-restart, software-host parity, bounds, shutdown, and host-surface audit coverage.

## Impact

- Affects `backends/core` host ports/runtime/C ABI/JNI, the generated checked-in C header, Apple bridge/runtime and Bluetooth composition, Android JNI/worker and Bluetooth composition, native packaging, and canonical conformance.
- Deletes the Swift and Kotlin software VESC peripheral/adapter implementations, duplicated bridge encode/decode paths, random token maps, unused saved-device configuration APIs, unreachable command variants, and superseded tests/dependencies.
- Changes the private native ABI and JNI method set atomically across Rust, iOS, macOS, and Android; the ABI version and checked-in header provenance must be updated together.
- Changes saved-board logical identity from a process-random discovery token to the meaningful VESC firmware device UUID while retaining private platform transport associations. Test-stage storage may be recreated if existing random-token development data cannot be identified safely; no migration or heuristic merge is introduced.
- Keeps the hand-written frontend contract and canonical persistence formats unchanged.
