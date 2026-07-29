## Why

Frontend iteration against native Bluetooth currently requires either a fixed physical conformance scenario or a non-Bluetooth mock backend, so developers cannot manually move a real native backend through useful Refloat states while Vite hot reload is running. A development-only fixture controller will make the existing ESP32 byte bridge interactive without moving protocol behavior into firmware or weakening native ownership.

## What Changes

- Add `just fixture dev [port]` to run a SvelteKit development application on fixed loopback address `127.0.0.1:11004` that controls the ESP32 fixture over serial.
- Add a server-side fixture coordinator that exclusively owns the selected serial port, maintains authoritative fixture state, and continuously answers allowlisted VESC and Refloat requests.
- Add high-level controls for serial attachment, power on/off, and start/stop riding, with complete revisioned state, bounded diagnostics, and automatic re-advertising while powered on.
- Add an atomic configuration form whose independent controller-firmware and Refloat-package selectors compose VESC 6.02, 6.05, and 6.06 variants with exact Refloat 1.0.0, 1.1.0, and 1.2.3 codecs at runtime rather than declaring nine board presets.
- Add direct-controller and VESC Express transport topologies; Express mode exposes one simulated motor controller through CAN with a developer-configurable ID.
- Present THOR300 as the single simulated motor-controller model in this change, with a stable deliberate identity and source-faithful selected VESC layout/version fields and Refloat INFO, layout, and telemetry responses.
- Generate smooth, plausible, seeded Refloat telemetry for manual riding simulation while preserving deterministic canonical conformance scenarios.
- Reuse the existing Express/Thor300/Refloat identity, topology, layout, framing, serial client, and request safety policy owned by `/conformance`.
- Keep the SvelteKit application development-only: no production adapter, build artifact, packaging, deployment, or production runtime is introduced.
- Preserve the pure ESP-IDF byte-bridge firmware and the existing meanings of `just fixture build`, `flash`, `monitor`, and physical conformance commands.

Direct topology reproduces source-faithful identity and package discovery, but the current native backends do not issue direct-controller Refloat telemetry requests. Direct riding therefore remains unavailable to the product frontend until a follow-up backend compatibility change. Older Refloat selections likewise expose each current backend's existing unsupported or connected-without-telemetry outcome without forcing a synthetic failure.

Non-goals include controlling a motor, emulating mutating VESC commands, simulating multiple simultaneous CAN motor controllers, simulating controller models other than THOR300, replacing canonical automated conformance, deploying the controller, sharing product frontend components, expanding Float View's current Refloat 1.2 compatibility policy, or changing the frontend/backend wire contract, native behavior, persistence, or cross-platform behavior.

## Capabilities

### New Capabilities

- `interactive-fixture-development`: Development-only SvelteKit control UI, server-side serial coordination, interactive Refloat state simulation, lifecycle safety, and verification.

### Modified Capabilities

None.

## Impact

- Adds SvelteKit/Vite development dependencies and scripts to `/conformance`, which remains the owner of host serial and VESC simulation dependencies.
- Adds an ownership-documented `/conformance/fixture-dev` SvelteKit source tree, keeps reusable device simulation under `/conformance/fixture`, and extends the root `just fixture` dispatcher.
- Adds immutable protocol fixtures and composable typed firmware, Refloat, controller, and topology variants derived from the tagged Refloat source and VESC firmware source/archive.
- Refactors shared conformance simulation only as needed to support direct and CAN-forwarded composed read-only behavior, configurable CAN identity, and a pluggable interactive telemetry source while retaining deterministic physical tests.
- Adds no production artifact, API, persistence schema, native-backend behavior, firmware command, or platform-specific frontend code.
- No superseded production or compatibility paths are introduced; the development controller stops and cleans up the serial resource it owns.
