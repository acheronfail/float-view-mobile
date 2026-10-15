# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. The process-owned Rust backend already consumes normalized telemetry and exchanges bounded OS effects/facts with Swift and Kotlin through existing C/JNI bindings. Frontend state publication and durable sample cadence are intentionally distinct from raw telemetry observation. iOS currently declares Bluetooth and location background modes; Android already maintains a connected-device foreground service during connections.

## Goals / Non-Goals

**Goals:** Keep one policy owner, one speech slot, deterministic time-based behavior, and native audio cleanup independent of WebView lifetime. Exercise automatic and manual speech through the same adapter.

**Non-Goals:** No speech in JavaScript, generated bridge, second runtime, additional BLE polling, arbitrary text endpoint, microphone capture, call injection, or system-volume manipulation. No custom Rust speech engine or new third-party dependency.

## Decisions

### Shared policy consumes normalized telemetry

Place the coordinator in backend-service with the other application policies. Observe fresh decoded board speed and load components before frontend publication throttling; do not tie speech to a session append or require recording. Keep persistence ordering for public telemetry unchanged. The coordinator is independent of transport and receives monotonic time, connection/controller generation, settings and typed native outcomes.

Eligibility uses a two-second freshness deadline and finite, bounded readings: speed magnitude through 200 km/h and load components normalized to 0–100 percent as on the gauge. Compare unrounded canonical values; format whole numbers only for speech. No ride-state gate is needed: a ready connected board with at least one qualifying enabled reading is sufficient. At most one wake deadline drives repeat, freshness and operation expiry. Native timer scheduling remains through the existing exchange; timers do not confer OS background execution rights.

The first eligible observation attempts immediately if cooldown permits. Record every admitted or due-but-skipped automatic attempt against the monotonic interval, including unavailable and busy outcomes. Keep the last attempt timestamp process-local across rule/threshold edits, reconnect and test requests; use the current interval when calculating eligibility. Sustained excess repeats even if the number is unchanged. Cancel outstanding automatic speech when no enabled reading qualifies or telemetry is missing/stale; losing an individual speed/load reading does not cancel while another enabled reading still qualifies. A missed interval creates no debt. Global cooldown and the single slot apply to test speech too, preventing test-button spam or a test/automatic overlap.

Alternative: duplicate timers in native hosts or derive triggers from WebSocket snapshots. Both introduce policy drift and dependence on presentation or persistence cadence.

### Reuse gauge component semantics

There are currently three formula owners: frontend connection/gauge.ts (used by gauge, widgets and live charts), backend-service/live_activity.rs (native presentation), and application-runtime persisted_sample (recorded motorLoad/batteryLoad). Session review already uses captured current-load fractions, but its load chart uses raw dutyCycle rather than the gauge's absolute saturated duty. Native Swift/Kotlin views already consume projected load and need no new formula.

Introduce one pure Rust drive-load module in the lowest shared layer usable by application-runtime and backend-service, with no dependency back up to product service or transport. Prefer a small module in an existing suitable crate; use a small leaf crate only if the dependency graph requires it. Its explicit inputs are telemetry and current limits, and its output is nullable duty/motor/battery fractions plus nullable winning source/fraction. All fractions are finite in [0,1]. Absolute duty saturates at 1; motor current prefers filtered, then directional, then raw; raw current sign selects positive versus negative limits; battery uses its corresponding signed current limit. Missing/zero/non-finite required inputs make only the affected component unavailable. Strict-greater winner replacement preserves duty/motor/battery ties after saturation.

Remove the formulas from TypeScript gauge.ts, native presentation projection and session capture. Keep only presentation selectors/formatters where helpful; parity tests alone are not a substitute for deleting duplicate implementations. Add a bounded load value to LiveSample containing components and winner with the existing sample sequence/time identity. Rust computes it from the inputs belonging to that sample/controller generation. Saved-mode publication still follows successful persistence; guest snapshots remain ephemeral. Native presentation and announcements may run at different cadences but must use this same pure function and correctly scoped inputs, not the throttled WebView state.

Persist the canonical component fractions at capture: add dutyLoad alongside recorded motorLoad and batteryLoad, leaving raw dutyCycle untouched. Session load charts use dutyLoad; raw duty remains a separately available telemetry field. If a historical consumer needs a winner, the shared Rust selector operates on captured fractions and exposes the result; neither JavaScript nor native views select a winner. Historical queries, export/import and pruning must preserve these derived readings and their nullability. Never recompute historical utilisation with current controller limits. No need to add a new historical winner UI to complete this refactor.

The primary gauge, load widgets and live charts consume LiveSample.load, and native notification/Live Activity projections consume the corresponding Rust result. Session review consumes captured derived numeric fields. The frontend mock supplies explicit authoritative load fixtures rather than retaining a second production formula. Include aligned fixtures across contract, storage and native/JS consumers for missing inputs, reverse/regen, filtering, saturation, ties and controller changes. Refactor frontend chart buffers so old points retain their projected load rather than later current limits.

