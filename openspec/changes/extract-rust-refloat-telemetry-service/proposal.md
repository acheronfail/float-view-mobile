## Why

Apple and Android still duplicate the same 366-line/357-line Refloat telemetry application service even though its legacy/dynamic decoders and connection-wide request scheduler now have bounded Rust counterparts. Extracting this pure coordination slice next lets the future unified Rust runtime compose those candidates directly, without adding another temporary Apple FFI or Android JNI package or changing either production host.

## What Changes

- Add one safe bounded `backends/core/refloat-telemetry-service` candidate that composes the Rust legacy/dynamic telemetry models with the Rust request-scheduler facts.
- Move route-correct layout, legacy, realtime, and compact request selection; bounded layout retry; one pending telemetry response; timeout recovery; adaptive ride/idle/post-ride cadence; compact-cache replacement/merge; normalized session sequencing; elapsed-time calculation; and ordered timer/diagnostic effects into a deterministic host-independent state machine.
- Add source-derived parity and adversarial tests covering the complete duplicated Swift/Kotlin policy, exact request facts and bytes, legacy/dynamic behavior, stale/mismatched events, counter/timestamp boundaries, cache lifetime, and deterministic effect ordering.
- Keep Swift and Kotlin selected in production. Add no bridge, generated binding, packaged artifact, callback, clock, timer, BLE writer, persistence port, local server, thread, or OS dependency; a later unified-runtime cutover will use the safe Rust API directly and delete the native services.
- Extend repository ownership/dependency audits and run the complete software and Android/iOS/macOS ESP32 gates with only slow location acquisition deferred to the agreed manual follow-up.
- Non-goals: host cutover or native deletion; changing frontend/backend contracts; SQLite or local-webserver migration; BLE, background, location, lifecycle, files, theme, permissions, WebViews, or UI changes; changing telemetry wire decoding semantics; creating `refloat-telemetry-service-apple-ffi` or `refloat-telemetry-service-android-jni`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the safe compositional Refloat telemetry-service candidate, complete state/effect parity, and strict boundary below host execution.
- `platform-hosts`: Record candidate versus selected-host ownership and the direct-composition/no-new-bridge migration path.
- `backend-conformance`: Require focused parity/adversarial/isolation evidence, the complete software gate, and unchanged all-host physical behavior.

## Impact

The locked Rust workspace gains one safe candidate depending only on the existing `refloat-dynamic-telemetry`, `refloat-legacy-telemetry`, and `vesc-request-scheduler` crates. Root verification gains focused tests and isolation audits. Native Swift/Kotlin source, host packages, runtime behavior, contracts, persistence, and OS capability ownership remain unchanged in this step; no compatibility machinery or bridge can yet be removed.
