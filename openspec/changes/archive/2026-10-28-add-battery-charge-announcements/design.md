# Design

## Context

See proposal.md for motivation and scope. The shared service already owns announcement qualification, one native speech slot, monotonic cooldown, expiry and connection-generation isolation. Current speech carries optional footpad/load/speed values; Settings stores revision-checked global preferences. Refloat configuration is read during connection setup through existing bounded readers, with a current values snapshot separate from schema/defaults/editor drafts.

Vendored Refloat defines `tiltback_lv` and `tiltback_hv` in `vendor/refloat/src/conf/settings.xml`. Newer supported combinations interpret values below 10 V as per-cell and multiply by the controller-configured series count; older families use pack values. `motor_data_refresh_motor_config` in `vendor/refloat/src/motor_data.c` demonstrates current semantics. This must be audited across supported package/firmware families rather than applying the newest heuristic universally. Firmware pushback additionally considers duty, sag, timing and optional BMS conditions; these announcements identify approaching voltage boundaries, not an exact prediction of pushback.

## Goals / Non-Goals

**Goals:** Derive deterministic headroom from authoritative board configuration; keep all qualification and scheduling in the existing shared owner; preserve bounded native playback and stored data.

**Non-Goals:** No second runtime, telemetry poller, queue or alert framework. No runtime estimation of chemistry, internal resistance, time-to-limit or individual cell health. No voltage policy in the frontend or native hosts, and no separate fault display or warning overlay.

## Decisions

### Resolve current board settings once through the existing configuration owner

Use only successfully read current values tied to the selected controller, connection generation and applicable supported schema. Resolve a pair of finite positive pack thresholds L < H using the matching upstream compatibility rules. Per-cell conversion requires the controller's positive series count, never the saved app profile or a count inferred from voltage. Schema defaults and unsaved drafts are not evidence of current configuration. Unknown semantics, read failure, missing fields or invalid values make the derived battery rule unavailable without affecting other announcements.

Refresh using existing configuration reads on reconnect and on confirmed configuration writes/readback or explicit current-value refresh. Suspend qualification while current settings are invalidated/replaced, and requalify against the confirmed replacement. Do not continuously poll settings or retain thresholds across controllers/generations. An external tool's unobserved edit is only discovered at a subsequent read; this is consistent with the existing configuration lifecycle.

### Use fixed fractions of the configured voltage window

For R = H − L:

| Boundary      | Formula            | L = 60 V, H = 86 V |
| ------------- | ------------------ | ------------------ |
| Low entry     | L + 0.10R          | 62.6 V             |
| High entry    | H − 0.05R          | 84.7 V             |
| Low recovery  | Low entry + 0.02R  | 63.12 V            |
| High recovery | High entry − 0.02R | 84.18 V            |

Use unrounded readings for comparisons, with strict entry (< low, > high) and inclusive recovery (>= low recovery, <= high recovery). The same formulas work for pack and per-cell representations after normalization. Reject non-finite or unrepresentable derived boundaries. Fixed pack-volt offsets scale poorly; assumed cell voltages violate the chosen source of truth; trend prediction adds complexity without guaranteeing advance notice. The 10%/5% constants are the agreed initial policy, not a claim of universal physical validation. Representative recorded traces should check timing/chatter; any proposed adjustment must keep the artifacts coherent rather than silently changing accepted examples.

### Require Refloat riding state, plus regen for advance high warnings

Require fresh Refloat packageState `running` for both alerts, independent of speed and other announcement settings. The ready connection state alone does not establish riding. Zero, low, reverse or unavailable speed does not suppress battery alerts while fresh running state and required electrical observations qualify. Non-running, missing or unknown package state suppresses both alerts, even if the board reports motion. The existing footpad speed gate is unchanged. Low warning has no current-direction gate. High entry before H requires fresh negative battery current using the existing normalized sign convention; motor current, charging flags and battery percentage are not substitutes. At or above H, regen evidence is unnecessary, but fresh running state, fresh voltage and valid resolved limits remain mandatory. At or below L, low qualifies without the ordinary entry delay as well.

