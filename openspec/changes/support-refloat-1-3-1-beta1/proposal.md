## Why

Float View rejects Refloat 1.3.1-beta1 even though most of its wire interface matches supported 1.3.0. The beta also introduces an IMU-timeout alert and stop condition that must be represented correctly before admitting it.

## What Changes

- Admit exactly `1.3.1-beta1`, preserving its suffix throughout compatibility, configuration, cache, Tune, and display identities. Keep all existing audited releases and reject other prereleases and unaudited stable 1.3.1.
- Reuse the audited 1.3 command-33 telemetry and stable product controls; pin the beta source and derive configuration and protocol fixtures from that exact tag.
- Decode IMU timeout as a package error distinct from VESC firmware faults and the persistent fatal latch. Expose its active state and stop reason, recover bounded activation/clear records through command 35, and retain them in saved-session history and exports.
- Extend the shared contract, client/mock, both host integrations, and canonical conformance together. Preserve supported storage and archive data without resets.
- Non-goals: the upstream **Convert from 1.2** feature, any other Tune conversion or cross-version application, runtime-tuning commands, remote/motor control, data recording, a general alert framework, and automatic support for later betas or stable 1.3.1.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Exact prerelease admission and actionable rejection of neighboring identities.
- `refloat-telemetry-pipeline`: Beta telemetry, IMU-timeout state and bounded alert-history recovery, with unchanged older profiles.
- `refloat-configuration`: Exact beta schema identity, verified configuration lifecycle, cache separation, and Tune isolation.
- `refloat-live-controls`: Beta control profile and timeout-aware mutation safety without treating timeout as a fatal latch.
- `backend-contract`: Typed timeout state, stop reason, and bounded historical events shared by every backend and the mock.
- `live-board-dashboard`: Accessible live timeout warning and historical timeout presentation.
- `backend-conformance`: Source-derived beta coverage, supported-data preservation, and focused interoperability evidence.

## Impact

Affected owners include shared Rust compatibility, discovery, telemetry codecs/service, configuration, mutation coordination, runtime, session persistence/readers, archive/export and loopback projection; frontend contract/client/mock and live/history UI; conformance fixtures and the configuration-fixture generator. Apple and Android continue using the existing shared service and C/JNI bindings. The public API gains additive timeout semantics; durable events gain a distinct timeout kind. Existing databases, Tunes, archives, and frozen baseline fixtures remain supported. No dependency, generated API, parallel runtime, or general migration framework is added. Replace firmware-only alert parsing assumptions where necessary; remove no supported release or decoder.
