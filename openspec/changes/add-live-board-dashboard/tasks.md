## 1. Contract and Current Schemas

- [ ] 1.1 Extend `frontend/src/lib/backend/contract.ts` with the explicit live sample, active saved-device/session state, saved-device identity revision and update method, and metric/imperial settings preference.
- [ ] 1.2 Replace the catalog's hard-coded display-name default with revisioned nullable custom-name storage, retain advertised and platform identifiers independently, and bump the disposable storage version without adding a migration.
- [ ] 1.3 Add persisted board state to the shared session sample schema and update the canonical diagnostic/series field definitions where that field is exposed.

## 2. Apple Backend

- [ ] 2.1 Update Apple domain and SQLite records to persist board state, resolve custom/advertised/platform board identity, update custom names revision-safely, and round-trip `unitPreference`.
- [ ] 2.2 Retain the latest sample only after a selected append succeeds, associate it and the current session summary with the active connection, preserve it through reconnect, and clear it when the connection is removed or replaced.
- [ ] 2.3 Extend the Apple loopback request validation and snapshot serialization for `savedDevice.update`, resolved identity, active live state, and complete settings.
- [ ] 2.4 Make Apple sample-driven `state.changed` delivery follow persistence cadence and conflate pending delivery to the newest snapshot, with focused storage, application-service, and loopback tests.

## 3. Android Backend

- [ ] 3.1 Update Android domain and SQLite records to persist board state, resolve custom/advertised/platform board identity, update custom names revision-safely, and round-trip `unitPreference`.
- [ ] 3.2 Retain the latest sample only after a selected append succeeds, associate it and the current session summary with the active connection, preserve it through reconnect, and clear it when the connection is removed or replaced.
- [ ] 3.3 Extend the Android loopback request validation and snapshot serialization for `savedDevice.update`, resolved identity, active live state, and complete settings.
- [ ] 3.4 Make Android sample-driven `state.changed` delivery follow persistence cadence and replace older pending delivery with the newest snapshot, with focused storage, application-service, and loopback tests.

## 4. Mock and Cross-Platform Conformance

- [ ] 4.1 Extend the deterministic frontend mock with representative live samples, cadence-driven active-session aggregates, board-name fallback and updates, reconnect retention, and unit settings.
- [ ] 4.2 Update canonical contract fixtures and assertions for the new mandatory method, live sample shape and ordering, saved-device identity semantics, default/updated units, and invalid revisions or preferences.
- [ ] 4.3 Add bounded cadence/conflation coverage proving skipped probes do not publish, persisted samples do publish, slow clients converge on the newest revision, and durable sample counts are unaffected.

## 5. Shared Board Configuration

- [ ] 5.1 Add backend-state operations for saved-device custom-name mutation and reconcile returned saved-device/profile revisions into authoritative frontend state.
- [ ] 5.2 Extract one board-owned configuration editor and a colocated serial auto-save controller that commits text on blur, other controls on change, validates fields independently, and flushes queued mutations before dismissal.
- [ ] 5.3 Replace the Boards route form with the shared editor, showing resolved/custom names and the selectable read-only platform Bluetooth identifier, and update board-list naming tests.
- [ ] 5.4 Add focused editor tests for progressive incomplete profiles, clearing the custom-name override, rapid ordered changes, invalid transient values, and flush-on-close behavior.

## 6. Units and Live Dashboard

- [ ] 6.1 Add pure shared measurement formatters for metric and imperial speed, distance, and temperature while preserving voltage, current, duty, time, counts, and eRPM semantics; cover conversions and null formatting with unit tests.
- [ ] 6.2 Add the persisted metric/imperial control to Settings and apply the formatters consistently to existing session list/detail measurements without changing fetched or exported values.
- [ ] 6.3 Replace the placeholder live view with the responsive fixed-position telemetry and active-session grids, stable placeholders, resolved board header, unsupported-firmware warning, and dimmed retained readings for every non-ready active state.
- [ ] 6.4 Add the accessible live header menu and near-full-height modal bottom sheet, render the shared board editor, support close control/Escape/backdrop/focus restoration, and keep configuration available during reconnect.
- [ ] 6.5 Move disconnect into the menu and require confirmation for every latest ride state except exactly `idle`, including missing telemetry, while preserving the authoritative return-to-Home flow.
- [ ] 6.6 Add focused live-view tests for first-sample placeholders, responsive content completeness, persisted updates, reconnect dimming, unsupported firmware, menu availability, modal dismissal, and conditional confirmation.

## 7. Verification

- [ ] 7.1 Run frontend formatting, type checks, and focused unit tests for backend state, board configuration, units, navigation, and the live view.
- [ ] 7.2 Run focused Apple and Android storage, application-service, protocol, and loopback tests covering the changed contract and schemas.
- [ ] 7.3 Run the canonical mock conformance suite and verify the physical BLE checkpoint remains the only hardware-required gate.
- [ ] 7.4 Run `just verify` as the complete software gate.