Leaving running state or losing required observations resets qualification and active battery latches, prevents new speech and preserves cooldown. An already admitted phrase finishes when fresh valid board context remains, matching existing playback behavior. Invalid connection context, stale speech inputs, configuration invalidation cancels affected battery speech. Re-entry after returning to running state requires fresh confirmation unless already beyond a critical boundary.

### Confirm entry and recovery without accumulating time across gaps

Require continuous fresh eligible observations for 2 seconds below low entry or 500 ms above high entry; crossing the actual critical boundary bypasses entry confirmation. Require 2 seconds beyond the recovery boundary to clear an active latch. Values between entry and recovery keep an active latch, but do not qualify an inactive one. Broken entry/recovery conditions reset the corresponding timer. A telemetry gap of at least 2 seconds resets transient battery state even if the next sample arrives after the gap. No timer-only transition can make stale evidence eligible. After high entry, loss of fresh regen before H suppresses further high attempts and resets high qualification; it is not a reason to announce while drawing power.

### Extend one speech slot with closed battery phrases

Add a closed optional battery phrase (`low` or `overVoltage`) to the existing shared speech effect and its validated host representations. Native hosts only render “Battery low” or “Over voltage” with the existing offline voice/audio policy. Do not encode arbitrary speech text or duplicate threshold decisions in adapters.

Battery attempts are standalone utterances, chosen ahead of due ordinary footpad/load/speed announcements when the slot is free. If conflicting battery evidence is encountered, choose over voltage. Do not interrupt admitted ordinary/manual speech to make room. Busy battery attempts are dropped, not queued; subsequent admission re-evaluates fresh conditions. Ordinary phrase ordering remains footpad, load, speed.

Use separate process-local monotonic last-attempt timestamps for low and high, each with a fixed 60-second minimum interval. A newly due high warning does not wait for low or routine cooldown. When a free-slot battery attempt occurs, advance its timestamp even if muted/unavailable/focus-denied, and advance the existing ordinary cooldown to avoid immediately following a warning with routine speech. Routine/manual attempts do not advance battery timestamps. A busy dropped attempt does not consume a battery interval; fresh evidence can qualify when the slot next becomes free. There is no stored pending phrase or catch-up. Chatter, riding-state transitions, toggle edits, settings refresh and reconnect do not clear battery cooldowns. Process restart clears transient state and keeps saved preferences.

The existing ten-second speech deadline, one-slot bound, volume-zero behavior, audio preferences and late-callback rejection apply unchanged. The host speech-enabled status includes battery charge and footpad settings as well as speed/load when volume is nonzero, so Android uses its existing media-playback foreground-service type for any enabled announcement rule. Background/locked delivery is subject to existing native execution/audio availability, not a new guarantee or background mode.

### Add one default-on global setting compatibly

Add `batteryChargeEnabled: boolean` to the hand-written frontend contract/defaults/mock and shared settings. Public results and complete revisioned updates carry it. Missing fields in supported stored settings decode as true; explicit false survives save/reopen and invalid present types fail validation. New public updates validate the full current shape. Use the same additive compatibility pattern as footpad settings, without modifying frozen baseline fixtures, resetting storage or changing the storage marker/archive format.

Place a Battery announcements switch alongside Speed, Load and Footpads in Ride announcements. Brief helper text explains the two voltage alerts, board-derived thresholds and one-minute repeats. Reuse autosave serialization, revision conflict handling, keyboard/native button semantics and guarded primary-pointer activation. No threshold, delay or repeat controls are added for this rule. Turning it off cancels battery speech but preserves shared visual qualification; it leaves other rule settings intact. Re-enabling retains cooldown and uses currently fresh qualified evidence.

No new dependency is needed. Reuse configuration/controller facts and existing audio adapters; do not keep a parallel legacy speech path after extending the current closed representation.

## Risks / Trade-offs

