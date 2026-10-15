# Tasks

## 0. Unify load ownership and all consumers

- [x] 0.1 Audit the three formula owners and add one pure shared Rust component/winner calculation below application-runtime and backend-service; delete the duplicate Rust formulas and verify focused unit tests for filtered/directional/raw preference, draw/regen limits, missing/non-finite inputs, saturation and tie ordering.
- [x] 0.2 Extend the live sample contract, client/mock and Rust projection with authoritative load components/winner scoped to sample sequence and controller generation; verify saved persistence-before-publication, guest isolation, controller replacement, nullability and complete monotonic snapshots through affected runtime/contract/conformance tests.
- [x] 0.3 Replace capture-time motorLoad/batteryLoad formulas with the shared owner while retaining raw dutyCycle and existing recorded fields. Verify existing session query/prune/export/import round trips and historical stability after current-limit changes; do not introduce dutyLoad, change historical chart field selection or reset session storage solely for this refactor.
- [x] 0.4 Switch primary gauge, load widgets and live charts to projected values; remove frontend utilisation and winner logic and current-limit reconstruction in chart buffers. Preserve session review of raw dutyCycle and captured current-load fields. Verify frontend type checks and focused gauge/widget/chart/session/mock tests, including a retained point whose controller limits later change.
- [x] 0.5 Feed Android notification, iOS Live Activity/Dynamic Island and announcement eligibility from the shared Rust result; verify cross-consumer fixtures assert matching source and unrounded values for identical samples, without requiring identical presentation cadence. Update frontend/core/host ownership documentation and canonical conformance for the new live load contract and unchanged recorded field semantics.

## 1. Contract, storage and bounded OS exchange

- [x] 1.1 Extend contract.ts, client/mock, shared settings persistence and complete state projection with validated announcement settings (independent enable flags, thresholds, shared interval and 0–100 volume and lowerOtherAudio) and audio readiness/loweringControl; verify defaults, bounds, atomic nested replacement, revision conflicts, persistence failures, unit conversion, unsupported false lowering requests failing atomically, and process recreation with focused store/contract/mock tests.
- [x] 1.2 Add typed operation-identified speak/cancel effects and terminal/readiness facts through the existing C/JNI families, with one slot, bounded payloads and ten-second deadlines; verify Apple/Android decoding and binding consumer tests reject malformed effects and ignore duplicate/late outcomes.
- [x] 1.3 Add authenticated announcements.test dispatch and bounded cancellable waiter handling across hosts and mock, initially returning unavailable where native speech is not yet connected; verify fixed speed, single-winner load and combined phrases, saved speech gain and silent zero-volume completion, invalid input, busy/cooldown errors, disconnect cleanup and exactly one terminal response in canonical conformance. Document new contract shapes beside their owning code.

## 2. Shared Rust announcement policy

- [x] 2.1 Implement the coordinator against normalized board telemetry and monotonic time, independent of publication/persistence cadence; verify saved/guest eligibility, reverse magnitude, highest load selection and duty/motor/battery tie order, current-limit selection, unavailable components, rounding, bounds, equality, missing readings and two-second freshness using deterministic tests and consumer fixtures against the single Rust load owner, including saturation and reverse/regenerative current limits.
- [x] 2.2 Implement immediate eligible attempts, repeat timing, shared test/automatic admission and skipped busy/unavailable attempts; verify only the highest qualifying load component plus qualifying speed share one utterance and cooldown with load first, both enable flags work together, newly qualifying rules wait for shared cooldown, load-winner changes do not interrupt while still eligible, unchanged excess repeats, threshold chatter, settings edits, failed audio, delayed workers and long utterances cannot overlap, bypass cooldown or create catch-up speech.
- [x] 2.3 Wire connection/controller changes, shutdown and frontend-independent lifetime into cancellation; verify reconnect requires fresh readings, cooldown survives same-process reconnect, restart restores only settings, and obsolete callbacks cannot affect replacement speech. Document policy ownership and deadlines in the core backend documentation.

## 3. Apple native speech

- [ ] 3.1 Add the service-owned Apple synthesizer with offline voice selection, localized combined speed/load phrases, per-utterance volume and closed readiness; verify adapter tests cover initialization, cancellation, duplicate callbacks, route/interruption/reset events and watchdog resource cleanup on iOS/macOS boundaries.
- [ ] 3.2 Add iOS background-audio capability and explicit playback/voicePrompt session ownership, choosing duck/spoken-interruption options when lowering is on and mix-only options when off; verify both preference values, next-utterance changes, test parity, and music restoration on success, stop, error and interruption. Document the chosen session ownership and call limitation in the Apple README.
- [x] 3.3 Connect speech effects/facts across service and listener recreation; verify linked Apple consumer tests and affected iOS/macOS build checks, with no iOS-only audio-session calls in macOS builds.

