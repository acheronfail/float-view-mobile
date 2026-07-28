## Context

Float View ships one SvelteKit frontend inside Android, iOS, and macOS applications. The native process owns Bluetooth, VESC communication, lifecycle, background work, SQLite storage, and a loopback WebSocket backend. The current repository expresses that boundary through seven JSON Schemas, three generated model outputs, 32 advertised methods, API negotiation, capability negotiation, separate native contract checks, a separate conformance package, six Android modules, six Apple targets, and a programmable hardware harness of roughly 5,900 lines.

Only the generated TypeScript model is consumed by production frontend code. Native hosts construct and parse JSON manually. The frontend currently exercises a smaller method set than the schemas and mock advertise. Because each frontend build is bundled with its backend, there is no independently deployed client that requires runtime API-version negotiation.

The retained product behavior is the behavior visible in the current frontend: Bluetooth permission and discovery, one active connection, firmware identity, durable automatic sessions, saved boards and Board Profiles, settings, session lists/review/diagnostics/export/deletion/grouping, backend survival across WebView recreation, and current-schema SQLite recovery.

## Goals / Non-Goals

**Goals:**

- Make the backend boundary small enough to read and implement directly.
- Make supported mean complete conformance, without optional methods or frontend platform branches.
- Ship one identical inline HTML resource in all three applications.
- Keep one shared Apple backend and one Android backend while retaining clear source-file dependency direction.
- Use one TypeScript conformance suite as the acceptance authority for mock and native targets.
- Exercise native implementations through real OS Bluetooth stacks with a small deterministic ESP32 VESC fixture.
- Delete generated, speculative, duplicated, historical, and empty repository content.
- Keep `npm run dev`, `npm run build`, root `just` commands, and `just verify` as the entire normal developer interface.

**Non-Goals:**

- New product features, protocols, platforms, remote clients, or cloud services.
- Compatibility with the unreleased JSON-RPC v1 contract.
- Database migrations or preservation of test-stage data across schema changes.
- Cross-language code generation or sharing implementation code between Swift and Kotlin.
- Replacing SvelteKit, TypeScript, Tailwind, platform Bluetooth APIs, SQLite, FlyingFox, or Ktor without a concrete simplification benefit.

## Decisions

### 1. Top-level directories express ownership

The repository uses lowercase ownership boundaries:

```text
frontend/                       SvelteKit application and structural contract
backends/
  apple/                        shared Apple backend plus iOS/macOS hosts
  android/                      one-module Android backend and host
  fixtures/                     immutable Swift/Kotlin protocol fixtures
  sql/                          shared current SQLite schemas
conformance/                    canonical software and physical acceptance
tools/
  vesc-test-device/             pure ESP-IDF BLE bridge firmware
openspec/                       change planning
```

Top-level directories do not represent generic technical layers. There are two production backend implementations, not separate iOS and macOS backends. Hosts live with the backend they host; tests live with their owner except for cross-backend conformance. Generated output exists only in ignored build directories.

Directories named `common`, `shared`, `core`, `helpers`, or `utils` are not introduced as catch-alls. Cross-language immutable data may be shared under `backends/`; Swift/Kotlin implementation code may not. `conformance/` remains top-level because it is the executable acceptance authority, while `/tools` contains optional supporting instruments.

Alternatives rejected include `backends/ios` plus `backends/macos`, which hides the one Apple implementation; a generic `packages/` monorepo layer; and placing conformance under `/tools`, which would incorrectly present required acceptance as optional tooling.

### 2. The frontend contract is hand-written TypeScript

`frontend/src/lib/backend/contract.ts` will be the single structural definition of requests, results, events, application state, resources, and errors. The frontend client, store, deterministic mock, and conformance suite import it directly. Native developers implement that readable contract and prove compatibility through black-box conformance.

This replaces JSON Schema and Swift/Kotlin generation. Keeping generation for one consumer preserves more machinery than value; making Swift/Kotlin generated models production dependencies would instead add translation and dispatcher work to already functioning native implementations.

### 3. Supported backends implement one complete current contract

The contract will not negotiate API versions, backend identity, capabilities, limits, or optional operations. The frontend and backend ship atomically in one application, and all Android, iOS, and macOS builds must implement the same current behavior. Platform constraints appear as state or stable errors, never as missing methods or platform branches.

The initial mandatory request set is:

```text
state.get
bluetooth.requestPermission
bluetooth.scan
device.connect
device.disconnect
settings.update
boardProfile.get
boardProfile.update
session.get
session.getDiagnostics
session.export
session.delete
sessionGroup.unlink
```

`state.get` returns the authoritative `AppState`, including Bluetooth state, discovered and saved devices, active connection and firmware identity, recent session summaries, session groups, and settings. It replaces snapshot, collection-list, settings-get, firmware polling, active-device, telemetry-subscription, and logging-job contract surfaces.

`session.get` returns the summary and bounded display series required by the current review screen. Diagnostics remain separate because they are loaded on demand and may be lossless over a bounded window.