- Sudden sag/regen can cross headroom faster than BLE and speech respond → describe the alerts as supplementary voltage notice; do not promise seconds before protection.
- Bad board configuration produces bad reference thresholds → reject structurally invalid pairs and missing semantics, while avoiding invented chemistry validation. Firmware remains responsible for protection.
- The two margins cannot model every pack's impedance → test scale invariance and replay representative low-battery/regen traces where available; record actual coverage limits.
- State transitions and brief voltage spikes can chatter → fresh running-state gate, entry confirmation, proportional hysteresis and cooldown retention.
- Priority can delay ordinary speech → battery phrases are short, capped at one attempt per kind per minute, with no playback preemption or queues.
- Native audio differs by OS → reuse established handling and scope physical checks to new phrase playback and running/non-running behavior under foreground/background/lock where supported, using safe simulated telemetry rather than real voltage extremes.

## Automatic native replay verification

The opt-in `conformance/test/announcements.native.ts` cases drive source-derived
wire telemetry through the Apple macOS host and Android emulator host for both
pack and per-cell configuration families. The existing fixture relay accepts
an optional telemetry source/version; native target wrappers observe closed
battery phrases emitted at the speech-driver boundary. Apple replaces only its
OS driver under `FLOAT_VIEW_CONFORMANCE`; Android uses a replacement engine
only in the Conformance source set, while Debug/Release explicitly select the
native engine. The usual native request decoding, phrase formatting, admission,
expiry and completion handling still execute. No production endpoint, business
policy or arbitrary speech command is added.

Replay covers invalid limit rejection, non-running suppression, brief low
crossings, confirmed low wording at zero speed, regen-gated high wording,
independent priority/cooldown, a real one-minute repeat, persisted off and
cooldown retention across reconnect/re-enable. Real voice availability, audible
output, audio routing and background/locked-device execution remain the
separate physical checkpoint using production builds.

## Migration Plan

Ship contract, client/mock, shared persistence/policy, loopback/native boundaries and both host speech formatters together. Verify baseline settings with the new field absent, explicit-off round trips and invalid-present rejection. No destructive migration is needed. Roll back code together if necessary; preserve the stored additive preference, and do not use a storage reset or promise arbitrary downgrade compatibility for strict older decoders.

## Live battery widget warnings

Publish nullable `batteryWarning` (`low` / `overVoltage`) in the complete active-connection snapshot from separate shared visual gates. Speech retains its existing 10%/5% margins, riding/regen gates, confirmation and cadence unchanged. Visual qualification requires a ready selected non-BMS connection and voltage less than two seconds old, independently of riding state, regen, speech settings, voice readiness, volume, slot and cooldown. Invalid context or stale observations clears visuals.

Low visuals use the current resolved Refloat limits: entry below L + 0.25R, recovery at or above L + 0.27R for two continuous seconds, with two-second entry confirmation unless at or below L. For 60/86 V this means 66.5 V entry and 67.02 V recovery. High visuals enter immediately on the first fresh unrounded pack-voltage reading strictly above the configured cell profile’s zero-current full-charge voltage multiplied by its validated series count. This is independent of the capped/smoothed percentage and Refloat high speech limit. High recovers after two continuous seconds at or below 99.5% of that full-charge voltage. Missing/invalid/mismatched estimation configuration disables high visuals without guessing chemistry; missing Refloat limits disables low visuals independently. Changed threshold context, reconnect/controller replacement and gaps of at least two seconds reset visual qualification. High wins overlapping latches. Profile edits that change or invalidate visual thresholds requalify only on fresh subsequent samples. Both native hosts project this shared field unchanged. The frontend and mock do not invent voltage thresholds.

The normal and custom dashboard battery widgets show a warning icon and Battery low / Over voltage label, amber voltage, and an amber outline around the whole track breathing every three seconds. High voltage adds stationary diagonal hatching over the track. Warning presentation overrides charging animation, preserves real fill length and percentage (including unavailable values), and never uses firmware fault presentation. Reduced motion keeps the label, steady outline and hatching without animation. Compact cards retain a readable warning label.
