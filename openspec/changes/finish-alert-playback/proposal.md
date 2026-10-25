# Proposal

## Why

Speed and load announcements cut off mid-phrase when readings fall below their thresholds. Letting the current phrase finish makes the alerts easier to hear and less abrupt.

## What Changes

- Finish admitted automatic speech when fresh readings drop below all enabled thresholds.
- Continue requiring fresh above-threshold readings for subsequent attempts, with the existing cooldown and no backlog.
- Preserve cancellation for stale/unavailable board context, disconnection, controller replacement, rule edits, interruption and timeout.
- Remove threshold loss from the active-speech cancellation condition.

Non-goals: changing speech wording, thresholds, repeat timing, native audio adapters or test playback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-announcements`: distinguish below-threshold readings from invalid board context when deciding whether to cancel admitted speech.

## Impact

Shared Rust announcement policy and regression tests, applying equally to Apple and Android. No public wire API, persistence, compatibility or dependency changes.