Mutations return their directly updated resource or acceptance result. Asynchronous Bluetooth, connection, and session transitions are represented by `state.changed`.

### 4. Use a small WebSocket protocol rather than JSON-RPC

The wire envelopes are:

```ts
type Request = { id: number; method: BackendMethod; params: unknown };
type Success = { id: number; result: unknown };
type Failure = { id: number; error: BackendError };
type StateChanged = { event: 'state.changed'; data: AppState };
```

Stable error codes are limited to:

```text
invalid_request
invalid_state
permission_required
not_found
conflict
unavailable
internal
```

Each valid request ID receives exactly one success or failure. IDs, messages, queues, parameters, series windows, result collections, reconnect attempts, and timeouts remain bounded.

JSON-RPC was considered but rejected because its version marker, numeric error layer, method-not-found conventions, and generated request/response envelopes do not improve a private, bundled, single-client protocol.

### 5. Send authoritative state in notifications

Every externally visible coarse state transition increments a monotonic revision and emits `state.changed` with the complete new `AppState`. The frontend replaces its state atomically and ignores revisions older than the newest accepted revision.

This replaces invalidation notifications followed by five collection requests, refresh debouncing, firmware polling, and much of the frontend wait-for-snapshot logic. Large session series and diagnostics remain request results and are not placed in `AppState`.

### 6. Build and embed one HTML resource

SvelteKit, TypeScript, Tailwind, and `adapter-static` remain under `frontend/`. SvelteKit uses inline bundle output so `frontend/build/index.html` is the one canonical shipped frontend artifact with its runtime JavaScript and CSS inline. External runtime assets are removed or embedded.

Each native host implements only:

```text
GET /                 embedded HTML
GET /api/bootstrap    WebSocket URL and random launch token
WS  /api/ws           authenticated backend protocol
```

The WebSocket binds only to loopback, requires the launch token, validates the expected Origin, and enforces message and queue bounds. The native hosts retain no general static-file server, MIME table, traversal logic, or web-root discovery.

### 7. Keep two backend implementations and three thin hosts

`backends/apple/` contains one `FloatViewBackend` library target with application service, domain, Bluetooth, VESC protocol, storage, and loopback server source files. macOS remains a conventional SwiftPM executable under `Sources/FloatViewMac`; `backends/apple/ios/` contains the Xcode application consuming the same backend library and owns only application lifecycle, WebView, permission presentation, and export presentation.

`backends/android/` contains one `:app` module. Architectural boundaries remain concrete Kotlin packages and source files such as `backend`, `bluetooth`, `protocol`, `storage`, and `host` rather than Gradle modules or catch-all `core`/`common` packages. Android application/process services continue owning backend lifetime; the Activity owns only WebView and presentation concerns.

This preserves dependency direction inside source code while removing build-system boundaries that do not correspond to independently reusable products.

### 8. Conformance is the acceptance authority

One TypeScript suite connects to a backend as a black box and validates every mandatory method, result shape, stable failure, ordering rule, lifecycle transition, persistence behavior, and bound. Target adapters only build, install, launch, expose bootstrap readiness, and stop mock, Android, iOS, or macOS.

There is one top-level `conformance/` TypeScript package and no language-specific copy. It imports the hand-written frontend contract and owns any hardware-only serial dependency rather than adding it to the frontend. `just verify` runs fast software conformance; explicit hardware commands run the same relevant scenarios through real Bluetooth.

Native unit tests remain focused on protocol decoding, application-state transitions, storage recovery/integrity, and OS policies that cannot be observed efficiently through the wire.

### 9. The programmable VESC fixture is pure firmware; conformance owns simulation

`tools/vesc-test-device/` is a conventional ESP-IDF project containing `CMakeLists.txt`, `sdkconfig.defaults`, `main/`, and a concise README. It has no `package.json`, TypeScript, generic scenario engine, VESC framing logic, board definitions, or platform verifier.

The ESP32 firmware hardcodes the canonical VESC UART GATT service and accepts only:

```text
RESET
ADVERTISE
NOTIFY <HEX>
DISCONNECT
LED <RRGGBB>
```

It reports only the bounded events required by conformance:

```text
READY
CONNECTED
DISCONNECTED
WRITE <HEX>
ERROR <CODE>
```

The firmware is a byte bridge and LED diagnostic, not a VESC simulator. It forwards accepted GATT writes to conformance, sends exact notification bytes supplied by conformance, disconnects when commanded, and rejects malformed control input.

`conformance/fixture/` owns the serial client, VESC frame assembler and classifier, typed board configurations, telemetry phases, fragmentation, request cadence, disconnect injection, read-only safety checks, and bounded failure diagnostics. When an application writes over real BLE, the firmware emits `WRITE`; conformance selects the deterministic response and sends `NOTIFY`. This lets the same simulator semantics drive software and real-Bluetooth scenarios.

