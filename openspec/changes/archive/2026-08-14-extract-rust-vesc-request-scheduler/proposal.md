## Why

Apple and Android still maintain behaviorally equivalent connection-wide VESC request schedulers even though every migrated protocol and application component depends on their ordering, generation, and response-attribution guarantees. Extracting that deterministic state machine is the smallest useful application-behavior step toward one Rust runtime and gives later host cutovers a shared composition boundary without adding another feature-specific Apple FFI or Android JNI crate.

## What Changes

- Add a safe `backends/core/vesc-request-scheduler` Rust candidate that owns bounded priority/FIFO arbitration, one in-flight response request, one-way request completion, generation replacement, typed request contexts, response matching, deferred pumping, cancellation, explicit deadline handling, and stable completion outcomes.
- Model time, writes, deadline changes, diagnostics, and completions as bounded typed inputs and ordered effects so the candidate remains deterministic and contains no timer, Bluetooth, callback, thread, or host reference.
- Add source-derived parity vectors and adversarial Rust tests covering the complete Swift/Kotlin scheduler surface, queue bounds, all priorities and contexts, stale generations, route/context validation, timeouts, one-way work, cancellation, deferred resume, sequence/generation rollover, and effect ordering.
- Extend workspace, ownership, dependency, and verification audits, then run complete software and Android/iOS/macOS ESP32 regression while native schedulers remain selected.
- Document the candidate as the intended connection-wide arbitration owner for a later unified Rust runtime, where protocol services can compose directly without per-feature foreign bridges.
- Non-goals: no production host cutover; no new Apple FFI or Android JNI surface; no frontend, protocol-wire, SQLite, loopback, Bluetooth, lifecycle, location, file, theme, or background behavior change; no deletion of Swift/Kotlin scheduler code until separate host-cutover changes pass.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded Rust ownership and parity requirements for the extracted connection-wide VESC request scheduler state machine.
- `platform-hosts`: Locate the unselected candidate below native application services and preserve production host and OS ownership until explicit cutovers.
- `backend-conformance`: Require exhaustive candidate parity, isolation, complete software verification, and all-host physical regression evidence.

## Impact

- Adds one locked safe Rust workspace crate under `backends/core` with focused tests and no third-party dependency.
- Updates root audits and core ownership documentation.
- Leaves frontend contracts, Swift/Kotlin public models and schedulers, native packages, bridge catalogues, exported symbols, persistence, and production runtime behavior unchanged.
- Establishes a reusable stateful boundary beneath telemetry, discovery, BMS, controls, and configuration so later migration can compose those Rust owners into one runtime and ultimately delete the temporary `*-apple-ffi` and `*-android-jni` crates.