## 4. Android native speech

- [ ] 4.1 Add service-owned asynchronous TextToSpeech initialization, installed offline voice selection, per-utterance volume, speech/guidance attributes and operation-specific callbacks; verify focused JVM tests for no engine/voice, initialization failure, cancellation, readiness recovery and late callbacks.
- [ ] 4.2 Request transient may-duck focus per utterance without delayed acquisition, release focus on every terminal path and enforce watchdog cancellation; verify fake focus/TTS tests cover focus denial/loss, error, stop, timeout and suppression of obsolete callbacks.
- [x] 4.3 Integrate existing foreground-service lifetime, review current service-type/permission requirements for actual TTS playback and add only required declarations; verify affected Android service tests/build checks and document lifetime, audio ownership and best-effort calls in the Android README.

## 5. Shared Settings UI

- [x] 5.1 Add independent speed/load switches and threshold inputs, shared interval input, announcement-volume slider with muted feedback, bounded readiness, iOS-only “Lower other audio while speaking” with helper text driven by loweringControl, and explicit test action in the existing settings feature; verify frontend tests and npm run check cover units, revisions, preserved edits, invalid input, save failures, both enabled together, volume boundaries, iOS switch visibility/persistence, Android/macOS switch absence without platform detection, and speed/load/combined test outcomes without board connection.
- [x] 5.2 Use native button semantics and established pointer/keyboard activation guards; verify primary pointer, non-primary pointer, cancellation and keyboard activation in relevant mock/browser coverage, including no duplicate tests and pending-state reactivation. Update frontend ownership/product documentation where needed.
- [x] 5.3 Add a bullet to the root README.md feature list describing spoken speed/load announcements with configurable thresholds and repeat intervals; verify the wording matches implemented behavior, does not promise playback during calls, and passes git diff --check.

## 6. Integration acceptance

- [x] 6.1 Run affected Rust crate/consumer tests, formatting and warnings-denied Clippy, relevant frontend checks, linked Apple/Android tests and affected canonical direct/host conformance cases; verify contract parity, bounded resources, frontend recreation and audio failure without BLE or recording regression. Record actual commands and results.
- [ ] 6.2 On iOS and Android hardware, verify above/below-threshold speech, repeats, background/locked-screen BLE delivery, speech volume including zero without ducking, simultaneous speed/load phrases, iOS lowering on/off for automatic and test speech, Android automatic ducking, music restoration, Bluetooth headset routing/disconnection, interruption, unavailable audio recovery and disabling; check macOS speech/mixing where affected. Probe representative calls as best effort, recording graceful skipping as acceptable. Preserve fixture safety/restore checks for any hardware fixture runs and report unavailable required coverage.
- [ ] 6.3 Review implementation against all delta scenarios, confirm no duplicate load formulas or frontend winner selection, no speech backlog, alternate runtime, browser speech, new polling or compatibility machinery remains, and record remaining limitations before declaring implementation complete.

## Workflow follow-up

- Commit the validated planning artifacts before implementation; keep coherent implementation commits and task updates together.
- After relevant verification, sync delta specs and archive the change together in one final commit, preserving actual software and hardware evidence.

## Implementation and verification evidence (2026-10-15)

All feature code is present, including Apple/Android speech executors. Unchecked
native tasks retain their full original acceptance criteria; they are not claims
that native engine/focus/background behavior was established by software tests.

- Shared load ownership, capture-time semantics, settings validation/reopening,
  speech combination/cooldown/freshness, backpressure, timeout-before-late-result,
  client cancellation and stale callback regression checks pass. Affected Rust
  runtime/backend/store suites and C/JNI consumer tests pass. Warnings-denied
  Clippy passes for application-runtime, product-store, backend-service and
  loopback-server with all targets; Rust formatting passes.
- Frontend type checks and all 378 Vitest tests pass. The existing BMS view test
  expected obsolete warning copy; its assertion now matches the unchanged UI.
  Production frontend build and changed-file ESLint/Prettier checks pass.
- Canonical `cd conformance && npm run direct` and `npm run mock` pass with the
  new settings/test method, audio availability, atomic failures and live load
  projection. `npm run check` passes. Built-UI announcement tests in Chromium
  and WebKit cover pointer/keyboard activation, cancellation, combined enable
  flags, muted tests, duplicate suppression, immediate Save activation, rejected-save
  edit retention and conditional iOS control visibility.
