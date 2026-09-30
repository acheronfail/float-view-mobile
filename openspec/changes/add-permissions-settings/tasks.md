## 1. Contract and shared permission policy

- [x] 1.1 Define bounded permission inventory, access details, guidance, pending state, and permissions.recover in contract.ts; update client/mock and handwritten shared contract models in a buildable slice; verify type checks and fixture decoding cover each platform inventory and reject unknown values.
- [x] 1.2 Extend existing C/JNI OS facts/effects and shared projection to separate grants, service availability, accuracy, and request eligibility; verify Rust tests for disabled recording, partial Bluetooth grants, approximate location, restrictions, unknown observations, and the Android API matrix.
- [x] 1.3 Implement authenticated recovery dispatch, expected-action validation, one presentation slot, timeout and lifetime guards; verify conflict, unavailable, invalid_request, invalid_state, failed navigation, and late-completion tests.

## 2. Native adapters and lifecycle

- [x] 2.1 Implement Apple authorization/accuracy facts, unresolved permission requests, iOS app Settings and macOS pane/manual guidance with navigation completion; verify Apple adapter tests preserve existing Always-access ride eligibility and foreground-only When In Use behavior and distinguish services disabled from denied grants.
- [x] 2.2 Implement Android grant/rationale/resolved-history facts and grouped nearby/foreground requests with separate background recovery for API 29 versus 30+; verify Android unit tests cover first request, denial with retry, blocked retry, partial grants, prerequisite failure, and absent Activity.
- [x] 2.3 Refresh OS facts on startup, authorization callbacks and activation, independent of recording or connection state; verify lifecycle tests for Settings return, unchanged access, revocation, temporary grant expiry, failed refresh, and frontend recreation without duplicate prompts.
- [x] 2.4 Route existing Bluetooth/location permission entry points through the shared policy and remove superseded denial-only and single requested-flag routing; verify existing scan, recording-toggle and live-warning tests plus concurrent old/new entry-point coverage.

## 3. Settings interface

- [x] 3.1 Add Permissions rows with purpose, text status, access detail, optional-feature context, guidance and advertised action; verify mock/component coverage for each platform inventory with recording both enabled and disabled, and ensure no permission interaction changes preferences or starts scanning/capture.
- [x] 3.2 Add pending/disconnected/error presentation and pointer/keyboard activation handling; verify browser tests for primary activation, non-primary rejection, cancellation, keyboard/assistive clicks, duplicate-click suppression, stale actions, and Settings return updates.

## 4. Integration and acceptance

- [x] 4.1 Extend canonical conformance cases for inventory completeness, authenticated recovery, bounds, monotonic snapshots, reconstruction and failures across affected host integrations; run affected frontend checks, Rust crate/consumer tests, Apple and Android tests/build checks and conformance cases, recording actual results.
- [ ] 4.2 Perform focused physical iOS, macOS and Android checks for permission presentation, denial recovery, settings routing and activation refresh; cover Android pre-31 and 31+ groups and API 29 background prompt versus 30+ Settings as available, explicitly recording unavailable required coverage rather than claiming mock results as physical validation.
- [ ] 4.3 Review delivered behavior against delta scenarios, validate OpenSpec, then sync specs and archive together after acceptance; verify the final diff and retain verification evidence without running unrelated device/firmware matrices.

## Verification evidence

See [the implementation and physical checkpoint record](../../../conformance/physical-checkpoints/2026-09-30-permissions-settings.md). Tasks 4.2 and 4.3 remain open for macOS interactive acceptance (desktop locked), followed by spec sync and archive. Unavailable Android 30+/31+ physical coverage is explicitly recorded.
