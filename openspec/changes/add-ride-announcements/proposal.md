# Proposal

## Why

Riders need to hear speed or duty readings above a chosen threshold without looking at the screen. Native speech can provide short announcements over music while the shared Rust service keeps timing and behavior consistent across hosts and disposable WebViews.

## What Changes

- Add application-wide off/speed/duty selection, independent saved speed and duty thresholds, and a configurable repeat interval, disabled by default.
- Evaluate fresh board telemetry in Rust and emit bounded typed speak/cancel effects independently of frontend publication and recording cadence.
- Use native text-to-speech on Apple and Android, briefly requesting audio ducking/focus for each utterance and releasing it on every terminal path.
- Skip unavailable or interrupted audio gracefully without queuing old readings, disturbing the connection, or failing recording. Playback during calls is best effort.
- Extend the authenticated contract, client/mock, settings UI, and canonical conformance coverage together, including an explicit test-announcement operation and bounded audio readiness state.

### Non-goals

Multiple simultaneous rules, combined drive-load announcements, firmware alerts, custom voice downloads or voice pickers, cloud speech, a bundled speech engine, identical voices across platforms, guaranteed call playback, and new telemetry polling are out of scope.

## Capabilities

### New Capabilities

- `ride-announcements`: Threshold-triggered speed/duty speech, repeat policy, lifecycle behavior, native audio fallback and bounded playback.

### Modified Capabilities

- `backend-contract`: Persist announcement settings, expose audio readiness, and add a mandatory authenticated test operation with bounded outcomes.
- `app-navigation`: Include application-wide announcement controls and testing in Settings.

## Impact

- Frontend contract/client/mock and settings feature; shared Rust backend-service, product settings persistence, loopback dispatch and existing C/JNI OS fact/effect exchange; Apple speech adapter and iOS background-audio configuration; Android TTS/audio-focus adapter and existing foreground-service integration; canonical conformance tests.
- Apple covers iOS and macOS with shared synthesis and platform-specific audio-session handling. Android retains the same public contract and policy.
- No new third-party dependency or generated API layer. There is no existing announcement implementation to remove. Do not add browser speech, compatibility readers, migration code or alternate runtimes; use explicit storage version handling if the settings format becomes incompatible and preserve data on operational failure.