- Apple: focused speech request/resource-lease, Live Activity and linked shared
  server tests pass; the iOS speech adapter type-checks. The arm64 iOS simulator
  app builds with `ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO`.
  The generic simulator build requested an unavailable Intel Rust target; the
  focused arm64 build avoids that unrelated architecture. An initial broad Swift
  run also encountered a macOS effective-appearance test failure; the affected
  speech/shared-server/Live Activity selection passes with normal native access.
- Android SDK is now installed at `~/Library/Android/sdk` with API 36 (revision
  2), Build Tools 35.0.0, NDK 28.2.13676358 and Platform Tools 37.0.1, using the
  existing Homebrew JDK 21 and all four repository-pinned Rust targets. The SDK
  path is configured in ignored `backends/android/local.properties`.
- Android: the initial `./gradlew clean assembleDebug assembleRelease
assembleConformance test lint assembleDebugAndroidTest` run exposed Kotlin/API
  compilation failures and existing lint errors. After fixes,
  `./gradlew assembleDebug assembleRelease assembleConformance test lintDebug
lintRelease lintConformance assembleDebugAndroidTest` passes. Debug, Release and Conformance APKs
  build; all 27 JVM tests pass in each variant (81 executions, no skips/failures),
  including linked JNI, request validation and resource cleanup. Each variant's
  lint has zero errors and 36 existing advisory warnings. The Rust helper's exact JNI
  symbol/architecture audits pass, and ZIP integrity, the exact four-ABI Rust
  library set and frontend assets are verified in all three APKs. The final
  aggregate `./gradlew build auditAndroidRustRuntimeTasks` also passes.
- Full Android compilation exposed speech request/focus variable shadowing and
  use of JVM-only `JSONObject.keySet()`; these now use distinct names and the
  Android-supported key iterator. The manifest declares TTS engine visibility
  for Android 11+. Existing location registration now explicitly catches
  permission revocation and provider removal, allowing lint to establish the
  intended graceful failure behavior. Conformance's API-33-only `readNBytes`
  is replaced by a size/deadline-bounded API-29-supported read loop.
- Android emulator 37.2.12 with API 36 Google APIs ARM64: all three
  `./gradlew connectedDebugAndroidTest` tests pass (JNI envelope/lifetime plus
  native loading canvas, retained/disposed resume, memory pressure and Activity
  recreation). Canonical `cd conformance && npm run android` passes at revision
  489, including announcement settings/test behavior and frontend recreation.
  The software target now pre-grants location/notification access rather than
  waiting for interactive OS dialogs. The UI checkpoint assertion tolerates
  independent native TTS readiness revisions while asserting all other state is
  unchanged; Rust's deterministic checkpoint test retains the no-event assertion.
  Direct and mock conformance reruns pass, as do `npm run check` and all eight
  `node --strip-types --test test/targets.test.ts` checks.
- The new required persisted announcement settings use product storage marker
  23; earlier development storage is reset only by the existing explicit version
  mismatch path. Session field representation and review behavior are unchanged.

Remaining acceptance: native fake-engine/focus coverage in tasks 3.1, 4.1 and
4.2 remains incomplete; the passing request/lease and shared-policy tests must
not be described as full adapter fault-injection coverage. Perform the exact
[physical checklist](physical-checks.md), including native voice recovery,
interruptions, iOS lowering on/off, headset routes, locked-screen BLE/speech and
music restoration. At this 2026-10-15 checkpoint, no physical audio/BLE validation
had been performed; see the Android follow-up below for subsequent evidence.
Keep this change active and do not sync/archive until these checkpoints and the final
scenario audit are complete.

## Android physical checkpoint follow-up (2026-10-16)

Core Android hardware observations now pass on a Samsung Galaxy S9+ / Android 10
with the ESP32 fixture and production BLE adapter. See the
[retained physical evidence](../../../conformance/physical-checkpoints/2026-10-16-android-announcements.md)
for actual values, manual listening confirmations, cleanup and remaining scope.
The run exposed Samsung's ISO 639-2 `eng` voice identifiers; the selector now
accepts both `en` and `eng`, with regression tests. All 31 JVM tests pass in each
variant, all builds/lint pass, and all four instrumentation tests pass on the
phone. The new interactive physical harness is type-checked, and the eight host
target tests pass. A matching signed physical APK can be supplied explicitly
without overwriting Gradle outputs or discarding installed app data.

Task 6.2 remains unchecked because iOS/macOS and the listed remaining native
audio cases have not been exercised. Do not interpret the earlier “No physical
validation” note as current status; it describes the 2026-10-15 checkpoint.
Native fake-engine/focus tests and the final scenario audit remain outstanding.
