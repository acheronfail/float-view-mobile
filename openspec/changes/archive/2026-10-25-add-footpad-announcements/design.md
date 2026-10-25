# Design

## Context

See proposal.md for motivation and the ride-announcements delta for observable behavior. `backend-service::announcements` already owns current observations, one monotonic cooldown and one bounded speech slot. Normalized telemetry includes known left/right/both/none footpad states. Native Apple and Android speech adapters format typed load/speed effects; Android currently validates an exact effect key set. Product announcement settings are serialized in the existing settings object. The earlier finish-alert-playback change is already reflected in code and the main specification.

## Goals / Non-Goals

**Goals:** Extend the existing shared service with bounded per-side timing; keep identical policy on iOS, macOS and Android; preserve existing supported settings; use one simple control group.

**Non-Goals:** Duplicate policy in the frontend or native audio adapters, infer pressure from ADC voltages, add hardware polling, modify session fault overlays or add dependencies.

## Decisions

### Extend existing settings and contract

Add `footpadEnabled` and `footpadDelaySeconds` to announcement settings, with defaults false and 1. Validate the delay as a whole number in 1–5 through the same complete revision-checked update path. Update contract, defaults, client/mock, shared projection and consumers together. Add defaults only for absent persisted fields; do not treat malformed values as missing. Preserve frozen baseline fixtures and add regression coverage for opening old supported settings and reopening new settings. This is an additive settings change within the existing storage layout, with no schema transition or marker bump required; validate that conclusion against actual persistence decoding during implementation.

### Track release qualification in the shared service

Extend the current observation with footpad state and keep two optional monotonic release-start timestamps plus the previous observation time/context. Process observations before replacing prior freshness evidence, so a gap of at least two seconds resets continuity even if the stale timer did not run. Map `left` to left pressed/right released, `right` to the opposite, `both` to both pressed, and `none` to both released. Missing/unknown states reset qualification rather than counting as release.

Apply the spec's fixed magnitude gate in km/h before unit conversion. Begin each duration on the first fresh eligible released observation; never backdate across an unknown interval. Returning contact clears only that side. Gate/context/freshness loss and footpad enable/delay edits clear both. A settings edit starts counting again from a fresh observation after the edit; changes to unrelated audio gain do not manufacture new release time. All resets retain the shared cooldown.

Reuse the existing bounded speech timer to evaluate pending release deadlines and freshness as well as qualified speech. Keep it armed while either side is accumulating time, not only after qualification; otherwise a delay can stall when speed/load alerts are off. Two timestamps and existing observation/context state suffice; no samples, episodes or pending utterances are collected. Frontend timers were rejected because WebViews are disposable, and native timers would duplicate product policy.

### Extend the typed speech effect with a closed footpad value

Add an optional footpad value with the closed cases left/right/both. The shared service selects currently qualified sides. Apple and Android decode the same bounded value and format the exact agreed phrases before the existing load/speed fragments. Accept footpad-only speech as nonempty. Update strict effect validation, bridge fixtures and tests together; no arbitrary phrase strings or new host policy are needed. Retain current speech expiry, callback ownership, volume and audio-focus behavior.

All automatic rules use the existing admission cadence. Completing the release delay makes a side eligible but does not reserve or preempt a speech slot. Persistent eligible sides repeat, using the latest qualification at admission. Fresh contact recovery or slowing allows the admitted phrase to finish; stale or replaced board context and rule edits preserve cancellation. Existing manual preview kinds remain unchanged; exact footpad wording is exercised through typed host tests and canonical automatic scenarios rather than adding another preview API.

### Reuse the Settings control and save patterns

Extend `Announcements.svelte` with the existing Off/On control style and a native range input labelled Footpad delay, `min=1`, `max=5`, `step=1`, displaying the selected seconds. Keep the delay visible and retained while alerts are off, like the existing editable thresholds. Reuse serialized autosave and error handling and explain the above-5-km/h gate with the selected-unit equivalent when appropriate. Verify touch/pointer and keyboard changes, without introducing custom drag mechanics. Remove outdated two-rule-only explanatory copy and assumptions.

## Risks / Trade-offs

- Brief gaps can falsely accumulate release time → reset continuity on the previous observation age as well as timer expiry, and test delayed workers at the exact freshness boundary.
- “Off” describes a reported contact state and does not diagnose failure → use only known decoded contact state, and explain the trigger in Settings.
- Existing cooldown can delay a newly qualified footpad warning → retain the agreed common cadence and include warnings first at the next eligible attempt.
- Supported persisted JSON predates the new fields → default only absent new fields, preserve unrelated preferences, and test frozen baseline data without modifying it.
- Strict native decoders can reject an expanded effect → update both host decoders and phrase tests in the same implementation change.

## Migration Plan

Ship contract, persistence defaults, policy, frontend/mock and both native effect consumers together. Existing installations acquire disabled footpad alerts and a one-second delay on read, then save the expanded object normally. No data deletion, new compatibility reader, archive version or general migration framework is introduced. Disable the new toggle for a reversible operational rollback; reverting binaries must not assume older strict decoders accept newly saved keys, so any code rollback must preserve readers for these supported additive fields.
