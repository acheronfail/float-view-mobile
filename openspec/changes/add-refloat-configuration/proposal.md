## Why

Float View can observe and perform a small allowlisted set of Refloat operations, but riders still need VESC Tool to inspect, tune, back up, and restore the complete version-specific Refloat configuration. VESC and Refloat already expose a self-describing custom-configuration protocol, so Float View can provide a safe generated editor and exact-version saved configurations without hardcoding each release's fields into the shared frontend.

## What Changes

- Add runtime discovery of Refloat custom configuration index `0`, including bounded retrieval and validation of its compressed XML schema, audited full-schema fingerprint, current values, and package defaults through the selected direct-or-CAN controller route; use the fingerprint to distinguish legacy 1.0.0 from 1.0.1 because INFO reports only `1.0`.
- Add mirrored native schema, signature, and value codecs that normalize supported parameter metadata for a generated frontend editor and serialize only complete validated configurations matching the active exact package version and schema signature.
- Add mandatory bounded backend operations for paged schema reads, current/default value reads, persistent writes, and exact-version saved-configuration list/create/update/delete/apply workflows without placing the large schema in authoritative `AppState`; withhold current-value reads while flywheel mode exposes Refloat's temporary in-memory calibration state.
- Add a `Configure Refloat` action to `/live` that opens a grouped, searchable generated form with dirty-state handling, source selection, validation, a write diff and safety confirmation, and mandatory device readback verification.
- Add a `Tunes…` action for bounded named full Refloat configurations stored by exact package version and schema signature; configurations are never migrated or offered across Refloat versions.
- Extend native catalog persistence, the mock, immutable upstream-derived fixtures, the programmable fixture, and canonical conformance across every audited stable Refloat 1.0.0–1.2.3 release, direct and CAN routes, malformed schemas/values, stale generations, rejected writes, and readback mismatch.
- Replace the post-`add-refloat-live-controls` rejection of arbitrary persistent configuration with this narrower safety-gated custom-configuration path while continuing to reject motor control, firmware mutation, unsupported package commands, and unknown or prerelease Refloat versions. Route semantic controls, full configuration writes, and Tune apply through one native mutation coordinator; while flywheel mode is active it rejects every device mutation except the exact eligible flywheel-disable command.
- Require this change to be revised against canonical specs and repository ownership after all preceding OpenSpec changes, especially `support-vesc-bms-telemetry` and `add-refloat-live-controls`, are implemented, synced, and archived; implementation MUST NOT begin from stale planning artifacts.

Non-goals are support for unaudited Refloat releases, automatic cross-version Tune migration, reproducing Refloat's bespoke QML application, runtime motor/remote/Handtest controls, firmware or motor-configuration editing, data recording/history commands, and optimistic or partial configuration writes.

## Capabilities

### New Capabilities

- `refloat-configuration`: Define runtime custom-configuration schema discovery and audited fingerprinting, exact serialization, current/default reads, safe persistent writes with readback, exact-version saved configurations, caching, and lifecycle behavior.

### Modified Capabilities

- `backend-contract`: Add mandatory bounded Refloat configuration and saved-configuration operations plus a small active-connection availability summary.
- `live-board-dashboard`: Add the generated Configure Refloat experience and exact-version Tunes entry to the connected-board action surface.
- `vesc-device-connection`: Retain bounded custom-config identity metadata and extend centralized route, scheduler, compatibility, and read/write policy to VESC commands 92–95.
- `refloat-telemetry-pipeline`: Permit only the audited custom-configuration protocol in addition to existing telemetry and product controls, and provide the authoritative idle/mode evidence required by configuration safety gates.
- `refloat-live-controls`: Coordinate full configuration writes with semantic Refloat controls through one mutation authority, preserve dedicated product-control behavior, synchronize shared persistent fields, and distinguish runtime lighting overrides from configuration defaults.
- `platform-hosts`: Add mirrored native Refloat configuration ownership and exact-version durable Tune storage while keeping XML and raw wire data out of the frontend.
- `backend-conformance`: Cover the complete contract, wire codecs, bounds, persistence, safety, direct/CAN routing, scheduler arbitration, and readback verification across mock, Apple, and Android.
- `interactive-fixture-development`: Add source-faithful versioned custom schemas and disposable current/default configuration state while preserving the strict mutation allowlist and no-motor behavior.

## Impact

The hand-written TypeScript contract and mock, frontend connection feature and `/live` menu, mirrored Swift/Kotlin VESC and Refloat protocol/application owners, authenticated loopback handlers, native SQLite catalog stores, SQL schema version, canonical fixtures/conformance suite, and ESP32 test-device behavior change together. Implementation depends on the connection-wide `VescRequestScheduler` and live-control ownership being present after earlier changes are archived, consolidates all board mutations behind one native coordinator, adds no third-party runtime dependency, and removes superseded blanket custom-config rejection paths rather than introducing a parallel request coordinator or platform-specific frontend.