Storage changes follow the pre-release explicit version-marker policy: reset incompatible owned storage only on a version mismatch, never on corruption/write failure. No migrations or old-format load reconstruction. Update current-format archives and canonical conformance together if their schema changes.

The load rule selects the highest available component using the gauge winner calculation, then compares it against the shared load threshold. Ties retain duty before motor before battery, including ties created by saturation at 100 percent. Each interval recomputes the winner from fresh telemetry and current controller limits; never list runner-up components. The winning source may change between intervals without resetting cooldown. A combined load/speed phrase is one utterance, one audio-focus lease and one cooldown event, still subject to the existing ten-second bound. Limits are scoped to the active controller and cannot survive controller replacement as valid inputs.

### One scheduler for both enabled rules

Replace exclusive mode selection with two enable flags. At every due attempt evaluate both enabled rules, then compose one phrase containing the qualifying load winner first and speed last. This is bounded at two readings, avoids starvation and repeated ducking, and needs no priority queue or separate per-rule deadlines. A reading newly above threshold during cooldown joins the next attempt; it does not interrupt speech or bypass the cooldown. If an utterance is still active, skip the due attempt as before. Users retain the one shared repeat interval.

### Typed speech effects and bounded lifecycle

Add closed speak/cancel effects with an operation identity, connection generation for automatic speech, an optional winning load reading (source duty/motor/battery) and optional speed reading, at most two items in load-then-speed order, plus speech gain. Swift/Kotlin render one localized utterance, naming the winning load component before its percentage. Rust selects the qualifying load winner and speed; native code does not calculate utilisation. Do not pass board identity, raw telemetry collections or arbitrary strings. A cancellation arriving before speech starts must suppress that operation; adapters must check admission and expiry before acquiring audio resources.

Maintain one active operation and no pending queue. Native started/finished/interrupted/unavailable facts carry its identity. Both coordinator and adapter enforce a ten-second deadline from admission; every terminal path stops playback as appropriate and releases only the matching operation's audio resources. Ignore duplicate or late completions. Coalesce readiness changes rather than publishing per-sample audio state. The readiness reason is bounded; a denied transient focus request need not mark the engine permanently unavailable.

Rule/threshold/interval changes, generation replacement and disconnect cancel automatic speech; a volume-only or lowering-only edit applies to the next utterance. Listener/WebView replacement cannot reset automatic policy. A disconnected test requester releases its bounded waiter and cancels only its own test. Process shutdown cancels all playback; process restart restores settings only. Late callbacks after timeout cannot deactivate a newer speech session.

### Settings and authenticated testing

Add a complete `Settings.announcements` value with `speedEnabled`, `loadEnabled`, `speedThresholdKph`, `loadThresholdPercent`, `repeatIntervalSeconds`, `volumePercent`, and `lowerOtherAudio` (default true). Defaults and bounds are specified in the ride-announcements delta. A supplied nested value is validated and replaced atomically; do not introduce ambiguous nested partial merges. Persist through the existing product store and settings revision. Unit changes convert display and input without rewriting the canonical physical threshold merely for presentation.

Add `AppState.announcementAudio` with the closed readiness and reason fields defined in the contract delta. Add mandatory `announcements.test({ kind })` with `kind: speed | load | combined`, returning `{ outcome }` within the speech deadline. The result describes the playback lifecycle, not guaranteed audibility. Test readings are fixed in the service: a load test says “motor 72 percent”, or a speed test uses 25 km/h, converted/rounded for current speed units. A combined test concatenates the fixed load and speed examples. A test works without a board or enabled rule; it never changes preferences. Busy/cooldown admission returns invalid_state before allocating a waiter. Use existing authenticated dispatch, error envelopes and cancellation machinery.

The shared settings feature owns controls, units, status and pending/error feedback. Use the existing primary-pointer and guarded keyboard activation patterns. Mock transport implements the complete schema and deterministic test outcomes; it does not call browser speech. Extend canonical conformance for request validation, revisions, persistence, state ordering and bounded native outcome simulation.

### Native synthesis and audio ownership

Apple retains AVSpeechSynthesizer in the service-owned adapter and uses an installed offline voice. On iOS use one explicitly managed AVAudioSession with category .playback and mode .voicePrompt. When lowerOtherAudio is true, use .duckOthers and .interruptSpokenAudioAndMixWithOthers; when false, use .mixWithOthers alone, so speech mixes without requesting either ducking or spoken-content interruption. Set usesApplicationAudioSession = true, activate only for the utterance, and deactivate with .notifyOthersOnDeactivation on every terminal path. Snapshot the preference at admission; edits apply to the next utterance. Reuse synthesis on macOS with its existing automatic audio behavior; do not call iOS-only AVAudioSession APIs there.

Enable iOS audio background capability for real spoken announcements, retaining BLE delivery as the source of observations. Do not play silence to keep the process alive. The synthesizer stays alive across listener replacement; the audio session does not stay active between prompts. Observe interruption/route/reset events and make cancellation release resources deterministically.

