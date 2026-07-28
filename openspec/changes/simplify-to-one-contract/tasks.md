## Execution checkpoint

Implement and verify every task in this plan without requiring connected physical devices. Local iOS Simulator and Android Emulator execution, native compilation, ESP-IDF compilation, host-side fixture simulation, and deterministic fakes are part of this checkpoint. Leave only tasks 7.5 and 9.6 unchecked for the follow-up physical-device session; do not retain legacy code as a temporary fallback for that session.

## 1. Establish the new frontend contract

- [x] 1.1 Move the conventional SvelteKit project from `web/` to lowercase `frontend/` and update root commands, configuration, ignored paths, and native build inputs without changing the `npm run dev`, `npm run build`, or `npm test` interface.
- [x] 1.2 Add `frontend/src/lib/backend/contract.ts` with the mandatory method map, request/result/event envelopes, stable errors, bounded `AppState`, retained resources, and no generated or platform-specific types.
- [x] 1.3 Replace the JSON-RPC client with the small authenticated WebSocket protocol and add focused tests for one response per ID, failures, state events, stale revisions, closure, timeout, and message bounds.
- [x] 1.4 Refactor the frontend store to consume authoritative `AppState`, remove collection fan-out and firmware polling, combine session summary/series loading, and delete calls to methods outside the mandatory contract.
- [x] 1.5 Reduce the deterministic mock to the mandatory contract and retained frontend behavior, then make it publish complete monotonic `state.changed` events.
- [x] 1.6 Configure SvelteKit inline bundle output at `frontend/build/index.html`, remove or inline external runtime assets, and add a build assertion that only one self-contained HTML resource must be shipped.
- [x] 1.7 Verify `npm run dev`, frontend hot reload, `npm run build`, frontend type checking, and frontend unit tests before changing native hosts.

## 2. Create canonical conformance

- [x] 2.1 Reorganize top-level `conformance/` into the one canonical TypeScript package, import the contract from `frontend/`, and move any hardware-only serial dependency out of the frontend and fixture firmware.
- [x] 2.2 Implement shared target/client interfaces and one bounded WebSocket client used by mock, Android, iOS, and macOS target adapters.
- [x] 2.3 Move serial fixture control, VESC frame reassembly/classification, typed board configurations, telemetry phases, fragmentation, cadence assertions, disconnect injection, read-only enforcement, and bounded diagnostics into `conformance/fixture/`.
- [x] 2.4 Cover bootstrap authentication, valid and malformed envelopes, exactly-one-response behavior, stable errors, message/queue bounds, initial state, monotonic state events, stale revision handling, and unknown methods.
- [x] 2.5 Cover every retained Bluetooth, device, settings, Board Profile, session review/diagnostic/export/delete, and session-group operation with success and relevant failure scenarios.
- [x] 2.6 Cover WebView recreation, reconnect/disconnect ordering, automatic durable session lifecycle, interrupted-session recovery, current-schema recreation, and bounded collection behavior.
- [x] 2.7 Make the reduced mock pass the complete canonical suite and remove old conformance scenario implementations superseded by it.

## 3. Simplify the shared Apple backend and hosts

- [x] 3.1 Move `apple/` and `ios/` under lowercase `backends/apple/`, keeping conventional SwiftPM `Sources/<Target>` paths and the iOS Xcode project under `backends/apple/ios/`.
- [x] 3.2 Replace the Apple JSON-RPC dispatcher with the mandatory request envelopes, stable errors, authoritative `AppState`, and complete `state.changed` publication.
- [x] 3.3 Reduce the Apple loopback server to embedded `/`, `/api/bootstrap`, and authenticated `/api/ws` routes with no filesystem serving or web-root discovery.
- [x] 3.4 Move firmware identity, saved devices, recent sessions, groups, and settings into bounded Apple application-state snapshots and remove obsolete public API models and operations.
- [x] 3.5 Collapse the Apple package into `FloatViewBackend`, `FloatViewMac`, and focused backend tests while preserving source-file boundaries for service, Bluetooth, protocol, storage, and server code.
- [x] 3.6 Make iOS consume the shared Apple backend and reduce iOS/macOS shells to entry, lifecycle, WebView, permission/export presentation, and one embedded HTML resource.
- [x] 3.7 Remove `FrontendLifecycleCoordinator` and its tests after direct host lifecycle wiring proves backend survival and frontend recovery.
- [x] 3.8 Add minimal macOS and iOS conformance target adapters and make both hosts pass canonical software conformance.

