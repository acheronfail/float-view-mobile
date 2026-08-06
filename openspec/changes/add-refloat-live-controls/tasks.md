## 1. Scheduler and protocol fixtures

- [ ] 1.1 Land or integrate the connection-wide `VescRequestScheduler` from `support-vesc-bms-telemetry`, including route/generation cancellation and priority tests required by Refloat controls.
- [ ] 1.2 Add source-derived golden request and response fixtures for package command 12, flywheel command 22, legacy lighting command 202, stable lighting command 20, and Refloat 1.2.x fatal-clear command 36.
- [ ] 1.3 Add malformed, oversized, wrong-route, test-version, and unsupported-command fixture cases that prove the product-control allowlist fails closed.

## 2. Shared backend contract and mock

- [ ] 2.1 Extend `frontend/src/lib/backend/contract.ts` with the complete live-only `RefloatControls` state and the `refloat.control.set` and `refloat.fatalError.clear` request/result shapes and stable errors.
- [ ] 2.2 Update the frontend backend client and state ownership so control methods consume complete monotonic snapshots without optimistic mutation or persistence.
- [ ] 2.3 Implement stateful mock behavior for all four boolean controls, conditional fatal clearing, pending actions, safety/read-only rejection, the flywheel-mode prohibition with only the flywheel-disable exception, timeouts, reconnect reset, and external lighting refresh.
- [ ] 2.4 Add contract and mock tests for validation, idempotence, one-action bounds, exact revision changes, guest/saved parity, and frontend recreation.

## 3. Apple Refloat protocol and control owner

- [ ] 3.1 Add exact Apple control-profile selection for audited Refloat 1.0.x, 1.1.0–1.1.2, and 1.2.0–1.2.3 packages, preserving read-only access for audited test firmware and failing closed for every other version.
- [ ] 3.2 Implement and golden-test Swift encoders/decoders for the allowlisted commands, route correlation, lighting capability/state, and distinct Refloat 1.2.x active-fault and fatal-latch fields.
- [ ] 3.3 Add the Apple application-service Refloat control owner and sole extensible connection-scoped `RefloatMutationCoordinator` with current-generation authorization, idle/mode/fault gates, idempotence, one pending action, three-second confirmation, lifecycle cancellation, no parallel write path, and rejection of every flywheel-mode mutation except eligible flywheel disable.
- [ ] 3.4 Submit Apple control writes, one-way confirmation telemetry, and one-second best-effort lighting refresh through the shared scheduler with riding telemetry priority and no catch-up queue.
- [ ] 3.5 Implement Apple loopback handlers and authoritative snapshots for both semantic methods, including bounded diagnostics and exact error mapping.
- [ ] 3.6 Add Apple unit/integration tests for direct and CAN routes, every release-family codec, rejected safety states including every prohibited flywheel-mode write, the sole command-22 disable exception, malformed/late responses, timeout, disconnect, and stale generation.

## 4. Android Refloat protocol and control owner

- [ ] 4.1 Add exact Android control-profile selection for audited Refloat 1.0.x, 1.1.0–1.1.2, and 1.2.0–1.2.3 packages, preserving read-only access for audited test firmware and failing closed for every other version.
- [ ] 4.2 Implement and golden-test Kotlin encoders/decoders for the allowlisted commands, route correlation, lighting capability/state, and distinct Refloat 1.2.x active-fault and fatal-latch fields.
- [ ] 4.3 Add the Android application-service Refloat control owner and sole extensible connection-scoped `RefloatMutationCoordinator` with current-generation authorization, idle/mode/fault gates, idempotence, one pending action, three-second confirmation, lifecycle cancellation, no parallel write path, and rejection of every flywheel-mode mutation except eligible flywheel disable.
- [ ] 4.4 Submit Android control writes, one-way confirmation telemetry, and one-second best-effort lighting refresh through the shared scheduler with riding telemetry priority and no catch-up queue.
- [ ] 4.5 Implement Android loopback handlers and authoritative snapshots for both semantic methods, including bounded diagnostics and exact error mapping.
- [ ] 4.6 Add Android unit/integration tests for direct and CAN routes, every release-family codec, rejected safety states including every prohibited flywheel-mode write, the sole command-22 disable exception, malformed/late responses, timeout, disconnect, and stale generation.

## 5. Simulator and canonical conformance

- [ ] 5.1 Replace the fixture's blanket Refloat-write rejection with a strict version/route-aware allowlist and a disposable normalized state model for package, flywheel, LEDs, headlights, active fault, and fatal latch.
- [ ] 5.2 Implement exact legacy and stable lighting responses plus later telemetry confirmation for one-way commands in direct and CAN-routed simulator scenarios, without any motor-motion path.
- [ ] 5.3 Retain rejection and bounded diagnostics for test-version writes, malformed payloads, every mutation while flywheel is active except eligible flywheel disable, and every tuning, profile, reverse-stop, simple-start, recording, history, Handtest, remote, RC-move, arbitrary-config, or motor-control command.
- [ ] 5.4 Extend canonical mock/Apple/Android conformance for both methods and every `RefloatControls` field across representative 1.0.0, 1.1.0, and 1.2.3 direct and CAN scenarios.
- [ ] 5.5 Add conformance matrices for normal/flywheel/read-only access, the flywheel-mode no-write rule and sole disable exception, idle/riding safety, absent capability, active fatal fault, idempotence, competing actions, response bounds, timeout, reconnect, stale generation, guest/saved tracking, and frontend recreation.
- [ ] 5.6 Add the physical-fixture direct and CAN control checkpoints for each native backend while preserving the explicit no-motor-movement assertion.

## 6. `/live` connected-board controls

- [ ] 6.1 Add state-labelled package, flywheel, LED, and headlight actions plus conditional Clear fatal error to the existing `/live` connected-board menu, retaining Configure board and Disconnect behavior.
- [ ] 6.2 Gate visibility and enablement from authoritative readiness, writability, motion, mode, lighting capability, fatal state, and pending action without frontend protocol/version inference; while flywheel is active, expose only eligible flywheel disable among mutation actions.
- [ ] 6.3 Reuse the existing confirmation component for package disablement and flywheel enablement with the specified balancing and disengaged nose-up warnings.
- [ ] 6.4 Implement primary Pointer Events, cancellation, guarded keyboard/assistive click activation, pending-state disabling, and focus restoration without duplicate backend requests.
- [ ] 6.5 Add frontend component and route tests for ready, read-only, reconnecting, riding, lighting-unavailable, clearable-fatal, confirmation, non-primary pointer, keyboard, pending, error, and recovery states.

## 7. Documentation and verification

- [ ] 7.1 Update retained frontend/backend/conformance documentation to describe the semantic Refloat control ownership, exact allowlist, live-only state, and exclusion of Tunes/profiles, reverse stop, and simple start.
- [ ] 7.2 Run the focused frontend, Apple, Android, protocol-fixture, simulator, and canonical conformance test targets and resolve all failures.
- [ ] 7.3 Run `just verify` and record the separate physical BLE checkpoint as required before declaring the change implementation complete.