Conformance rejects unknown, motor-control, firmware, and persistent-configuration requests. The orchestrator uses `LED <RRGGBB>` for initialization, advertising, running, pass, failure, and cleanup diagnostics. Platform launch adapters live under `conformance/targets/`. Historical evidence, manifests, retained transcripts, soak-specific programs, the standalone Swift verifier, AJV schemas, and generic occurrence scheduling are removed. Failure diagnostics are produced on demand.

### 10. Keep the current shared SQL schema and delete unused product machinery

The two current SQLite schema files move to `backends/sql/` and remain shared inputs to Swift and Kotlin stores. Protocol fixtures required by both implementations move to `backends/fixtures/`. Schema-version mismatch continues deleting and recreating test-stage storage; no migration history is introduced.

Alert rules/evaluation and contract operations not exercised by the retained frontend are deleted. Protocol fixtures required by Swift and Kotlin decoder tests remain small and platform-neutral.

### 11. Make the root describe and verify the actual repository

A concise root README will identify `/frontend`, `/backends/apple`, `/backends/android`, top-level `/conformance`, the pure-firmware fixture under `/tools`, and normal commands. `AGENTS.md`, OpenSpec context, and compatibility text will be updated to remove generated-schema, obsolete module, and historical path guidance.

`just setup`, `just dev`, `just run <platform>`, `just test`, `just verify`, and the small `just vesc` surface remain. Wrapper scripts and preflight commands that do not replace meaningful complexity are removed.

## Risks / Trade-offs

- **[Risk] Hand-written native payload construction can drift from TypeScript structures.** → The canonical suite validates every frontend-visible request, result, event, and error against every supported target.
- **[Risk] Full-state notifications can become large or frequent.** → Keep `AppState` coarse and bounded, exclude series/diagnostics, coalesce pending state events, cap collections, and assert payload bounds in conformance.
- **[Risk] Removing runtime negotiation prevents independently upgrading frontend and backend.** → They are embedded and released atomically; any future independent client requires a separately proposed compatibility boundary.
- **[Risk] Collapsing modules can obscure architectural boundaries.** → Preserve source directories, package namespaces, explicit protocols/interfaces at OS and persistence boundaries, and dependency rules in repository guidance.
- **[Risk] A single HTML artifact can accidentally reference external build assets.** → Add a build assertion and compare the embedded bytes across all platform builds.
- **[Risk] Physical BLE tests are slower and hardware-dependent.** → Keep deterministic software conformance in `just verify`; make hardware conformance explicit while reusing the same scenarios and assertions.
- **[Risk] Moving simulation out of the firmware requires a reliable serial bridge.** → Keep the serial protocol fixed and bounded, make conformance own its serial dependency, and test both directions independently.
- **[Risk] Simplifying the VESC simulator can remove useful diagnostics.** → Retain bounded request timing, first-failure reporting, read-only enforcement, fragmentation control, and LED status in conformance while deleting historical evidence infrastructure.
- **[Risk] A large breaking refactor can leave parallel old and new paths.** → Migrate vertically, keep the repository building at each stage where practical, and finish with searches proving old schemas, methods, modules, dependencies, and scripts are gone.

## Migration Plan

1. Move the SvelteKit project to `frontend/`, establish the new contract, mock, canonical conformance client, and inline artifact while old native hosts still compile.
2. Move Apple/iOS into `backends/apple/`, replace the Apple loopback API and state publication, then run conformance against macOS and iOS.
3. Move Android into `backends/android/`, replace its API, collapse its modules, and run the same conformance.
4. Move SQL and protocol fixtures under `backends/`, reduce `tools/vesc-test-device` to pure ESP-IDF firmware, and move all simulator/orchestrator behavior into canonical conformance.
5. Delete the old API, generated models, obsolete operations, duplicate harnesses, modules, targets, dependencies, evidence, and documentation.
6. Run focused frontend/native/fixture checks followed by `just verify` and supported hardware scenarios.

### Implementation checkpoint without physical devices

The repository restructuring, production implementation, deterministic tests, simulator/emulator builds, fixture firmware source, host-side VESC simulation, canonical software conformance, deletion work, and `just verify` gate do not require a connected iOS device, Android device, or ESP32. They are implemented and committed as one software-complete checkpoint.

Physical validation is a deliberately small subsequent checkpoint. Only connecting canonical conformance through the flashed fixture and running the final real-Bluetooth platform matrix are deferred. The software checkpoint therefore completes 59 of 61 tasks; tasks 7.5 and 9.6 remain unchecked until the ESP32 and target devices are available. A failed local toolchain or simulator setup is an implementation blocker to resolve, not grounds to defer additional planned work.

Rollback is a source-level revert before release. Stored test data is disposable; if the retained SQL schema changes during simplification, bump the storage version and recreate it rather than adding migrations.

## Open Questions

None. The retained product surface is the behavior currently visible in the frontend; future features require a new contract change and conformance scenarios.
