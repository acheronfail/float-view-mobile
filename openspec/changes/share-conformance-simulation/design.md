## Context

`conformance/fixture/software.ts` already serves the direct Rust target, using the independent frame assembler and configuration model. Apple and Android separately implement `protocol/vesc/SoftwareVescPeripheral` (197 and 149 lines at proposal time), with their own framing, firmware, BMS, configuration and telemetry responses. Their software Bluetooth adapters also invent discovery/link events. Android currently chooses this path by emulator detection. The physical simulator and immutable source-derived vectors already live in conformance.

## Goals / Non-Goals

**Goals:** One independent scenario/protocol implementation; thin reusable host integration; deterministic reset and lifecycle isolation; no simulation availability in ordinary or physical builds.

**Non-Goals:** Moving test protocol logic into Rust, using production codecs as the oracle, bypassing the production application service, exercising real BLE through a software test, or making shared testing depend on the BLE policy extraction.

## Decisions

### Reuse the conformance simulator through a test-only relay

Use the existing TypeScript models for protocol responses. Keep `SoftwarePeripheral` as a configuration/preset facade, sharing encoding with the physical fixture model where equivalent; do not force unrelated physical scenarios into one giant simulator. Add exact transcript tests to resolve current native/TypeScript differences rather than taking any one implementation as authoritative. Verify bytes against immutable vectors and vendored upstream evidence.

The conformance runner starts a loopback-only HTTP fixture relay. Thin native test Bluetooth adapters send normalized scan/connect/disconnect/write commands and deliver returned ordered discovery/link/notification events. Direct Rust uses the same session dispatcher; it need not serialize an extra HTTP hop. The application service, product store, codecs and frontend API remain production implementations.

```text
canonical suite -> production frontend API -> Rust application service
                                             | BLE calls/events
                                    thin native test adapter
                                             | test-only HTTP relay
                                conformance-owned scenario/model
```

Choose HTTP because Node, Foundation URLSession and Android HttpURLConnection are already available. Do not add a WebSocket library, native protocol codec, Rust simulator crate or third binding family. Device-specific address/port forwarding stays in target launchers; use Android `adb reverse` to a dedicated loopback fixture port. iOS simulator and macOS connect to host loopback. The relay is separate from the authenticated production API and has no product methods.

### Explicit build and launch selection

Require a dedicated conformance build setting/source set plus explicit launch endpoint and an independent random per-run token. Debug alone, emulator detection alone and user-provided frontend input are insufficient. Keep the seam out of Release and physical-conformance builds, including fixture assets and selection symbols. Scope any cleartext HTTP permission to the test target and loopback; do not weaken production network policy. A missing relay or invalid token fails the software test rather than falling back to real Bluetooth or canned responses.

### Bound and isolate all relay state

One runner owns at most four active host sessions. Per session: one serialized command stream, one outstanding event poll, at most 64 pending commands, 128 queued events and 65,536 aggregate queued byte payload bytes. Limit a complete encoded message to 65,536 bytes and an individual BLE byte payload to 8,192 bytes; check encoded and decoded lengths before allocation. Polls return within one second; commands expire after five seconds; teardown completes within five seconds. Oversize/admission failures terminate the affected test session explicitly instead of silently dropping bytes. Reserve close/cancel outside normal admission.

Every message carries run/session, connection generation and monotonic operation identity. A duplicate/unknown operation is rejected without replaying a write. Responses and events remain ordered; reconnect invalidates old connection traffic without resetting the configured device model unless the scenario requests reset. New test runs reset all mutable configuration and frame buffers. Frontend recreation preserves the running session. Relay loss ends the test connection and reports a bounded harness error; no automatic mutation replay.

### Keep the oracle independent and the coverage honest

Native adapters know only test transport envelopes and OS callback delivery. They cannot parse VESC commands, generate identities, own configuration values or assert product behavior. Retain independent Rust codec vectors and physical ESP32 checks. Compare golden transcripts, fragmented/multiple frames, rejected malformed input, configuration writes/readback, CAN/BMS responses and reconnect/reset behavior across all software targets. This change does not claim OS BLE coverage.

## Risks / Trade-offs

- Relay latency changes timing → asynchronous bounded polling and predicate-based assertions; benchmark representative runs and reject timeout inflation as a substitute for correctness.
- Native replicas differ today → record an explicit transcript/fixture reconciliation before deleting them.
- Test hooks leak into packaged apps → audit Release and physical artifacts, not just source conditionals; test attempted activation without build authorization.
- Shared oracle hides protocol defects → keep independent immutable vectors and physical protocol evidence; never use production decoding to construct expected results.

## Migration Plan

Implement and prove the canonical session/relay first. Switch one software host at a time with its conformance gate, deleting its old peripheral and unused helper/fixture assets in the same verified slice. Preserve real OS adapter selection for physical builds. No data migration or permanent fallback. Rollback is reverting a coherent source/build slice. Finish software and production-exclusion audits, then `just verify`, sync and archive. A Linux host can later implement the same thin relay adapter without a protocol simulator.
