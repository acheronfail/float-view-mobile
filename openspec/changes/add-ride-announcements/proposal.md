# Proposal

## Why

Riders need to hear speed or load readings above a chosen threshold without looking at the screen. Native speech can provide short announcements over music while the shared Rust service keeps timing and behavior consistent across hosts and disposable WebViews.

## What Changes

- Unify load calculation in one shared Rust owner for the primary gauge, widgets, live charts, notification/Live Activity/Dynamic Island, session capture and announcements; remove duplicate frontend and Rust calculations while preserving session review of recorded telemetry.
- Add independent application-wide speed and load enable switches and thresholds, a shared repeat interval, and announcement volume, with both rules disabled by default.
- When load is enabled, announce only the highest available duty, motor or battery utilisation component when it exceeds the shared threshold, using the live gauge calculations.
- Combine the qualifying highest load component and speed in one utterance per interval, with load first, so neither rule overlaps or starves the other.
- Evaluate fresh board telemetry in Rust and emit bounded typed speak/cancel effects independently of frontend publication and recording cadence.
- Use native text-to-speech on Apple and Android, briefly requesting audio ducking/focus for each utterance and releasing it on every terminal path.
- Add an iOS-only “Lower other audio while speaking” switch, enabled by default; Android and macOS retain automatic native audio handling and hide the switch.
- Skip unavailable or interrupted audio gracefully without queuing old readings, disturbing the connection, or failing recording. Playback during calls is best effort.
- Extend the authenticated contract, client/mock, settings UI, and canonical conformance coverage together, including an explicit test-announcement operation and bounded audio readiness state.
- Update the root README's dot-point feature list when implemented to advertise spoken speed/load announcements with configurable thresholds and repeat intervals.

### Non-goals

Arbitrary rule collections, configurable ducking depth or an Android/macOS ducking switch, firmware alerts, custom voice downloads or voice pickers, cloud speech, a bundled speech engine, identical voices across platforms, guaranteed call playback, and new telemetry polling are out of scope.

## Capabilities

### New Capabilities

- `ride-announcements`: Threshold-triggered speed/load speech, repeat policy, lifecycle behavior, native audio fallback and bounded playback.

### Modified Capabilities

- `backend-contract`: Expose authoritative live load values and preserve captured historical telemetry semantics; persist announcement settings, expose audio readiness, and add a mandatory authenticated test operation with bounded outcomes.
- `app-navigation`: Include application-wide announcement controls and testing in Settings.

## Impact

- Frontend contract/client/mock and settings feature; shared Rust load calculation, application-runtime session capture, backend-service, existing captured session load fields, product settings persistence, loopback dispatch and existing C/JNI OS fact/effect exchange; Apple speech adapter and iOS background-audio configuration; Android TTS/audio-focus adapter and existing foreground-service integration; canonical conformance tests.
- Apple covers iOS and macOS with shared synthesis and platform-specific audio-session handling. Android retains the same public contract and policy.
- No new third-party dependency or generated API layer. Remove the frontend gauge utilisation/winner implementation and duplicate Rust notification and session-capture formulas, retaining presentation-only helpers. There is no existing announcement implementation to remove. Do not add browser speech, compatibility readers, migration code or alternate runtimes; use explicit storage version handling if the settings format becomes incompatible and preserve data on operational failure.