Android retains TextToSpeech in the process-owned native adapter. Initialize asynchronously, select an installed voice that does not require a network connection, set CONTENT_TYPE_SPEECH and USAGE_ASSISTANCE_NAVIGATION_GUIDANCE, and request AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK immediately before speaking. Use the same attributes for focus and playback. Do not request delayed focus: discard blocked readings instead. Release focus on completion, error, stop and watchdog expiry. Tie utterance callbacks to operation identity and stop on focus loss. Reuse the existing foreground-service lifetime for background riding and audit whether additional mediaPlayback service typing/permissions are required for this actual use; do not introduce a separate service or assume a connectedDevice declaration covers every playback requirement.

Refresh readiness on foreground return, explicit test and connection start with bounded initialization, rather than continuous retries. Idle readiness is advisory; playback can still be blocked by a call, route or user volume. No promise of exact Siri voice selection, identical Android voices, or audibility during calls.

Ducking depth is not an application setting: the standard iOS duckOthers option and Android transient-may-duck request do not accept an attenuation amount. The OS and other player control the resulting level or pause behavior. Apple separately offers min/mid/max ducking in its voice-processing input pipeline, intended for voice chat; introducing that pipeline solely for TTS ducking would add unnecessary audio/input complexity. Do not manipulate global media volume. A shared volumePercent setting maps to AVSpeechUtterance.volume on Apple and TextToSpeech.Engine.KEY_PARAM_VOLUME on Android (0–1 gain). It does not alter system volume or boost above it. Zero suppresses automatic speech without focus acquisition and returns finished for a silent test. Snapshot gain at admission; subsequent volume edits apply to the next utterance.

References: [Apple managed speech sessions](https://developer.apple.com/documentation/avfaudio/avspeechsynthesizer/usesapplicationaudiosession), [voice prompts](https://developer.apple.com/documentation/avfaudio/avaudiosession/mode-swift.struct/voiceprompt), [Android audio focus](https://developer.android.com/media/optimize/audio-focus), [Android TTS](https://developer.android.com/reference/android/speech/tts/TextToSpeech).

Show “Lower other audio while speaking” only when the native host reports adjustable lowering. Helper text: “Temporarily makes music and other audio quieter during announcements. Some apps may pause instead.” The shared frontend renders from mandatory AppState.announcementAudio.loweringControl: adjustable on iOS, automatic on Android/macOS. This is a closed OS presentation fact and an explicit narrow exception to the contract's general capability-branch restriction, not optional-method negotiation or frontend platform detection. All hosts expose identical settings/state/method schemas.

Rust persists lowerOtherAudio in the same atomic settings object. Reject false on automatic-only hosts with unavailable and leave the entire settings patch unchanged; true is accepted everywhere. Keep stored data on ordinary errors. The native fact and Rust policy determine admission; frontend hiding alone is not enforcement. Automatic-only hosts keep normal audio-focus behavior without attempting to simulate unchanged-volume mixing. The setting controls a request to the OS, not guaranteed behavior of every player or route. Test speech uses the same preference. Zero volume still acquires no audio session/focus.

Android's focus API has no granted-focus mode requesting unchanged-volume mixing. Retain transient-may-duck there; do not bypass focus handling or change global stream volume. macOS lowering configurability is outside this change.

### Persistence and rollout

Use the existing settings codec and explicit format/schema marker if representation becomes incompatible. Do not add database migrations, old-format readers or compatibility aliases. Ordinary corruption or write failure must preserve owned data. Ship disabled by default; disabling immediately cancels automatic speech and is the operational escape hatch. Reverting implementation must not leave active audio resources or hidden background workers.

Additional ducking reference: [Apple voice-processing ducking](https://developer.apple.com/documentation/avfaudio/avaudioinputnode/voiceprocessingotheraudioduckingconfiguration).

## Risks / Trade-offs

- OS suspension or sparse telemetry → suppress stale speech; validate locked-screen behavior during real BLE delivery on iOS and Android. No hard realtime guarantee.
- Other players pause rather than duck, and calls deny focus → accept native audio policy and report test outcomes; call playback is a bonus, not an acceptance gate.
- Missing offline voices and Android engine differences → expose bounded readiness and test feedback; recover at lifecycle opportunities without downloading assets automatically.
- Bluetooth audio route changes and missed callbacks → operation-specific cleanup, timeout and targeted headset/device tests.
- A one-second configured interval can exceed phrase duration → skip busy intervals and never queue or overlap speech.
- Shared Apple ownership includes macOS → keep its public behavior and focused adapter coverage coherent; mobile locked-screen checkpoints remain scoped to mobile hosts.

## Migration Plan

Commit this validated planning change before implementation. Implement coherent verified slices with task updates, update the contract and all consumers together, then run affected software and native audio/background checkpoints. Sync delta specs and archive in one final commit after relevant verification. No migration or legacy settings path is introduced.
