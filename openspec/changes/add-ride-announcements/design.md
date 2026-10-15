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

Extract a small shared Rust load-component calculation from the current native projection and reuse it for native presentation and announcements. Match frontend gauge semantics with parity fixtures: absolute duty saturated at 100 percent; motor current prefers filtered, then directional, then raw; raw current sign selects the positive or negative controller limit; battery uses its corresponding signed current limit. Missing/zero limits omit only that current component, and non-finite inputs are unavailable. Keep the frontend gauge aligned without moving policy into JavaScript.

The load rule selects the highest available component using the gauge winner calculation, then compares it against the shared load threshold. Ties retain duty before motor before battery, including ties created by saturation at 100 percent. Each interval recomputes the winner from fresh telemetry and current controller limits; never list runner-up components. The winning source may change between intervals without resetting cooldown. A combined load/speed phrase is one utterance, one audio-focus lease and one cooldown event, still subject to the existing ten-second bound. Limits are scoped to the active controller and cannot survive controller replacement as valid inputs.

### One scheduler for both enabled rules

Replace exclusive mode selection with two enable flags. At every due attempt evaluate both enabled rules, then compose one phrase containing the qualifying load winner first and speed last. This is bounded at two readings, avoids starvation and repeated ducking, and needs no priority queue or separate per-rule deadlines. A reading newly above threshold during cooldown joins the next attempt; it does not interrupt speech or bypass the cooldown. If an utterance is still active, skip the due attempt as before. Users retain the one shared repeat interval.

### Typed speech effects and bounded lifecycle

Add closed speak/cancel effects with an operation identity, connection generation for automatic speech, an optional winning load reading (source duty/motor/battery) and optional speed reading, at most two items in load-then-speed order, plus speech gain. Swift/Kotlin render one localized utterance, naming the winning load component before its percentage. Rust selects the qualifying load winner and speed; native code does not calculate utilisation. Do not pass board identity, raw telemetry collections or arbitrary strings. A cancellation arriving before speech starts must suppress that operation; adapters must check admission and expiry before acquiring audio resources.

Maintain one active operation and no pending queue. Native started/finished/interrupted/unavailable facts carry its identity. Both coordinator and adapter enforce a ten-second deadline from admission; every terminal path stops playback as appropriate and releases only the matching operation's audio resources. Ignore duplicate or late completions. Coalesce readiness changes rather than publishing per-sample audio state. The readiness reason is bounded; a denied transient focus request need not mark the engine permanently unavailable.

Rule/threshold/interval changes, generation replacement and disconnect cancel automatic speech; a volume-only edit applies to the next utterance. Listener/WebView replacement cannot reset automatic policy. A disconnected test requester releases its bounded waiter and cancels only its own test. Process shutdown cancels all playback; process restart restores settings only. Late callbacks after timeout cannot deactivate a newer speech session.

### Settings and authenticated testing

Add a complete `Settings.announcements` value with `speedEnabled`, `loadEnabled`, `speedThresholdKph`, `loadThresholdPercent`, `repeatIntervalSeconds`, and `volumePercent`. Defaults and bounds are specified in the ride-announcements delta. A supplied nested value is validated and replaced atomically; do not introduce ambiguous nested partial merges. Persist through the existing product store and settings revision. Unit changes convert display and input without rewriting the canonical physical threshold merely for presentation.

Add `AppState.announcementAudio` with the closed readiness and reason fields defined in the contract delta. Add mandatory `announcements.test({ kind })` with `kind: speed | load | combined`, returning `{ outcome }` within the speech deadline. The result describes the playback lifecycle, not guaranteed audibility. Test readings are fixed in the service: a load test says “motor 72 percent”, or a speed test uses 25 km/h, converted/rounded for current speed units. A combined test concatenates the fixed load and speed examples. A test works without a board or enabled rule; it never changes preferences. Busy/cooldown admission returns invalid_state before allocating a waiter. Use existing authenticated dispatch, error envelopes and cancellation machinery.

