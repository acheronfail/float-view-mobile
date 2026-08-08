## Why

The shared Rust backend has removed the former protocol, storage, application, and loopback duplication, but the remaining native hosts still mix required OS capabilities with bridge coordination and do not make their supported capability set easy to discover. The bridge minimization also left required document presentation, location-authorization propagation, background capability reporting, and appearance behavior unreachable or ambiguous. A smaller one-way bridge, stable VESC identity boundary, and explicit capability-oriented host layout will keep shared behavior in Rust while making every retained native responsibility visible and testable.

## What Changes

- Replace the Swift and Kotlin software VESC peripherals/adapters with the existing Rust software host for simulators, emulators, browser development, and software conformance.
- **BREAKING** revise the private atomically shipped C/JNI bridge so command metadata is represented once, command kinds use the outer header, and hosts implement only command decoding plus event submission required in production.
- Add focused bounded platform-event submissions so Swift and Kotlin no longer maintain mirrored general-purpose event codecs or test-only reverse codecs.
- Move portable bootstrap work—secure randomness, clocks, runtime coordination, and filesystem operations available uniformly in Rust—into the core; pass genuinely platform-selected roots and packaged resources as bounded startup inputs.
- Collapse host-side Bluetooth tokenization, connection-operation policy, public state normalization, and saved-device policy into Rust while retaining OS GATT discovery, permission handling, write backpressure, callback ordering, and raw byte transport in the native adapters.
- Use the meaningful VESC firmware device UUID itself as the stable saved-board identity for now. Remove the per-process random device-token layer; Apple peripheral UUIDs and Android BLE addresses remain only private local transport associations used to rediscover and connect before the firmware UUID is available, and none of these identifiers may be logged or exported.
- Define one explicit native capability model for Bluetooth transport, background execution, location, native import/export, system appearance, lifecycle/WebView ownership, and the minimal Rust bridge. Give iOS, macOS, and Android predictable capability-oriented entry files and per-host support inventories so each capability is easy to locate and audit.
- Require every supported host to retain an entry file for every native capability. A host that cannot support a capability provides a small documented stub with a stable unavailable result rather than omitting the capability or scattering conditional behavior through unrelated files.
- Move the macOS host into a `backends/apple/macos/` tree that is a sibling of `backends/apple/ios/`; place genuinely shared Apple implementation in an explicit common area used by both hosts.
- Restore reachable Rust-orchestrated native document flows. Rust owns staging, bounds, archive semantics, validation, cleanup, and application results; Swift and Kotlin own only picker/save/share presentation and security-scoped or content-URI copying.
- Use exactly one verified system-appearance source per host. Prefer the WebView `prefers-color-scheme` observation when it reliably reports platform changes; otherwise retain one focused native effective-appearance observation. Do not maintain parallel theme synchronization paths for the same host.
- Delete bridge commands, event families, resource namespaces, host models, compatibility paths, and tests that have no reachable production owner after required Bluetooth, background, location, document, appearance, lifecycle, WebView, and presentation capabilities are wired end to end.
- Preserve one process-owned runtime and one off-UI-thread pull worker, caller-owned bounded buffers, no Rust-to-host callbacks, exact ABI/build identity checks, loopback authentication, and physical BLE behavior.
- Treat host line-count reduction as an audit outcome rather than a functional requirement: production Swift and Kotlin are counted separately from tests, generated files, and build checkouts, and every remaining production type must map to an allowed host responsibility.

### Non-goals

- Reimplementing CoreBluetooth, Android Bluetooth/GATT, Core Location, Android location, foreground services, permissions, WebViews, lifecycle, document URI/security-scoped access, or native presentation in Rust.
- Exposing platform Bluetooth addresses, Apple peripheral UUIDs other than the selected VESC device UUID identity, launch tokens, coordinates, or telemetry through logs, diagnostics, portable archives, or frontend state.
- Introducing generated Swift/Kotlin API models, UniFFI, native-to-host callbacks, a second runtime, or long-term ABI compatibility shims.
- Changing saved-versus-guest tracking semantics, VESC wire protocols, or durable session formats. Narrow frontend contract changes required to initiate and report native document presentation remain in scope; unrelated contract expansion does not.

## Capabilities

### New Capabilities

- `native-platform-bridge`: Defines the minimal one-way C/JNI command/event bridge, startup inputs, software-host selection, bounds, identity checks, reachable native document presentation, necessary background/appearance observations, and deletion criteria for unused bridge surface.

### Modified Capabilities

- `platform-hosts`: Tightens Apple and Android ownership to OS adapters and presentation seams; defines the cross-platform capability inventory, naming and file-layout conventions, supported/unsupported stubs, and sibling iOS/macOS organization; and requires removal of duplicated software peripherals, portable bootstrap services, and intermediate coordination models.
- `vesc-device-connection`: Uses the VESC device UUID as the stable local device identifier across scans, reconnects, WebView recreation, and process recreation, without per-process randomized aliases.
- `native-development-loop`: Routes simulator, emulator, browser-host, and software development through the Rust software host rather than native software-Bluetooth implementations.
- `backend-conformance`: Adds bridge-direction, stable-UUID, process-restart, software-host parity, bounds, shutdown, capability-matrix, native document presentation, appearance-change, background-operation, and host-surface audit coverage.

## Impact

- Affects `backends/core` host ports/runtime/C ABI/JNI, the generated checked-in C header, targeted frontend document-flow contract and presentation code, Apple bridge/runtime and capability layout, Android JNI/worker and capability layout, native packaging, and canonical conformance.
- Deletes the Swift and Kotlin software VESC peripheral/adapter implementations, duplicated bridge encode/decode paths, random token maps, unused saved-device configuration APIs, unreachable command variants, and superseded tests/dependencies.
- Changes the private native ABI and JNI method set atomically across Rust, iOS, macOS, and Android; the ABI version and checked-in header provenance must be updated together.
- Changes saved-board logical identity from a process-random discovery token to the meaningful VESC firmware device UUID while retaining private platform transport associations. Test-stage storage may be recreated if existing random-token development data cannot be identified safely; no migration or heuristic merge is introduced.
- Keeps the frontend contract hand-written and keeps canonical persistence formats unchanged; any contract delta is limited to initiating and reporting required native document presentation.