## 4. Simplify the Android backend and host

- [x] 4.1 Move `android/` under lowercase `backends/android/` and update root commands, Gradle paths, frontend resource inputs, fixtures, and conformance target discovery.
- [x] 4.2 Replace the Android JSON-RPC dispatcher with the mandatory request envelopes, stable errors, authoritative `AppState`, and complete `state.changed` publication.
- [x] 4.3 Reduce the Android loopback server to embedded `/`, `/api/bootstrap`, and authenticated `/api/ws` routes with no asset-directory file server.
- [x] 4.4 Move firmware identity, saved devices, recent sessions, groups, and settings into bounded Android application-state snapshots and remove obsolete public API models and operations.
- [x] 4.5 Move current `core`, `protocol`, `logging`, and `bluetooth` sources and focused tests into concrete packages within `:app`, then remove their Gradle modules, catch-all `core` package, and inter-module dependency wiring.
- [x] 4.6 Remove `:api-models`, Kotlin serialization generation dependencies, unused AppCompat/Material or other dependencies where current host behavior no longer requires them, and verify the one-module application build.
- [x] 4.7 Embed the same single HTML resource as Apple and preserve process-owned Bluetooth, background logging, permission, export, and WebView recreation behavior.
- [x] 4.8 Add the minimal Android conformance target adapter and make the application pass canonical software conformance.

## 5. Prune retained backend product code

- [x] 5.1 Remove alert rules/evaluation and any other application-service or persistence code not exercised by the retained frontend, mandatory contract, protocol safety, or durable-session invariants.
- [x] 5.2 Move the two shared current SQLite schemas to `backends/sql/` and immutable cross-language VESC/Refloat fixtures to `backends/fixtures/`; update both implementations without introducing a generic shared-code layer.
- [x] 5.3 Remove unused SQL columns/tables only when proven unreachable, bump storage versions for any schema change, and retain WAL, integrity, recovery, and disposable-schema tests.
- [x] 5.4 Reduce duplicated Swift/Kotlin domain types and ports to those required by current backend behavior or deterministic native tests while preserving OS, clock/scheduler, protocol, and storage test boundaries.
- [x] 5.5 Search all frontend and native implementations for removed methods, capability/version negotiation, logging-job/telemetry-subscription payloads, and obsolete error codes; delete every remaining path.

## 6. Reduce the ESP32 fixture firmware

- [x] 6.1 Flatten `tools/vesc-test-device/firmware/` into a conventional pure ESP-IDF project at `tools/vesc-test-device/` with no Node package, TypeScript, board/scenario data, platform launcher, or evidence code.
- [x] 6.2 Replace the configurable fixture control layer with the fixed canonical VESC UART GATT service and input commands `RESET`, `ADVERTISE`, `NOTIFY <HEX>`, `DISCONNECT`, and `LED <RRGGBB>`.
- [x] 6.3 Emit only bounded `READY`, `CONNECTED`, `DISCONNECTED`, `WRITE <HEX>`, and `ERROR <CODE>` events, forwarding exact GATT writes without VESC/Refloat interpretation.
- [x] 6.4 Remove the flat-JSON parser, dynamic GATT profile/write policy, unnecessary LED/state machinery, and firmware code not required to bridge GATT writes, notifications, disconnects, advertising, reset, and RGB output.
- [x] 6.5 Enforce strict serial command lengths, hex validation, BLE payload bounds, read/write buffer bounds, and deterministic events.
- [x] 6.6 Add focused ESP-IDF tests for serial parsing, exact GATT/notification forwarding, malformed commands, RGB values including `000000`, and bounded input.

