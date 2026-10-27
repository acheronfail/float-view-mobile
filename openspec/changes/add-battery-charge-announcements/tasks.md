# Tasks

## 1. Contract and compatible preference storage

- [x] 1.1 Add default-on `batteryChargeEnabled` to the hand-written contract, shared settings/defaults, complete update validation and mock/client consumers. Verify valid on/off updates, invalid-present rejection and revision conflicts with focused settings/contract tests.
- [x] 1.2 Add backward-compatible stored-field decoding and persistence tests using supported settings with the field absent, customized existing values and explicit false. Verify save/reopen preserves preferences and recordings, keeps frozen baseline fixtures unchanged and requires no storage marker/archive change.

## 2. Authoritative threshold resolution

- [x] 2.1 Audit supported vendored Refloat/firmware families for pack/per-cell semantics, document the mapping beside the resolver, and implement normalization using current configuration plus controller series count. Verify source-derived family cases, 60/86 V equivalence with 3.0/4.3 V at 20S, missing count, unknown semantics, invalid pairs and non-finite derived values.
- [x] 2.2 Integrate resolved thresholds with existing connection/configuration facts without another reader/runtime or continuous polling. Verify reconnect/controller replacement, read failure, write/readback and explicit refresh invalidate/rebuild battery eligibility, while drafts/defaults and saved app cell counts never supply limits.

## 3. Shared voltage announcement policy

- [x] 3.1 Add the proportional 10%/5% entry and 2% recovery boundaries, fresh Refloat running-state gate without a speed check and regen-only advance high eligibility. Verify running at zero/low/reverse/unavailable speed, suppression for non-running/unknown/missing state even when moving, exact/equality voltage boundaries, unchanged display-unit behavior, the agreed 62.6/84.7 V example and scale invariance in shared policy tests.
- [x] 3.2 Implement low 2-second/high 500-ms confirmation, critical-boundary entry bypass, 2-second recovery and freshness/reset handling. Verify brief sag/spikes, chatter, hysteresis bands, missing inputs, loss of regen, leaving/re-entering running state, two-second gaps and settings edits using monotonic test time; document constants and input semantics beside the policy.
- [x] 3.3 Add independent 60-second per-kind cooldowns and standalone battery priority through the existing single speech slot. Verify low repeat boundaries, high independent eligibility, routine cooldown advancement, non-preemption, busy-drop re-evaluation, mute/unavailable attempt accounting and no backlog/catch-up.
- [x] 3.4 Verify toggle disable, configuration invalidation, stale context, disconnect and late callbacks cancel only as specified; fresh exit from running state or voltage recovery lets admitted speech finish. Add policy regressions for existing footpad/load/speed/manual playback, WebView replacement and retained process-local cooldowns.

## 4. Native speech representation and playback

- [x] 4.1 Extend the existing closed speech effect/validated C/JNI representations and both host formatters for standalone “Battery low” and “Over voltage”; remove any superseded representation rather than adding parallel dispatch. Verify Apple/Android adapter and affected ABI tests cover exact wording, unknown-code rejection, volume, deadline and completion isolation.
- [x] 4.2 Document reuse of existing audio/background behavior and verify affected native host builds/tests with the new speech payload, preserving existing voice selection and mixing/ducking preferences without new OS modes or platform voltage policy.

## 5. Settings interaction

- [x] 5.1 Add Battery charge beside existing announcement switches, with concise helper text describing board-derived alerts and one-minute repeats, through existing autosave/revision handling. Verify default-on rendering, off/on persistence, delayed-save/conflict/error behavior and no changes to other rule values in frontend/mock tests; update `frontend/src/lib/README.md`.
- [x] 5.2 Add focused browser coverage for primary touch/mouse, keyboard and assistive click, non-primary/cancelled input and duplicate-activation guards. Build the frontend first, then run the affected `conformance/test/*.browser.ts` outside the macOS sandbox as documented.

## 6. Integration and acceptance

- [x] 6.1 Extend canonical conformance with configured pack/per-cell fixtures and controllable voltage/current/riding-state sequences, varying speed independently. Verify default-on/explicit-off contract round trips, both phrases, non-running suppression and running eligibility without speed evidence, regen gating, invalid limits, cooldown and reconnect behavior through affected Apple/Android integrations.
- [x] 6.2 Replay representative recorded low-battery/regen traces when available, alongside deterministic synthetic traces, and report warning timing/chatter against the agreed constants. Verify examples and document any real-trace coverage limits without presenting synthetic results as physical validation or silently changing the margins.
- [x] 6.3 Run focused affected Rust crate/consumer tests and lint, frontend type/lint/test/build checks, native speech/boundary checks and canonical announcement/browser cases. Run strict OpenSpec validation and diff checks; record actual commands/results and unresolved coverage. Broaden only for failures or demonstrated cross-cutting impact.
- [ ] 6.4 Perform a scoped physical audio checkpoint on affected native hosts using safe fixture telemetry for running/non-running low/high phrases and foreground/background/lock behavior where supported. Keep device inventories/raw evidence under ignored `.local/physical-testing/`, publish only brief source-linked outcomes/limits, and report unavailable hardware explicitly; never ride a real pack into voltage extremes for this check.

