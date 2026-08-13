## Why

Apple and Android still maintain near-duplicate 433-line and 373-line Refloat configuration lifecycle services even though Rust now owns the underlying schema and value codec on every host. Extracting their deterministic state machine is the next bounded application-behavior step toward one Rust runtime, and gives later host cutovers a source-derived, adversarially tested owner without adding another platform bridge in this change.

## What Changes

- Add a safe `backends/core/refloat-configuration-service` Rust candidate that depends on the existing codec and owns the audited release catalogue, schema paging/assembly, cache admission decisions, connection generations, operation identities, revisions, flywheel restrictions, pending-operation transitions, failure recovery, value reads, write acknowledgement, normalized readback verification, and conflict classification.
- Model host work as bounded typed inputs and effects so the candidate owns decisions while remaining independent of Bluetooth, VESC framing, schedulers, clocks, files, SQLite, loopback transport, and platform APIs.
- Add exhaustive source-derived and adversarial Rust tests that replay the complete Swift/Kotlin service behavior, including cache hit/miss/corruption, direct/CAN route identity, stale responses, paging bounds, read/default/write/readback, flywheel transitions, reconnect/reset, operation wrap, and payload-safe failures.
- Extend workspace, dependency, ownership, and verification audits, then run complete software and Android/iOS/macOS ESP32 regression while native services remain selected.
- Document the candidate as the intended stateful core for later Apple and Android cutovers and eventual deletion of both codec bridges and native configuration services.
- Non-goals: no production host cutover; no new Apple FFI or Android JNI surface; no change to frontend contracts, SQLite, cache storage formats, scheduler ownership, OS capabilities, or physical behavior; no deletion of Swift/Kotlin service code until separate host-cutover changes pass.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded Rust ownership and parity requirements for the extracted Refloat configuration lifecycle state machine.
- `platform-hosts`: Locate the candidate below native application services and preserve every production host and OS boundary until explicit cutovers.
- `backend-conformance`: Require exhaustive candidate parity, isolation, complete software verification, and all-host physical regression evidence.

## Impact

- Adds one locked safe Rust workspace crate under `backends/core` and focused fixtures/tests; it reuses the existing `refloat-configuration-codec` dependency and adds no third-party dependency.
- Updates root audits and core ownership documentation.
- Leaves the frontend contract, Swift/Kotlin public models and services, native packages, bridge catalogues, exported symbols, persistence, and production runtime behavior unchanged.
- Establishes the next migration boundary: Apple and Android can later select this state owner independently, then the unified Rust runtime can absorb scheduling/persistence and remove all temporary protocol bridge crates.
