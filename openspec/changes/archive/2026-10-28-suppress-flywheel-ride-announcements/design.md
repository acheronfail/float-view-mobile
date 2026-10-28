# Design

## Context

See proposal.md for motivation. One shared backend owner admits speech independently of views. Speech and visual battery warnings share telemetry freshness checks, so suppressing all freshness would accidentally remove electrical evidence. The mutation coordinator already rejects competing writes during flywheel, and configuration current values are invalidated because firmware temporarily modifies its in-memory configuration.

## Goals / Non-Goals

**Goals:** Apply flywheel exclusion to automatic speech without disabling telemetry freshness or the existing single-slot audio lifecycle.

Also exclude flywheel samples from derived session footpad-off-at-speed markers without altering stored history.

**Non-Goals:** No native adapter, frontend control, protocol, public contract or storage changes. Ride statistics and GPS policy are explicitly deferred by the user.

## Decisions

- Track observed flywheel mode in connection-scoped announcement state. Known normal/hand-test telemetry clears the exclusion; absent/unknown mode does not clear an established flywheel observation. Connection invalidation clears this transient state. Do not infer mode from speed, footpads or an optimistic enable request.
- Separate automatic-speech eligibility from valid telemetry context. Keep visual battery observation/freshness intact, but prevent battery/footpad qualification and remove all automatic candidates during flywheel. Reuse existing cancellation, speech completion and timer handling; late completions cannot affect a replacement phrase.
- Preserve routine and independent battery cooldowns. Exclusion resets qualification rather than advancing or clearing attempt times. Manual previews keep their explicit test behavior and bounded slot.
- Use focused service policy tests for entry, sustained exclusion, delayed callbacks, exit, reconnect, previews and visual evidence. No dependencies or data upgrades are needed.
- The shared frontend's bounded footpad-history loader consumes recorded package-mode transitions alongside footpad transitions, ordered by elapsed time and source sample sequence. Flywheel excludes both marker types and breaks episode continuity even when the interval contains no retained speed sample. Known normal/hand-test mode ends exclusion; missing or unknown mode cannot clear it. Sessions without mode history preserve their existing behavior. Unit pagination/ordering tests and the existing Chromium/WebKit overlay test verify this read-time derivation.

## Risks / Trade-offs

- Silent automatic battery phrases during flywheel → retain electrical telemetry/visual evidence; current configuration is already invalidated during flywheel, so riding thresholds cannot safely be assumed available.
- Shared freshness accidentally hides visual warnings → verify profile-based over-voltage evidence during flywheel.
- Existing ride metrics/GPS still interpret flywheel running state as riding → documented follow-up, outside this change.

## Deferred policy review

Keep the connection, telemetry polling, raw samples, package-mode transitions,
firmware fault evidence and the existing Disable action. Preserve firmware's
footpad-abort behavior; suppressing app speech must not change board safety.
Continue rejecting competing device mutations and current-configuration reads,
since flywheel temporarily replaces in-memory tuning values.

For a later change, exclude flywheel intervals from ride distance, riding time and
ride maxima, and pause ride GPS capture. Retain raw samples for diagnosis and
place explicit boundaries around those intervals so distance integration cannot
bridge entry or exit. Session footpad markers are handled in this change.
This change does not implement the statistics/GPS recommendations or rewrite saved recordings.