The shared settings feature owns controls, units, status and pending/error feedback. Use the existing primary-pointer and guarded keyboard activation patterns. Mock transport implements the complete schema and deterministic test outcomes; it does not call browser speech. Extend canonical conformance for request validation, revisions, persistence, state ordering and bounded native outcome simulation.

### Native synthesis and audio ownership

Apple retains AVSpeechSynthesizer in the service-owned adapter and uses an installed offline voice. Begin the iOS integration with `usesApplicationAudioSession = false`, allowing the system to manage speech mixing/ducking. If targeted device checks show this does not meet music/background requirements, use an explicit `.playback` / `.voicePrompt` session with `.duckOthers` and `.interruptSpokenAudioAndMixWithOthers`, activating per utterance and deactivating with `.notifyOthersOnDeactivation` on all terminal paths. This is an implementation choice inside the same adapter and acceptance behavior, not two shipping fallback paths. Keep the selected implementation only. Reuse synthesis on macOS with its appropriate audio behavior; do not call iOS-only AVAudioSession APIs there.

Enable iOS audio background capability for real spoken announcements, retaining BLE delivery as the source of observations. Do not play silence to keep the process alive. The synthesizer stays alive across listener replacement; the audio session does not stay active between prompts. Observe interruption/route/reset events and make cancellation release resources deterministically.

Android retains TextToSpeech in the process-owned native adapter. Initialize asynchronously, select an installed voice that does not require a network connection, set CONTENT_TYPE_SPEECH and USAGE_ASSISTANCE_NAVIGATION_GUIDANCE, and request AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK immediately before speaking. Use the same attributes for focus and playback. Do not request delayed focus: discard blocked readings instead. Release focus on completion, error, stop and watchdog expiry. Tie utterance callbacks to operation identity and stop on focus loss. Reuse the existing foreground-service lifetime for background riding and audit whether additional mediaPlayback service typing/permissions are required for this actual use; do not introduce a separate service or assume a connectedDevice declaration covers every playback requirement.

Refresh readiness on foreground return, explicit test and connection start with bounded initialization, rather than continuous retries. Idle readiness is advisory; playback can still be blocked by a call, route or user volume. No promise of exact Siri voice selection, identical Android voices, or audibility during calls.

Ducking depth is not an application setting: the standard iOS duckOthers option and Android transient-may-duck request do not accept an attenuation amount. The OS and other player control the resulting level or pause behavior. Apple separately offers min/mid/max ducking in its voice-processing input pipeline, intended for voice chat; introducing that pipeline solely for TTS ducking would add unnecessary audio/input complexity. Do not manipulate global media volume. A shared volumePercent setting maps to AVSpeechUtterance.volume on Apple and TextToSpeech.Engine.KEY_PARAM_VOLUME on Android (0–1 gain). It does not alter system volume or boost above it. Zero suppresses automatic speech without focus acquisition and returns finished for a silent test. Snapshot gain at admission; subsequent volume edits apply to the next utterance.

References: [Apple managed speech sessions](https://developer.apple.com/documentation/avfaudio/avspeechsynthesizer/usesapplicationaudiosession), [voice prompts](https://developer.apple.com/documentation/avfaudio/avaudiosession/mode-swift.struct/voiceprompt), [Android audio focus](https://developer.android.com/media/optimize/audio-focus), [Android TTS](https://developer.android.com/reference/android/speech/tts/TextToSpeech).

A configurable “Lower other audio while speaking” switch is deferred: iOS can mix with mixWithOthers without duckOthers, but Android's focus API has no granted-focus mode requesting unchanged-volume mixing. Transient focus without may-duck generally requests interruption instead. Avoid bypassing focus handling, promising identical mixing, or changing global stream volume to simulate this option. Keep automatic platform-managed lowering in this change.

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
