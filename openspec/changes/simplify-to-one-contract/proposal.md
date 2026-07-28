## Why

Float View accumulated generated contracts, speculative operations, duplicated test harnesses, and fine-grained native build modules before the product required them. The repository should be reduced now to the minimum architecture the product actually needs: one bundled frontend, one mandatory backend contract, one Apple backend shared by iOS and macOS, one Android backend, and one canonical conformance suite backed by a small programmable VESC BLE fixture.

## What Changes

- **BREAKING** Replace the generated JSON Schema/TypeScript/Swift/Kotlin API with one concise, hand-written TypeScript contract used directly by the frontend and canonical conformance suite.
- **BREAKING** Remove API version negotiation, capability negotiation, speculative methods, unused response fields, and compatibility machinery. A supported backend implements the complete current contract.
- **BREAKING** Reduce the loopback surface to an embedded frontend, `GET /api/bootstrap`, and one authenticated WebSocket endpoint.
- Adopt lowercase ownership boundaries: `/frontend` for the SvelteKit application, `/backends/apple` for the shared Apple backend and iOS/macOS hosts, `/backends/android` for the Android implementation and host, top-level `/conformance` for normative acceptance, and `/tools` only for supporting developer instruments.
- Build the SvelteKit, TypeScript, and Tailwind frontend as one inline HTML artifact and embed the identical bytes in Android, iOS, and macOS.
- Replace invalidation-only notifications and collection fan-out with an authoritative application state and a full `state.changed` event.
- Collapse the Apple implementation into one shared backend used by thin iOS and macOS hosts.
- Collapse Android into one application module with package and file boundaries instead of Gradle module boundaries.
- Establish one TypeScript black-box conformance suite as the acceptance authority for the mock, Android, iOS, and macOS backends.
- Retain `tools/vesc-test-device` as a pure ESP-IDF BLE bridge with no TypeScript package or VESC/scenario knowledge. Canonical conformance owns serial orchestration, typed boards, scenarios, VESC simulation, cadence checks, and failure injection. The firmware input protocol is limited to `RESET`, `ADVERTISE`, `NOTIFY <HEX>`, `DISCONNECT`, and `LED <RRGGBB>`.
- Preserve the current disposable-schema persistence policy and the two shared SQLite schema files while removing unused product and alert machinery.
- Delete generated sources, API schemas and examples made redundant by conformance, native contract smoke modules, duplicate conformance packages and launchers, unused dependencies, historical evidence, empty scaffolding, and other code outside the current frontend/contract/backend/conformance path.

## Capabilities

### New Capabilities

- `shared-frontend-artifact`: The SvelteKit frontend builds to one inline HTML resource that every native host serves unchanged.
- `backend-contract`: The complete platform-neutral wire contract, authoritative application state, methods, events, errors, transport security, and lifecycle semantics.
- `backend-conformance`: The single executable suite every supported backend must pass.
- `platform-hosts`: One shared Apple backend with thin iOS/macOS hosts and one single-module Android backend.
- `programmable-vesc-fixture`: A minimal, controllable ESP32 BLE bridge firmware used by canonical physical conformance scenarios.

### Modified Capabilities

None. There are no existing main OpenSpec capabilities.

## Impact

- Removes `api/`, `android/api-models/`, API generator scripts, generated native checks, and associated JavaScript/Kotlin dependencies.
- Moves `web/` to `frontend/`, Apple and iOS sources under `backends/apple/`, Android under `backends/android/`, and shared backend SQL/fixtures under `backends/`.
- Replaces the current JSON-RPC v1 method surface and therefore requires coordinated frontend, mock, Swift, Kotlin, and conformance changes.
- Reorganizes Apple SwiftPM targets, the iOS project dependency, and Android Gradle modules without changing supported OS versions.
- Simplifies native static-content serving to one embedded HTML response.
- Reworks top-level `conformance/` as the owner of software and physical simulation, reduces `tools/vesc-test-device/` to firmware only, and updates the root `justfile`, repository documentation, and `just verify` gate.
- Keeps native Bluetooth, background execution, VESC framing/decoding, durable SQLite logging, automatic sessions, Board Profiles, session review/export/deletion/grouping, and current frontend-visible settings.

## Non-goals

- Adding product features, supported platforms, remote networking, cloud synchronization, motor control, firmware/configuration writes, or database migrations.
- Preserving unused API methods or compatibility with unreleased wire contracts.
- Sharing Swift/Kotlin backend implementation code across operating-system families.
- Replacing SvelteKit, TypeScript, Tailwind, CoreBluetooth, Android Bluetooth APIs, SQLite, FlyingFox, or Ktor solely to reduce dependency count.
