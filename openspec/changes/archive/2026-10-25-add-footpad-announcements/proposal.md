# Proposal

## Why

Riders can already hear speed and load announcements, but must look at the screen to notice a footpad side losing contact. Optional spoken footpad alerts would make sustained contact loss apparent while riding, with a configurable delay to filter brief releases.

## What Changes

- Add a separate Footpad announcements toggle alongside speed and load, off by default.
- Add a simple delay slider from 1–5 seconds in one-second steps, defaulting to 1 second.
- Track continuous untriggered time independently for each footpad side while speed magnitude is strictly above 5 km/h, independent of display units.
- Say “left footpad off”, “right footpad off”, or “both footpads off” according to the sides that have met the delay.
- Repeat persistent alerts using the existing shared announcement interval and speech slot; put footpad wording before qualifying load and speed in a combined utterance.
- Reuse announcement volume, audio preferences, background service ownership, freshness checks, cancellation and graceful failure behavior.
- Preserve supported saved settings, defaulting only the newly absent fields when opening existing data.

Non-goals: firmware or board configuration changes, fault diagnosis, new telemetry commands, changes to dashboard/session footpad visualization thresholds, separate per-side toggles, new audio engines, and a separate warning queue or repeat interval.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-announcements`: footpad configuration, continuous contact-loss qualification, exact wording, shared cadence and phrase ordering.

## Impact

The frontend announcement contract, Settings UI, client/mock and canonical conformance scenarios gain the footpad options. Shared Rust product settings and announcement policy own validation, persistence and timers. Typed speech effects and both Apple and Android speech consumers gain a closed footpad phrase value. Existing supported settings must remain readable without changing unrelated values; no storage reset, archive-format change or general migration framework is warranted for these additive defaulted settings.

No dependencies or standalone services are added. Extend the existing announcement path and remove superseded two-rule-only assumptions in policy, UI, tests and documentation; no compatibility reader or alternate implementation is retained.