## Verification notes

- Shared backend-service library: 122 tests passed, including source-family threshold resolution, independent cooldowns, hysteresis, actual configuration bootstrap, battery priority, busy admission, muted attempts, missing voltage cancellation and running-state transitions.
- Product-store announcement compatibility tests and invalid-present battery-field preservation test passed; supported settings default the absent field on and round-trip explicit off without changing frozen fixtures.
- Loopback-server library: 4 tests passed. Existing Apple FFI/Android JNI Rust boundary tests: 9 tests passed, including ABI integration.
- Frontend: 553 tests passed; Svelte/type checks, formatting/lint and production build passed. Conformance TypeScript check passed. Chromium and WebKit announcement interaction cases both passed after the frontend build.
- Canonical direct announcement cases passed (existing footpad regression plus legacy pack and modern per-cell battery wire-to-speech coverage). The two representations produce the same configured 20S headroom. Source-family normalization is also covered in Rust. Automatic host replay subsequently passed all four pack/per-cell cases through the macOS and Android hosts, including exact native phrases and a full one-minute interval on each host.
- Apple native speech: 14 request/adapter tests passed; the rebuilt macOS announcement conformance scenario passed (`revision=5`). Android validation subsequently passed using installed Homebrew Java 21: all 56 JVM tests (including 18 speech tests), Debug lint, the conformance APK build and four focused emulator voice/engine instrumentation tests. Android announcement conformance passed (`revision=7`). Android Studio’s bundled Java 25 failed with the pinned Gradle, so command-line checks explicitly selected Java 21.
- Synthetic voltage sequences verified the agreed 62.6 V / 84.7 V example, proportional recovery, short spikes, sustained crossings, recovery interruption and independent one-minute repeats. No representative recorded low-battery/regen traces were available locally; these results are software checks, not validation of real pack response or advance notice duration.
- Physical audio/background/lock verification has not been performed. The user will perform task 6.4 after the production commit/build; it remains open. Software replay replaces OS playback and is not physical audio evidence. No real pack was driven toward voltage extremes.

- Follow-up Android integration found and fixed the shared host playback flag omitting battery/footpad-only configurations; a focused regression verifies each rule and zero-volume suppression. This reuses the existing foreground-service playback mode. Shared library tests and warnings-denied lint passed after the fix. Tasks 4.2 and 6.3 are complete. Task 6.1 is now also complete: the opt-in native replay drives the real host and shared service to the decoded native speech-driver boundary.

- Native replay commands: `FLOAT_VIEW_NATIVE_REPLAY_HOST=macos node --strip-types --test conformance/test/announcements.native.ts` and the same command with `android` (Java 21/SDK environment and emulator). Both pack/per-cell cases passed per host, covering invalid limits, running state at zero speed, low confirmation, regen gating, exact phrases, independent cooldowns, one-minute repeats and off/re-enable across reconnect. Production audio remains unchanged; Apple normal speech tests also passed (14 tests), and fixture regressions passed (29 tests). See `docs/development.md` for the repeatable procedure and limits.

- Final replay regression checks passed: normal Apple speech tests (14), Android JVM tests (56), Release Kotlin compilation, Debug/Conformance lint, conformance TypeScript checks, fixture tests (29), strict OpenSpec validation and diff checks. The only unchecked task is the user-owned physical checkpoint, 6.4.

## 7. Live battery widget warnings

- [x] 7.1 Project the shared confirmed battery warning in complete snapshots independently of speech enable/readiness/cooldown; update contract/mock and verify fresh context, recovery, stopped/stale suppression and voice-off behavior.
- [x] 7.2 Add amber warning labels/voltage and a three-second full-track outline to standard/custom battery widgets, with high-voltage hatching, charging precedence, unchanged charge fill and reduced-motion support.
- [x] 7.3 Run affected shared/contract/frontend/browser checks and generate reviewable normal/charging/low/high widget previews.

- Battery-widget extension verification: shared service library 123 tests, loopback library 4 tests and Apple/Android binding libraries 4 tests passed; shared service/server warnings-denied Clippy passed. Canonical pack/per-cell battery and footpad cases passed (3). Frontend types, lint, build and unit tests passed (558). Chromium/WebKit battery cases passed for standard and compact/medium/large custom cards, warning precedence, preserved charge, visual recovery, dark mode and reduced motion. Conformance TypeScript, strict OpenSpec and diff checks passed. Reproducible animated preview: `node --strip-types --test conformance/test/battery-warnings.browser.ts` after frontend build; output is ignored `conformance/generated/battery-warnings/index.html`. Physical task 6.4 remains user-owned and pending.
