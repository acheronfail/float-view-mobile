# Portable startup and software transport slice

Implemented on 2026-08-08.

## Ownership changes

- Added one bounded Rust runtime startup configuration containing requested port, physical/software mode, an absolute platform-selected durable root, and the packaged frontend index bytes.
- Rust now creates the launch token and starts the application actor, loopback server, stores, and runtime-ready presentation without bootstrap command round trips.
- Deleted the resource, document, storage-root, available-space, clock, random, diagnostic, theme, service-state, and export host commands after the reachability audit found no production requester or after portable startup superseded them.
- Retained exactly eleven command discriminators: five Bluetooth operations, two location operations, background logging, permission/settings presentation, and runtime readiness.
- Routed macOS development, iOS simulator, and Android emulator software selection into the Rust software host. Native software peripherals and their fixture dependency were deleted.
- Software mode forwards only the remaining non-Bluetooth OS commands to the native host; discovery, connect, notifications, VESC behavior, and disconnect stay inside the shared Rust composition.
- Connect command completion now reports immediate OS-adapter acceptance. Ordered connection events independently drive Rust readiness and terminal state.
- Removed production Swift/Kotlin command encoders, random transport aliases, pending-connect completion policy, and unreachable saved-device configuration. Apple uses the private peripheral UUID and Android uses the normalized private BLE address; Rust exposes only redacted public discovery identifiers.

## Verification

- `cargo check --manifest-path backends/core/Cargo.toml --workspace --all-targets`
- `cargo test --manifest-path backends/core/Cargo.toml -p host-runtime configured_startup_starts_the_shared_production_composition`
- `cargo test --manifest-path backends/core/Cargo.toml -p host-runtime software_transport_keeps_bluetooth_commands_inside_rust`
- `just test apple`
- `just test android`

The focused host-runtime tests cover runtime-ready ordering, lifecycle resume through the same backend, explicit stop, invalid root/resource rejection, and absence of native Bluetooth commands in software mode.