## 7. Consolidate VESC simulation into conformance

- [x] 7.1 Replace JSON Schema scenarios and the generic occurrence/rule engine with typed board configurations, telemetry phases, fragmentation options, cadence expectations, and disconnect injection under `conformance/fixture/`.
- [x] 7.2 Implement one small conformance simulator that reassembles VESC frames, recognizes the required read-only requests, returns configured identity/topology/Refloat/telemetry, and rejects unknown or mutating commands.
- [x] 7.3 Implement the conformance-owned serial fixture client and bounded diagnostics, including orchestrator-controlled blue/cyan/yellow/green/red/off LED conventions and first-failure output.
- [x] 7.4 Express current ride, unexpected-disconnect, background, and hardware-identity coverage as concise canonical conformance scenarios; replace soak-specific programs with bounded repetition parameters where still valuable.
- [ ] 7.5 Connect the pure-firmware physical fixture to canonical conformance so Android, iOS, and macOS use the same contract scenarios through production OS Bluetooth stacks.
- [x] 7.6 Delete the former `tools/vesc-test-device/host` package, AJV schemas, evidence generation, retained transcripts/manifests, `T3-EVIDENCE.md`, standalone Swift verifier, duplicate platform verifiers, preflight wrappers, and unused dependencies.
- [x] 7.7 Run conformance simulator/serial tests and compile the pure ESP-IDF fixture before physical platform runs.

## 8. Delete obsolete repository structure and document the result

- [x] 8.1 Delete `api/`, all generated TypeScript/Swift/Kotlin models, API generator/validator scripts, native contract smoke tests, compatibility documents, and imports or commands that reference them.
- [x] 8.2 Remove superseded conformance implementations/launchers, obsolete VESC scenarios/evidence, historical references, empty scaffolding, build artifacts, old `web/`, `apple/`, `ios/`, `android/`, `logging/`, and root `fixtures/` paths, and tracked files outside the retained ownership boundaries.
- [x] 8.3 Remove unused npm, Gradle, and Swift dependencies and regenerate only required lockfiles.
- [x] 8.4 Write a concise root README describing lowercase `/frontend`, `/backends/apple`, `/backends/android`, top-level `/conformance`, the pure-firmware `/tools/vesc-test-device`, ownership rules, generated-resource flow, and normal commands.
- [x] 8.5 Update `AGENTS.md`, OpenSpec context, `justfile`, and platform configuration to describe only the simplified repository and delete historical command aliases or compatibility guidance.
- [x] 8.6 Add deletion audits that fail on generated API directories, removed method names, obsolete modules/targets, old top-level platform paths, duplicate frontend artifacts, TypeScript under the fixture firmware project, or unused fixture framework files.

## 9. Verify the simplified repository

- [x] 9.1 Run focused frontend build/type/unit checks and assert the single inline HTML artifact.
- [x] 9.2 Run focused Apple backend, protocol, storage, host, macOS, and iOS Simulator build tests without requiring a connected Apple device.
- [x] 9.3 Run focused one-module Android backend, protocol, storage, host, application build, and Android Emulator tests without requiring a connected Android device.
- [x] 9.4 Run fixture firmware checks and TypeScript simulator tests.
- [x] 9.5 Run canonical software conformance against mock, Android Emulator, iOS Simulator, and macOS with no physical BLE dependency.
- [ ] 9.6 **PHYSICAL CHECKPOINT:** Run canonical physical BLE conformance against Android, iOS, and macOS with the flashed programmable VESC fixture and connected target devices.
- [x] 9.7 Run `just verify` as the final software gate and confirm the repository contains no parallel legacy contract or backend path.
