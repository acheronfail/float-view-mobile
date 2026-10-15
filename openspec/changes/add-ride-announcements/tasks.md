# Tasks

## 1. Contract, storage and bounded OS exchange

- [ ] 1.1 Extend contract.ts, client/mock, shared settings persistence and complete state projection with validated announcement settings (independent enable flags, thresholds, shared interval and 0–100 volume) and audio readiness; verify defaults, bounds, atomic nested replacement, revision conflicts, persistence failures, unit conversion and process recreation with focused store/contract/mock tests.
- [ ] 1.2 Add typed operation-identified speak/cancel effects and terminal/readiness facts through the existing C/JNI families, with one slot, bounded payloads and ten-second deadlines; verify Apple/Android decoding and binding consumer tests reject malformed effects and ignore duplicate/late outcomes.
- [ ] 1.3 Add authenticated announcements.test dispatch and bounded cancellable waiter handling across hosts and mock, initially returning unavailable where native speech is not yet connected; verify fixed speed, three-component load and combined phrases, saved speech gain and silent zero-volume completion, invalid input, busy/cooldown errors, disconnect cleanup and exactly one terminal response in canonical conformance. Document new contract shapes beside their owning code.

## 2. Shared Rust announcement policy

- [ ] 2.1 Implement the coordinator against normalized board telemetry and monotonic time, independent of publication/persistence cadence; verify saved/guest eligibility, reverse magnitude, all qualifying load components, current-limit selection, unavailable components, rounding, bounds, equality, missing readings and two-second freshness using deterministic tests and parity fixtures against existing gauge component semantics, including saturation and reverse/regenerative current limits.
- [ ] 2.2 Implement immediate eligible attempts, repeat timing, shared test/automatic admission and skipped busy/unavailable attempts; verify all and only qualifying duty/motor/battery components plus qualifying speed share one utterance and cooldown with load first, both enable flags work together, newly qualifying rules wait for shared cooldown, component-set changes do not interrupt while still eligible, unchanged excess repeats, threshold chatter, settings edits, failed audio, delayed workers and long utterances cannot overlap, bypass cooldown or create catch-up speech.
- [ ] 2.3 Wire connection/controller changes, shutdown and frontend-independent lifetime into cancellation; verify reconnect requires fresh readings, cooldown survives same-process reconnect, restart restores only settings, and obsolete callbacks cannot affect replacement speech. Document policy ownership and deadlines in the core backend documentation.

## 3. Apple native speech

- [ ] 3.1 Add the service-owned Apple synthesizer with offline voice selection, localized combined speed/load phrases, per-utterance volume and closed readiness; verify adapter tests cover initialization, cancellation, duplicate callbacks, route/interruption/reset events and watchdog resource cleanup on iOS/macOS boundaries.
- [ ] 3.2 Add iOS background-audio capability and select one audio-session approach after a focused device experiment; verify music ducking and restoration on success, stop, error and interruption, and retain only the selected approach. Document the chosen session ownership and call limitation in the Apple README.
- [ ] 3.3 Connect speech effects/facts across service and listener recreation; verify linked Apple consumer tests and affected iOS/macOS build checks, with no iOS-only audio-session calls in macOS builds.

## 4. Android native speech

- [ ] 4.1 Add service-owned asynchronous TextToSpeech initialization, installed offline voice selection, per-utterance volume, speech/guidance attributes and operation-specific callbacks; verify focused JVM tests for no engine/voice, initialization failure, cancellation, readiness recovery and late callbacks.
- [ ] 4.2 Request transient may-duck focus per utterance without delayed acquisition, release focus on every terminal path and enforce watchdog cancellation; verify fake focus/TTS tests cover focus denial/loss, error, stop, timeout and suppression of obsolete callbacks.
- [ ] 4.3 Integrate existing foreground-service lifetime, review current service-type/permission requirements for actual TTS playback and add only required declarations; verify affected Android service tests/build checks and document lifetime, audio ownership and best-effort calls in the Android README.

## 5. Shared Settings UI

- [ ] 5.1 Add independent speed/load switches and threshold inputs, shared interval input, announcement-volume slider with muted feedback, bounded readiness and explicit test action in the existing settings feature; verify frontend tests and npm run check cover units, revisions, preserved edits, invalid input, save failures, both enabled together, volume boundaries and speed/load/combined test outcomes without board connection.
- [ ] 5.2 Use native button semantics and established pointer/keyboard activation guards; verify primary pointer, non-primary pointer, cancellation and keyboard activation in relevant mock/browser coverage, including no duplicate tests and pending-state reactivation. Update frontend ownership/product documentation where needed.
- [ ] 5.3 Add a bullet to the root README.md feature list describing spoken speed/load announcements with configurable thresholds and repeat intervals; verify the wording matches implemented behavior, does not promise playback during calls, and passes git diff --check.

## 6. Integration acceptance

- [ ] 6.1 Run affected Rust crate/consumer tests, formatting and warnings-denied Clippy, relevant frontend checks, linked Apple/Android tests and affected canonical direct/host conformance cases; verify contract parity, bounded resources, frontend recreation and audio failure without BLE or recording regression. Record actual commands and results.
- [ ] 6.2 On iOS and Android hardware, verify above/below-threshold speech, repeats, background/locked-screen BLE delivery, speech volume including zero without ducking, simultaneous speed/load phrases, music ducking/restoration, Bluetooth headset routing/disconnection, interruption, unavailable audio recovery and disabling; check macOS speech/mixing where affected. Probe representative calls as best effort, recording graceful skipping as acceptable. Preserve fixture safety/restore checks for any hardware fixture runs and report unavailable required coverage.
- [ ] 6.3 Review implementation against all delta scenarios, confirm no speech backlog, alternate runtime, browser speech, new polling or compatibility machinery remains, and record remaining limitations before declaring implementation complete.

## Workflow follow-up

- Commit the validated planning artifacts before implementation; keep coherent implementation commits and task updates together.
- After relevant verification, sync delta specs and archive the change together in one final commit, preserving actual software and hardware evidence.
